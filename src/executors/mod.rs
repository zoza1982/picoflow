//! Task executors

pub mod http;
pub mod shell;
pub mod ssh;

use crate::models::{ExecutionResult, TaskExecutorConfig, MAX_OUTPUT_SIZE};
use async_trait::async_trait;

/// Executor trait for different task types
#[async_trait]
pub trait ExecutorTrait: Send + Sync {
    /// Execute a task with the given configuration
    async fn execute(&self, config: &TaskExecutorConfig) -> anyhow::Result<ExecutionResult>;

    /// Perform a health check
    async fn health_check(&self) -> anyhow::Result<()>;
}

/// Truncate string output to MAX_OUTPUT_SIZE
///
/// Returns (truncated_string, was_truncated)
pub(crate) fn truncate_output_str(data: &str) -> (String, bool) {
    let bytes = data.as_bytes();
    let truncated = bytes.len() > MAX_OUTPUT_SIZE;

    if truncated {
        let truncated_bytes = &bytes[..MAX_OUTPUT_SIZE];
        let output = String::from_utf8_lossy(truncated_bytes).to_string();
        (output, true)
    } else {
        (data.to_string(), false)
    }
}

/// Truncate byte output to MAX_OUTPUT_SIZE
///
/// Returns (truncated_string, was_truncated)
pub(crate) fn truncate_output_bytes(data: &[u8]) -> (String, bool) {
    let truncated = data.len() > MAX_OUTPUT_SIZE;
    let bytes = if truncated {
        &data[..MAX_OUTPUT_SIZE]
    } else {
        data
    };

    let output = String::from_utf8_lossy(bytes).to_string();
    (output, truncated)
}
