//! Task scheduler for workflow execution (sequential and parallel)

use crate::dag::DagEngine;
use crate::error::Result;
use crate::executors::http::HttpExecutor;
use crate::executors::shell::ShellExecutor;
use crate::executors::ssh::SshExecutor;
use crate::executors::ExecutorTrait;
use crate::models::{TaskConfig, TaskStatus, WorkflowConfig};
use crate::retry::calculate_backoff_delay;
use crate::state::StateManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

/// Task scheduler supporting both sequential and parallel execution
///
/// Phase 1: Sequential execution (topological sort)
/// Phase 3: Parallel execution with configurable concurrency limits
pub struct TaskScheduler {
    state_manager: Arc<StateManager>,
    shell_executor: Arc<ShellExecutor>,
    ssh_executor: Arc<SshExecutor>,
    http_executor: Arc<HttpExecutor>,
}

impl TaskScheduler {
    /// Create a new task scheduler
    pub fn new(state_manager: Arc<StateManager>) -> Self {
        Self {
            state_manager,
            shell_executor: Arc::new(ShellExecutor::new()),
            ssh_executor: Arc::new(SshExecutor::new()),
            http_executor: Arc::new(HttpExecutor::new()),
        }
    }

    /// Execute a workflow once (supports both sequential and parallel execution)
    ///
    /// # Execution Strategy
    ///
    /// - Phase 1: Sequential execution (max_parallel = 1)
    /// - Phase 3: Parallel execution by DAG levels (max_parallel > 1)
    ///
    /// When max_parallel > 1, tasks are executed in parallel levels:
    /// - All tasks at the same level run concurrently
    /// - Semaphore enforces max_parallel limit across all levels
    /// - Wait for all tasks at a level to complete before proceeding
    /// - Stop on first failure unless continue_on_failure is set
    ///
    /// # Performance
    ///
    /// Target: 10 parallel tasks <50MB memory (PRD PERF-006)
    pub async fn execute_workflow(&self, config: &WorkflowConfig) -> Result<bool> {
        info!("Starting workflow execution: {}", config.name);

        // Build DAG and validate
        let dag = DagEngine::build(&config.tasks)?;
        info!("DAG validation successful");

        // Create workflow execution record
        let workflow_id = self
            .state_manager
            .get_or_create_workflow(&config.name, config.schedule.as_deref())
            .await?;
        let execution_id = self.state_manager.start_execution(workflow_id).await?;

        info!("Created workflow execution record (id: {})", execution_id);

        // Build task lookup map with pre-allocated capacity
        let mut task_map: HashMap<String, TaskConfig> = HashMap::with_capacity(config.tasks.len());
        for task in &config.tasks {
            task_map.insert(task.name.clone(), task.clone());
        }

        // Execute workflow based on max_parallel setting
        let workflow_success = if config.config.max_parallel == 1 {
            // Sequential execution (Phase 1 behavior)
            info!("Executing workflow sequentially (max_parallel=1)");
            let execution_order = dag.topological_sort()?;
            info!("Execution order: {:?}", execution_order);
            self.execute_sequential(execution_id, &execution_order, &task_map)
                .await?
        } else {
            // Parallel execution by DAG levels (Phase 3)
            let parallel_levels = dag.parallel_levels();
            info!(
                "Executing workflow in parallel (max_parallel={}, levels={})",
                config.config.max_parallel,
                parallel_levels.len()
            );
            self.execute_parallel(
                execution_id,
                &parallel_levels,
                &task_map,
                config.config.max_parallel,
            )
            .await?
        };

        // Update workflow execution status
        let final_status = if workflow_success {
            TaskStatus::Success
        } else {
            TaskStatus::Failed
        };

        self.state_manager
            .update_execution_status(execution_id, final_status.clone())
            .await?;

        info!("Workflow execution completed with status: {}", final_status);

        Ok(workflow_success)
    }

