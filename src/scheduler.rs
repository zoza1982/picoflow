//! Task scheduler for workflow execution (sequential and parallel)

use crate::dag::DagEngine;
use crate::error::{PicoFlowError, Result};
use crate::executors::http::HttpExecutor;
use crate::executors::shell::ShellExecutor;
use crate::executors::ssh::SshExecutor;
use crate::executors::ExecutorTrait;
use crate::models::{TaskConfig, TaskStatus, WorkflowConfig};
use crate::retry::calculate_backoff_delay;
use crate::state::StateManager;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
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
    /// - `buffer_unordered` caps in-flight tasks at max_parallel across each level
    /// - Wait for all tasks at a level to complete before proceeding
    /// - Stop on first failure unless continue_on_failure is set
    ///
    /// # Performance
    ///
    /// Target: 10 parallel tasks <50MB memory (PRD PERF-006)
    pub async fn execute_workflow(&self, config: &WorkflowConfig) -> Result<bool> {
        info!("Starting workflow execution: {}", config.name);

        // Validate that every task's executor config matches its declared type. The CLI
        // parse path already does this, but library callers can construct a WorkflowConfig
        // directly (or deserialize one) and bypass parser validation, so enforce it here
        // too — otherwise a mismatched type/config only surfaces at execution time.
        for task in &config.tasks {
            crate::parser::validate_task_executor_config(task)?;
        }

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

        // Build task lookup map. Values are `Arc<TaskConfig>` so the parallel executor can
        // hand each spawned task a cheap refcount bump instead of a deep clone of its
        // command/args/env on every level.
        let mut task_map: HashMap<String, Arc<TaskConfig>> =
            HashMap::with_capacity(config.tasks.len());
        for task in &config.tasks {
            task_map.insert(task.name.clone(), Arc::new(task.clone()));
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
        task_map: &HashMap<String, Arc<TaskConfig>>,
    ) -> Result<bool> {
        let mut workflow_success = true;

        for task_name in execution_order {
            let task = task_map.get(task_name).ok_or_else(|| {
                PicoFlowError::Other(format!("internal error: unknown task '{task_name}'"))
            })?;

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
    /// Each level runs with bounded concurrency (`buffer_unordered(max_parallel)`), so at
    /// most `max_parallel` task futures are in flight at once regardless of how wide the
    /// level is. All tasks at a level must complete before moving to the next level.
    /// Tasks are skipped if their dependencies failed (unless those deps had continue_on_failure).
    async fn execute_parallel(
        &self,
        execution_id: i64,
        parallel_levels: &[Vec<String>],
        task_map: &HashMap<String, Arc<TaskConfig>>,
        max_parallel: usize,
    ) -> Result<bool> {
        let mut workflow_success = true;
        let mut failed_tasks: std::collections::HashSet<String> = std::collections::HashSet::new();

        for (level_num, level_tasks) in parallel_levels.iter().enumerate() {
            info!(
                "Executing level {} with {} tasks: {:?}",
                level_num,
                level_tasks.len(),
                level_tasks
            );

            // Decide which tasks in this level are runnable (their dependencies did not
            // fail — or failed but were marked continue_on_failure).
            let mut runnable: Vec<Arc<TaskConfig>> = Vec::new();
            for task_name in level_tasks {
                let task = task_map.get(task_name).ok_or_else(|| {
                    PicoFlowError::Other(format!("internal error: unknown task '{task_name}'"))
                })?;

                let mut should_skip = false;
                for dep_name in &task.depends_on {
                    if failed_tasks.contains(dep_name) {
                        let dep_skips = task_map
                            .get(dep_name)
                            .map(|d| !d.continue_on_failure)
                            .unwrap_or(true);
                        if dep_skips {
                            warn!(
                                "Skipping task '{}' because dependency '{}' failed",
                                task_name, dep_name
                            );
                            should_skip = true;
                            failed_tasks.insert(task_name.clone());
                            break;
                        }
                    }
                }

                if !should_skip {
                    runnable.push(Arc::clone(task));
                }
            }

            // Execute the runnable tasks with bounded concurrency. `buffer_unordered`
            // keeps at most `max_parallel` task futures in flight at once, so memory and
            // scheduling cost scale with the concurrency limit rather than the (possibly
            // very wide) level size.
            let results: Vec<(String, bool, Result<bool>)> = futures::stream::iter(runnable)
                .map(|task| async move {
                    let name = task.name.clone();
                    let continue_on_failure = task.continue_on_failure;
                    let result = self.execute_task_with_retry(execution_id, &task).await;
                    (name, continue_on_failure, result)
                })
                .buffer_unordered(max_parallel)
                .collect()
                .await;

            // Check results and track failed tasks
            for (task_name, continue_on_failure, result) in results {
                match result {
                    Ok(task_success) => {
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
                    Err(e) => {
                        error!("Task '{}' execution error: {}", task_name, e);
                        failed_tasks.insert(task_name);
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
        // Total attempts = initial try + retries. `saturating_add` guards against overflow
        // for library callers that bypass parser validation (the parser caps retry at
        // MAX_RETRY_COUNT); without it, retry == u32::MAX would wrap the range to empty and
        // the task would silently never run.
        let total_attempts = max_retries.saturating_add(1);

        for attempt in 1..=total_attempts {
            info!(
                "Executing task '{}' (attempt {}/{})",
                task.name, attempt, total_attempts
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
                                total_attempts
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
                                task.name, total_attempts
                            );
                            return Ok(false);
                        }
                    }
                }
                Err(e) => {
                    // Classify timeouts by the typed error variant rather than by matching
                    // formatted text, which would break silently if wording changed (or
                    // misfire on an unrelated error whose message contains "timed out").
                    let is_timeout = e
                        .downcast_ref::<PicoFlowError>()
                        .map(|pe| matches!(pe, PicoFlowError::TaskTimeout { .. }))
                        .unwrap_or(false);
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

        // Apply timeout. On elapse, return a *typed* timeout error so the caller can
        // classify it as TaskStatus::Timeout without string matching.
        match timeout(Duration::from_secs(timeout_secs), task_future).await {
            Ok(result) => result,
            Err(_) => Err(anyhow::Error::new(PicoFlowError::TaskTimeout {
                task: task.name.clone(),
                timeout: timeout_secs,
            })),
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

    #[tokio::test]
    async fn test_empty_workflow_succeeds_as_noop() {
        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());
        let scheduler = TaskScheduler::new(state_manager.clone());

        let config = WorkflowConfig {
            name: "empty".to_string(),
            description: None,
            schedule: None,
            config: Default::default(),
            tasks: vec![],
        };

        let success = scheduler.execute_workflow(&config).await.unwrap();
        assert!(success, "an empty workflow should succeed as a no-op");

        let history = state_manager
            .get_execution_history("empty", 10)
            .await
            .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, TaskStatus::Success);
    }

    #[tokio::test]
    async fn test_retry_then_succeed_recovers() {
        // A task that fails on its first attempt and succeeds on the retry should end
        // up successful — exercising the real retry loop (state persistence + backoff),
        // which the other integration tests bypass by setting retry: 0.
        let temp = tempfile::TempDir::new().unwrap();
        let marker = temp.path().join("marker");
        let marker_str = marker.to_string_lossy().to_string();

        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());
        let scheduler = TaskScheduler::new(state_manager.clone());

        let config = WorkflowConfig {
            name: "retry-recover".to_string(),
            description: None,
            schedule: None,
            config: Default::default(),
            tasks: vec![TaskConfig {
                name: "flaky".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec![],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    // $0 is the marker path: first run creates it and fails, second run
                    // sees it exists and succeeds.
                    command: "/bin/sh".to_string(),
                    args: vec![
                        "-c".to_string(),
                        "if [ -e \"$0\" ]; then exit 0; else : > \"$0\"; exit 1; fi".to_string(),
                        marker_str,
                    ],
                    workdir: None,
                    env: None,
                }),
                retry: Some(1),
                timeout: Some(10),
                continue_on_failure: false,
            }],
        };

        let success = scheduler.execute_workflow(&config).await.unwrap();
        assert!(success, "task should succeed on the second attempt");

        // Two task-execution rows: the failed first attempt and the successful retry.
        let history = state_manager
            .get_execution_history("retry-recover", 1)
            .await
            .unwrap();
        let tasks = state_manager
            .get_task_executions(history[0].id)
            .await
            .unwrap();
        assert_eq!(
            tasks.len(),
            2,
            "expected one failed attempt + one successful"
        );
        assert!(tasks.iter().any(|t| t.status == TaskStatus::Success));
    }

    #[tokio::test]
    async fn test_max_parallel_bounds_concurrency() {
        // Four independent tasks (all at DAG level 0) each sleep 0.4s. With max_parallel=2
        // they must run in two waves, so wall-clock is ~0.8s. If the concurrency cap were
        // not enforced, all four would run at once (~0.4s). We assert only a lower bound to
        // stay robust on slow CI.
        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());
        let scheduler = TaskScheduler::new(state_manager.clone());

        let make_task = |n: usize| TaskConfig {
            name: format!("sleep{n}"),
            task_type: TaskType::Shell,
            depends_on: vec![],
            config: TaskExecutorConfig::Shell(ShellConfig {
                command: "/bin/sleep".to_string(),
                args: vec!["0.4".to_string()],
                workdir: None,
                env: None,
            }),
            retry: Some(0),
            timeout: Some(10),
            continue_on_failure: false,
        };

        let config = WorkflowConfig {
            name: "parallel-bound".to_string(),
            description: None,
            schedule: None,
            config: crate::models::WorkflowGlobalConfig {
                max_parallel: 2,
                retry_default: 0,
                timeout_default: 10,
            },
            tasks: (0..4).map(make_task).collect(),
        };

        let start = std::time::Instant::now();
        let success = scheduler.execute_workflow(&config).await.unwrap();
        let elapsed = start.elapsed();

        assert!(success);
        assert!(
            elapsed >= std::time::Duration::from_millis(650),
            "4 tasks @0.4s with max_parallel=2 should take ~0.8s (two waves), took {elapsed:?} — \
             concurrency cap not enforced?"
        );
    }
}
