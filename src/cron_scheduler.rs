//! Cron-based workflow scheduler
//!
//! This module provides cron-based scheduling for workflow execution using
//! tokio-cron-scheduler. It supports multiple workflows with different schedules
//! running concurrently.
//!
//! # Example
//!
//! ```no_run
//! use picoflow::cron_scheduler::CronScheduler;
//! use picoflow::models::WorkflowConfig;
//! use picoflow::state::StateManager;
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let state_manager = Arc::new(StateManager::new("picoflow.db").await?);
//! let mut scheduler = CronScheduler::new(state_manager).await?;
//!
//! // Parse workflow with cron schedule (6-field format: sec min hour day month dayofweek)
//! let workflow: WorkflowConfig = serde_yaml::from_str(r#"
//! name: daily-backup
//! schedule: "0 0 2 * * *"  # Run at 2 AM daily
//! tasks: []
//! "#)?;
//!
//! scheduler.add_workflow(workflow).await?;
//! scheduler.start().await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{PicoFlowError, Result};
use crate::models::WorkflowConfig;
use crate::scheduler::TaskScheduler;
use crate::state::StateManager;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, info};

/// Cron-based workflow scheduler
///
/// Manages multiple workflows with cron schedules and executes them automatically
/// based on their configured schedule expressions.
pub struct CronScheduler {
    /// Underlying cron scheduler from tokio-cron-scheduler
    scheduler: JobScheduler,
    /// Task scheduler for executing workflows
    task_scheduler: Arc<TaskScheduler>,
}

impl CronScheduler {
    /// Create a new cron scheduler
    ///
    /// # Arguments
    ///
    /// * `state_manager` - State manager for workflow execution tracking
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - New cron scheduler instance
    ///
    /// # Example
    ///
    /// ```no_run
    /// use picoflow::cron_scheduler::CronScheduler;
    /// use picoflow::state::StateManager;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let state_manager = Arc::new(StateManager::new("picoflow.db").await?);
    /// let scheduler = CronScheduler::new(state_manager).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(state_manager: Arc<StateManager>) -> Result<Self> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| PicoFlowError::Other(format!("Failed to create job scheduler: {}", e)))?;

        let task_scheduler = Arc::new(TaskScheduler::new(state_manager.clone()));

        Ok(Self {
            scheduler,
            task_scheduler,
        })
    }

    /// Add a workflow with cron schedule to the scheduler
    ///
    /// # Arguments
    ///
    /// * `workflow` - Workflow configuration with schedule
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error
    ///
    /// # Errors
    ///
    /// * `PicoFlowError::Validation` - If workflow has no schedule or invalid cron expression
    pub async fn add_workflow(&mut self, workflow: WorkflowConfig) -> Result<()> {
        // Validate workflow has a schedule
        let schedule = workflow.schedule.as_ref().ok_or_else(|| {
            PicoFlowError::Validation(format!(
                "Workflow '{}' has no schedule defined",
                workflow.name
            ))
        })?;

        info!(
            "Adding workflow '{}' with schedule: {}",
            workflow.name, schedule
        );

        // Create a job for this workflow
        let workflow_clone = workflow.clone();
        let task_scheduler = self.task_scheduler.clone();
        let workflow_name = workflow.name.clone();

        // Create the cron job
        let job = Job::new_async(schedule.as_str(), move |_uuid, _lock| {
            let workflow = workflow_clone.clone();
            let scheduler = task_scheduler.clone();
            let name = workflow_name.clone();

            Box::pin(async move {
                info!("Cron trigger: executing workflow '{}'", name);

                match scheduler.execute_workflow(&workflow).await {
                    Ok(success) => {
                        if success {
                            info!("Cron workflow '{}' completed successfully", name);
                        } else {
                            error!("Cron workflow '{}' failed", name);
                        }
                    }
                    Err(e) => {
                        error!("Cron workflow '{}' execution error: {}", name, e);
                    }
                }
            })
        })
        .map_err(|e| {
            PicoFlowError::Validation(format!("Invalid cron expression '{}': {}", schedule, e))
        })?;

        // Add job to scheduler
        self.scheduler
            .add(job)
            .await
            .map_err(|e| PicoFlowError::Other(format!("Failed to add job: {}", e)))?;

        info!("Workflow '{}' added to scheduler", workflow.name);

        Ok(())
    }

    /// Start the cron scheduler
    ///
    /// This starts the background scheduler thread that will execute workflows
    /// based on their cron schedules.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error
    pub async fn start(&self) -> Result<()> {
        info!("Starting cron scheduler");

        self.scheduler
            .start()
            .await
            .map_err(|e| PicoFlowError::Other(format!("Failed to start scheduler: {}", e)))?;

        info!("Cron scheduler started successfully");

        Ok(())
    }

    /// Shutdown the cron scheduler
    ///
    /// This stops the scheduler and waits for any running jobs to complete.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down cron scheduler");

        self.scheduler
            .shutdown()
            .await
            .map_err(|e| PicoFlowError::Other(format!("Failed to shutdown scheduler: {}", e)))?;

        info!("Cron scheduler shutdown complete");

        Ok(())
    }

    /// Get the number of scheduled jobs
    pub fn job_count(&self) -> usize {
        // tokio-cron-scheduler doesn't expose job count directly
        // This is a placeholder - in production we'd track this ourselves
        0
    }
}