    /// Execute tasks sequentially in topological order
    async fn execute_sequential(
        &self,
        execution_id: i64,
        execution_order: &[String],
        task_map: &HashMap<String, TaskConfig>,
    ) -> Result<bool> {
        let mut workflow_success = true;

        for task_name in execution_order {
            let task = &task_map[task_name];

            info!("Executing task: {}", task_name);

            // Execute task with retry logic
            let task_success = self.execute_task_with_retry(execution_id, task).await?;

            if !task_success {
                workflow_success = false;

                if !task.continue_on_failure {
                    error!(
                        "Task '{}' failed and continue_on_failure=false, stopping workflow",
                        task_name
                    );
                    break;
                } else {
                    warn!(
                        "Task '{}' failed but continue_on_failure=true, continuing",
                        task_name
                    );
                }
            }
        }

        Ok(workflow_success)
    }

    /// Execute tasks in parallel by DAG levels
    ///
    /// Each level is executed concurrently, with a semaphore enforcing max_parallel limit.
    /// All tasks at a level must complete before moving to the next level.
    /// Tasks are skipped if their dependencies failed (unless those deps had continue_on_failure).
    async fn execute_parallel(
        &self,
        execution_id: i64,
        parallel_levels: &[Vec<String>],
        task_map: &HashMap<String, TaskConfig>,
        max_parallel: usize,
    ) -> Result<bool> {
        let mut workflow_success = true;
        let mut failed_tasks: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Create semaphore to enforce max_parallel limit
        let semaphore = Arc::new(Semaphore::new(max_parallel));

        for (level_num, level_tasks) in parallel_levels.iter().enumerate() {
            info!(
                "Executing level {} with {} tasks: {:?}",
                level_num,
                level_tasks.len(),
                level_tasks
            );

            // Spawn tasks for this level
            let mut handles = Vec::new();

            for task_name in level_tasks {
                let task = task_map[task_name].clone();
                let task_name_owned = task_name.clone();

                // Check if any dependencies failed (and didn't have continue_on_failure)
                let mut should_skip = false;
                for dep_name in &task.depends_on {
                    if failed_tasks.contains(dep_name) {
                        let dep_task = &task_map[dep_name];
                        if !dep_task.continue_on_failure {
                            warn!(
                                "Skipping task '{}' because dependency '{}' failed",
                                task_name_owned, dep_name
                            );
                            should_skip = true;
                            failed_tasks.insert(task_name_owned.clone());
                            break;
                        }
                    }
                }

                if should_skip {
                    continue;
                }

                let semaphore_clone = Arc::clone(&semaphore);

                // Share Arc references for the spawned task (no deep cloning)
                let state_manager = Arc::clone(&self.state_manager);
                let shell_executor = Arc::clone(&self.shell_executor);
                let ssh_executor = Arc::clone(&self.ssh_executor);
                let http_executor = Arc::clone(&self.http_executor);

                // Spawn task execution
                let handle = tokio::spawn(async move {
                    // Acquire semaphore permit
                    let _permit = match semaphore_clone.acquire().await {
                        Ok(permit) => permit,
                        Err(_) => {
                            warn!(
                                "Semaphore closed during task acquisition for '{}', shutdown in progress",
                                task_name_owned
                            );
                            return (
                                task_name_owned,
                                task.continue_on_failure,
                                Err(crate::error::PicoFlowError::Other(
                                    "Shutdown in progress".to_string(),
                                )),
                            );
                        }
                    };

                    info!("Task '{}' acquired execution permit", task_name_owned);

                    // Create a temporary scheduler for this task
                    let temp_scheduler = TaskScheduler {
                        state_manager,
                        shell_executor,
                        ssh_executor,
                        http_executor,
                    };

                    // Execute task with retry logic
                    let result = temp_scheduler
                        .execute_task_with_retry(execution_id, &task)
                        .await;

                    // Permit is automatically released when _permit is dropped
                    (task_name_owned, task.continue_on_failure, result)
                });

                handles.push(handle);
            }

            // Wait for all tasks at this level to complete
            let results = futures::future::join_all(handles).await;

            // Check results and track failed tasks
            for result in results {
                match result {
                    Ok((task_name, continue_on_failure, Ok(task_success))) => {
                        if !task_success {
                            workflow_success = false;
                            failed_tasks.insert(task_name.clone());

                            if !continue_on_failure {
                                error!(
                                    "Task '{}' failed and continue_on_failure=false, stopping workflow",
                                    task_name
                                );
                                return Ok(false);
                            } else {
                                warn!(
                                    "Task '{}' failed but continue_on_failure=true, continuing",
                                    task_name
                                );
                            }
                        }
                    }
                    Ok((task_name, _, Err(e))) => {
                        error!("Task '{}' execution error: {}", task_name, e);
                        failed_tasks.insert(task_name);
                        return Ok(false);
                    }
                    Err(e) => {
                        error!("Task spawn error: {}", e);
                        return Ok(false);
                    }
                }
            }
        }

        Ok(workflow_success)
    }

