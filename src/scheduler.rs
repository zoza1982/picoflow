//! Sequential task scheduler for workflow execution

use crate::dag::DagEngine;
use crate::error::Result;
use crate::executors::shell::ShellExecutor;
use crate::executors::ExecutorTrait;
use crate::models::{TaskConfig, TaskStatus, WorkflowConfig};
use crate::state::StateManager;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Sequential task scheduler (Phase 1 MVP)
pub struct TaskScheduler {
    state_manager: Arc<StateManager>,
    shell_executor: ShellExecutor,
}

impl TaskScheduler {
    /// Create a new task scheduler
    pub fn new(state_manager: Arc<StateManager>) -> Self {
        Self {
            state_manager,
            shell_executor: ShellExecutor::new(),
        }
    }

    /// Execute a workflow once (one-shot mode)
    pub async fn execute_workflow(&self, config: &WorkflowConfig) -> Result<bool> {
        info!("Starting workflow execution: {}", config.name);

        // Build DAG and validate
        let dag = DagEngine::build(&config.tasks)?;
        info!("DAG validation successful");

        // Get topological sort order
        let execution_order = dag.topological_sort()?;
        info!("Execution order: {:?}", execution_order);

        // Create workflow execution record
        let workflow_id = self.state_manager.get_or_create_workflow(&config.name)?;
        let execution_id = self.state_manager.start_execution(workflow_id)?;

        info!("Created workflow execution record (id: {})", execution_id);

        // Build task lookup map
        let task_map: HashMap<_, _> = config
            .tasks
            .iter()
            .map(|t| (t.name.clone(), t.clone()))
            .collect();

        // Execute tasks in topological order
        let mut workflow_success = true;

        for task_name in execution_order {
            let task = &task_map[&task_name];

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

        // Update workflow execution status
        let final_status = if workflow_success {
            TaskStatus::Success
        } else {
            TaskStatus::Failed
        };

        self.state_manager
            .update_execution_status(execution_id, final_status.clone())?;

        info!("Workflow execution completed with status: {}", final_status);

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
            let task_exec_id =
                self.state_manager
                    .start_task(execution_id, &task.name, attempt as i32)?;

            // Execute task
            let result = self.execute_task(task, timeout).await;

            match result {
                Ok(exec_result) => {
                    // Update task status in database
                    self.state_manager.update_task_status(
                        task_exec_id,
                        exec_result.status.clone(),
                        exec_result.exit_code,
                        exec_result.stdout.as_deref(),
                        exec_result.stderr.as_deref(),
                    )?;

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
                            let next_retry_at =
                                chrono::Utc::now() + chrono::Duration::from_std(delay).unwrap();
                            self.state_manager.set_task_retry(
                                task_exec_id,
                                attempt as i32,
                                next_retry_at,
                            )?;

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
                    error!("Task '{}' execution error: {}", task.name, e);

                    // Update task status to failed
                    self.state_manager.update_task_status(
                        task_exec_id,
                        TaskStatus::Failed,
                        None,
                        None,
                        Some(&format!("Execution error: {}", e)),
                    )?;

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
                crate::models::TaskType::Ssh => {
                    Err(anyhow::anyhow!("SSH executor not yet implemented"))
                }
                crate::models::TaskType::Http => {
                    Err(anyhow::anyhow!("HTTP executor not yet implemented"))
                }
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

/// Calculate exponential backoff delay
fn calculate_backoff_delay(attempt: u32) -> std::time::Duration {
    let base_delay_secs = 1;
    let delay_secs = base_delay_secs * 2u64.pow(attempt - 1);
    std::time::Duration::from_secs(delay_secs.min(60)) // Cap at 60 seconds
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
        let state_manager = Arc::new(StateManager::in_memory().unwrap());
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
            .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, TaskStatus::Success);
    }

    #[tokio::test]
    async fn test_execute_failing_workflow() {
        let state_manager = Arc::new(StateManager::in_memory().unwrap());
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
            .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, TaskStatus::Failed);
    }

    #[tokio::test]
    async fn test_continue_on_failure() {
        let state_manager = Arc::new(StateManager::in_memory().unwrap());
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
            .get_or_create_workflow("continue-workflow")
            .unwrap();
        let history = state_manager
            .get_execution_history("continue-workflow", 1)
            .unwrap();
        let tasks = state_manager.get_task_executions(history[0].id).unwrap();

        assert_eq!(tasks.len(), 2); // Both tasks executed
    }
}