/// Validate cron expression format
///
/// This is a helper function to validate cron expressions before adding them to the scheduler.
///
/// # Arguments
///
/// * `expression` - Cron expression string in 6-field format (sec min hour day month dayofweek)
///
/// # Returns
///
/// * `Result<()>` - Success if valid, error otherwise
///
/// # Example
///
/// ```
/// use picoflow::cron_scheduler::validate_cron_expression;
///
/// // 6-field format: sec min hour day month dayofweek
/// assert!(validate_cron_expression("0 0 2 * * *").is_ok());
/// assert!(validate_cron_expression("invalid").is_err());
/// ```
pub fn validate_cron_expression(expression: &str) -> Result<()> {
    // Try to create a dummy job with this expression to validate it
    Job::new(expression, |_uuid, _lock| {
        // Dummy closure
    })
    .map_err(|e| {
        PicoFlowError::Validation(format!("Invalid cron expression '{}': {}", expression, e))
    })?;

    debug!("Cron expression '{}' is valid", expression);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ShellConfig, TaskConfig, TaskExecutorConfig, TaskType, WorkflowGlobalConfig,
    };

    #[test]
    fn test_validate_cron_expression_valid() {
        // tokio-cron-scheduler supports both 5-field and 6-field cron expressions
        // 5-field: sec min hour day_of_month month day_of_week
        // 6-field: min hour day_of_month month day_of_week year
        assert!(validate_cron_expression("0 2 * * * *").is_ok()); // 6-field: Daily at 2 AM
        assert!(validate_cron_expression("0 */5 * * * *").is_ok()); // 6-field: Every 5 minutes
    }

    #[test]
    fn test_validate_cron_expression_invalid() {
        assert!(validate_cron_expression("invalid").is_err());
        assert!(validate_cron_expression("60 * * * * *").is_err()); // Invalid minute
    }

    #[tokio::test]
    async fn test_cron_scheduler_new() {
        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());
        let scheduler = CronScheduler::new(state_manager).await;
        assert!(scheduler.is_ok());
    }

    #[tokio::test]
    async fn test_add_workflow_with_schedule() {
        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());
        let mut scheduler = CronScheduler::new(state_manager).await.unwrap();

        let workflow = WorkflowConfig {
            name: "test-workflow".to_string(),
            description: Some("Test workflow".to_string()),
            schedule: Some("0 2 * * * *".to_string()), // 6-field format: Daily at 2 AM
            config: WorkflowGlobalConfig::default(),
            tasks: vec![TaskConfig {
                name: "test_task".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec![],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["test".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(1),
                timeout: Some(10),
                continue_on_failure: false,
            }],
        };

        let result = scheduler.add_workflow(workflow).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_workflow_without_schedule() {
        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());
        let mut scheduler = CronScheduler::new(state_manager).await.unwrap();

        let workflow = WorkflowConfig {
            name: "test-workflow".to_string(),
            description: None,
            schedule: None, // No schedule
            config: WorkflowGlobalConfig::default(),
            tasks: vec![],
        };

        let result = scheduler.add_workflow(workflow).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(PicoFlowError::Validation(_))));
    }

    #[tokio::test]
    async fn test_add_workflow_invalid_cron() {
        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());
        let mut scheduler = CronScheduler::new(state_manager).await.unwrap();

        let workflow = WorkflowConfig {
            name: "test-workflow".to_string(),
            description: None,
            schedule: Some("invalid cron".to_string()),
            config: WorkflowGlobalConfig::default(),
            tasks: vec![],
        };

        let result = scheduler.add_workflow(workflow).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scheduler_start_stop() {
        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());
        let mut scheduler = CronScheduler::new(state_manager).await.unwrap();

        // Start scheduler
        let result = scheduler.start().await;
        assert!(result.is_ok());

        // Shutdown scheduler
        let result = scheduler.shutdown().await;
        assert!(result.is_ok());
    }
}