    /// Execute a single task with retry logic
    async fn execute_task_with_retry(&self, execution_id: i64, task: &TaskConfig) -> Result<bool> {
        let max_retries = task.retry.unwrap_or(3);
        let timeout = task.timeout.unwrap_or(300);

        for attempt in 1..=max_retries + 1 {
            info!(
                "Executing task '{}' (attempt {}/{})",
                task.name,
                attempt,
                max_retries + 1
            );

            // Start task execution record
            let task_exec_id = self
                .state_manager
                .start_task(execution_id, &task.name, attempt as i32)
                .await?;

            // Execute task
            let result = self.execute_task(task, timeout).await;

            match result {
                Ok(exec_result) => {
                    // Update task status in database
                    self.state_manager
                        .update_task_status(
                            task_exec_id,
                            exec_result.status.clone(),
                            exec_result.exit_code,
                            exec_result.stdout.as_deref(),
                            exec_result.stderr.as_deref(),
                        )
                        .await?;

                    if exec_result.status == TaskStatus::Success {
                        info!("Task '{}' completed successfully", task.name);
                        return Ok(true);
                    } else {
                        error!(
                            "Task '{}' failed with exit code {:?}",
                            task.name, exec_result.exit_code
                        );

                        if attempt <= max_retries {
                            let delay = calculate_backoff_delay(attempt);
                            warn!(
                                "Task '{}' will retry in {} seconds (attempt {}/{})",
                                task.name,
                                delay.as_secs(),
                                attempt + 1,
                                max_retries + 1
                            );

                            // Set retry information
                            let next_retry_at = chrono::Utc::now()
                                + chrono::Duration::from_std(delay)
                                    .unwrap_or_else(|_| chrono::Duration::seconds(60));
                            self.state_manager
                                .set_task_retry(task_exec_id, (attempt - 1) as i32, next_retry_at)
                                .await?;

                            tokio::time::sleep(delay).await;
                        } else {
                            error!(
                                "Task '{}' failed after {} attempts",
                                task.name,
                                max_retries + 1
                            );
                            return Ok(false);
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    let is_timeout = error_msg.contains("timed out");
                    let status = if is_timeout {
                        TaskStatus::Timeout
                    } else {
                        TaskStatus::Failed
                    };

                    error!("Task '{}' execution error ({}): {}", task.name, status, e);

                    // Update task status
                    self.state_manager
                        .update_task_status(
                            task_exec_id,
                            status,
                            None,
                            None,
                            Some(&format!("Execution error: {}", e)),
                        )
                        .await?;

                    if attempt <= max_retries {
                        let delay = calculate_backoff_delay(attempt);
                        warn!(
                            "Task '{}' will retry in {} seconds after error",
                            task.name,
                            delay.as_secs()
                        );
                        tokio::time::sleep(delay).await;
                    } else {
                        return Ok(false);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Execute a single task with timeout enforcement
    async fn execute_task(
        &self,
        task: &TaskConfig,
        timeout_secs: u64,
    ) -> anyhow::Result<crate::models::ExecutionResult> {
        use tokio::time::{timeout, Duration};

        // Wrap task execution with timeout
        let task_future = async {
            match task.task_type {
                crate::models::TaskType::Shell => self.shell_executor.execute(&task.config).await,
                crate::models::TaskType::Ssh => self.ssh_executor.execute(&task.config).await,
                crate::models::TaskType::Http => self.http_executor.execute(&task.config).await,
            }
        };

        // Apply timeout
        match timeout(Duration::from_secs(timeout_secs), task_future).await {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!(
                "Task '{}' timed out after {} seconds",
                task.name,
                timeout_secs
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ShellConfig, TaskExecutorConfig, TaskType};

    #[test]
    fn test_calculate_backoff_delay() {
        assert_eq!(
            calculate_backoff_delay(1),
            std::time::Duration::from_secs(1)
        );
        assert_eq!(
            calculate_backoff_delay(2),
            std::time::Duration::from_secs(2)
        );
        assert_eq!(
            calculate_backoff_delay(3),
            std::time::Duration::from_secs(4)
        );
        assert_eq!(
            calculate_backoff_delay(4),
            std::time::Duration::from_secs(8)
        );
        assert_eq!(
            calculate_backoff_delay(5),
            std::time::Duration::from_secs(16)
        );
        assert_eq!(
            calculate_backoff_delay(6),
            std::time::Duration::from_secs(32)
        );
        assert_eq!(
            calculate_backoff_delay(7),
            std::time::Duration::from_secs(60)
        ); // Capped
    }

    #[tokio::test]
    async fn test_execute_simple_workflow() {
        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());
        let scheduler = TaskScheduler::new(state_manager.clone());

        let config = WorkflowConfig {
            name: "test-workflow".to_string(),
            description: Some("Test".to_string()),
            schedule: None,
            config: Default::default(),
            tasks: vec![TaskConfig {
                name: "task1".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec![],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["hello".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(1),
                timeout: Some(10),
                continue_on_failure: false,
            }],
        };

        let success = scheduler.execute_workflow(&config).await.unwrap();
        assert!(success);

        // Verify execution was recorded
        let history = state_manager
            .get_execution_history("test-workflow", 10)
            .await
            .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, TaskStatus::Success);
    }

    #[tokio::test]
    async fn test_execute_failing_workflow() {
        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());
        let scheduler = TaskScheduler::new(state_manager.clone());

        let config = WorkflowConfig {
            name: "fail-workflow".to_string(),
            description: None,
            schedule: None,
            config: Default::default(),
            tasks: vec![TaskConfig {
                name: "failing_task".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec![],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/sh".to_string(),
                    args: vec!["-c".to_string(), "exit 1".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(1),
                timeout: Some(10),
                continue_on_failure: false,
            }],
        };

        let success = scheduler.execute_workflow(&config).await.unwrap();
        assert!(!success);

        // Verify execution was recorded as failed
        let history = state_manager
            .get_execution_history("fail-workflow", 10)
            .await
            .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, TaskStatus::Failed);
    }

    #[tokio::test]
    async fn test_continue_on_failure() {
        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());
        let scheduler = TaskScheduler::new(state_manager.clone());

        let config = WorkflowConfig {
            name: "continue-workflow".to_string(),
            description: None,
            schedule: None,
            config: Default::default(),
            tasks: vec![
                TaskConfig {
                    name: "task1".to_string(),
                    task_type: TaskType::Shell,
                    depends_on: vec![],
                    config: TaskExecutorConfig::Shell(ShellConfig {
                        command: "/bin/sh".to_string(),
                        args: vec!["-c".to_string(), "exit 1".to_string()],
                        workdir: None,
                        env: None,
                    }),
                    retry: Some(0),
                    timeout: Some(10),
                    continue_on_failure: true, // Continue despite failure
                },
                TaskConfig {
                    name: "task2".to_string(),
                    task_type: TaskType::Shell,
                    depends_on: vec!["task1".to_string()],
                    config: TaskExecutorConfig::Shell(ShellConfig {
                        command: "/bin/echo".to_string(),
                        args: vec!["task2".to_string()],
                        workdir: None,
                        env: None,
                    }),
                    retry: Some(0),
                    timeout: Some(10),
                    continue_on_failure: false,
                },
            ],
        };

        let success = scheduler.execute_workflow(&config).await.unwrap();
        // Overall workflow fails because task1 failed, but task2 should have executed
        assert!(!success);

        // Verify both tasks were executed
        let _workflow_id = state_manager
            .get_or_create_workflow("continue-workflow", None)
            .await
            .unwrap();
        let history = state_manager
            .get_execution_history("continue-workflow", 1)
            .await
            .unwrap();
        let tasks = state_manager
            .get_task_executions(history[0].id)
            .await
            .unwrap();

        assert_eq!(tasks.len(), 2); // Both tasks executed
    }
}
