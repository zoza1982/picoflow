//! Task executors

pub mod shell;
pub mod ssh;

use crate::models::{ExecutionResult, TaskExecutorConfig};
use async_trait::async_trait;

/// Executor trait for different task types
#[async_trait]
pub trait ExecutorTrait: Send + Sync {
    /// Execute a task with the given configuration
    async fn execute(&self, config: &TaskExecutorConfig) -> anyhow::Result<ExecutionResult>;

    /// Perform a health check
    async fn health_check(&self) -> anyhow::Result<()>;
}
