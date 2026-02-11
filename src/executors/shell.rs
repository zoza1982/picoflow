//! Shell command executor

use crate::error::{PicoFlowError, Result};
use crate::executors::ExecutorTrait;
use crate::models::{
    ExecutionResult, ShellConfig, TaskExecutorConfig, TaskStatus, MAX_OUTPUT_SIZE,
};
use crate::parser::validate_shell_config;
use async_trait::async_trait;
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, error, info};

/// Shell executor for local command execution
#[derive(Debug, Clone)]
pub struct ShellExecutor;

impl ShellExecutor {
    pub fn new() -> Self {
        Self
    }

    async fn execute_shell(
        &self,
        config: &ShellConfig,
        timeout_secs: u64,
    ) -> Result<ExecutionResult> {
        // Validate configuration
        validate_shell_config(config)?;

        info!("Executing shell command: {}", config.command);
        debug!("Command args: {:?}", config.args);

        let start = std::time::Instant::now();

        // Create command with individual args (no shell interpolation)
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);

        // Set working directory if specified
        if let Some(workdir) = &config.workdir {
            cmd.current_dir(workdir);
        }

        // Set environment variables if specified
        if let Some(env) = &config.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        // Capture output
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        // Ensure child process is killed when the future is dropped (e.g. on timeout).
        // Without this, timed-out processes become orphan zombies.
        cmd.kill_on_drop(true);

        // Execute with timeout
        let output_result =
            tokio::time::timeout(Duration::from_secs(timeout_secs), cmd.output()).await;

        let duration = start.elapsed();

        match output_result {
            Ok(Ok(output)) => {
                // Truncate output if needed
                let (stdout, stdout_truncated) =
                    crate::executors::truncate_output_bytes(&output.stdout);
                let (stderr, stderr_truncated) =
                    crate::executors::truncate_output_bytes(&output.stderr);
                let output_truncated = stdout_truncated || stderr_truncated;

                let status = if output.status.success() {
                    TaskStatus::Success
                } else {
                    TaskStatus::Failed
                };

                if output_truncated {
                    debug!("Output truncated to {} bytes", MAX_OUTPUT_SIZE);
                }

                info!(
                    "Command completed with status: {} (exit code: {:?})",
                    status,
                    output.status.code()
                );

                Ok(ExecutionResult {
                    status,
                    stdout: Some(stdout),
                    stderr: Some(stderr),
                    exit_code: output.status.code(),
                    duration,
                    output_truncated,
                })
            }
            Ok(Err(e)) => {
                error!("Command execution failed: {}", e);
                Err(PicoFlowError::Io(e))
            }
            Err(_) => {
                error!("Command timed out after {} seconds", timeout_secs);
                Err(PicoFlowError::TaskTimeout {
                    task: config.command.clone(),
                    timeout: timeout_secs,
                })
            }
        }
    }
}

#[async_trait]
impl ExecutorTrait for ShellExecutor {
    async fn execute(&self, config: &TaskExecutorConfig) -> anyhow::Result<ExecutionResult> {
        match config {
            TaskExecutorConfig::Shell(shell_config) => {
                // Use a very large timeout here since scheduler applies the actual timeout
                // This prevents double-timeout issues and ensures scheduler timeout takes precedence
                let result = self.execute_shell(shell_config, 86400).await?;
                Ok(result)
            }
            _ => Err(anyhow::anyhow!("Invalid config type for ShellExecutor")),
        }
    }

    async fn health_check(&self) -> anyhow::Result<()> {
        // For shell executor, just verify we can spawn a process
        let output = Command::new("/bin/sh")
            .arg("-c")
            .arg("exit 0")
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Shell executor health check failed"))
        }
    }
}

impl Default for ShellExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_shell_executor_success() {
        let executor = ShellExecutor::new();
        let config = TaskExecutorConfig::Shell(ShellConfig {
            command: "/bin/echo".to_string(),
            args: vec!["hello".to_string()],
            workdir: None,
            env: None,
        });

        let result = executor.execute(&config).await.unwrap();
        assert_eq!(result.status, TaskStatus::Success);
        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn test_shell_executor_failure() {
        let executor = ShellExecutor::new();
        let config = TaskExecutorConfig::Shell(ShellConfig {
            command: "/bin/sh".to_string(),
            args: vec!["-c".to_string(), "exit 1".to_string()],
            workdir: None,
            env: None,
        });

        let result = executor.execute(&config).await.unwrap();
        assert_eq!(result.status, TaskStatus::Failed);
        assert_eq!(result.exit_code, Some(1));
    }

    #[tokio::test]
    async fn test_shell_executor_with_env() {
        let executor = ShellExecutor::new();
        let mut env = HashMap::new();
        env.insert("TEST_VAR".to_string(), "test_value".to_string());

        let config = TaskExecutorConfig::Shell(ShellConfig {
            command: "/bin/sh".to_string(),
            args: vec!["-c".to_string(), "echo $TEST_VAR".to_string()],
            workdir: None,
            env: Some(env),
        });

        let result = executor.execute(&config).await.unwrap();
        assert_eq!(result.status, TaskStatus::Success);
        assert!(result.stdout.unwrap().contains("test_value"));
    }

    #[tokio::test]
    async fn test_shell_executor_timeout() {
        let executor = ShellExecutor::new();
        let config = ShellConfig {
            command: "/bin/sleep".to_string(),
            args: vec!["10".to_string()],
            workdir: None,
            env: None,
        };

        // Execute with 1 second timeout
        let result = executor.execute_shell(&config, 1).await;
        assert!(matches!(result, Err(PicoFlowError::TaskTimeout { .. })));
    }

    #[tokio::test]
    async fn test_shell_executor_health_check() {
        let executor = ShellExecutor::new();
        let result = executor.health_check().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_truncate_output() {
        use crate::executors::truncate_output_bytes;

        let small_data = b"hello";
        let (output, truncated) = truncate_output_bytes(small_data);
        assert_eq!(output, "hello");
        assert!(!truncated);

        // Create large data
        let large_data = vec![b'x'; MAX_OUTPUT_SIZE + 1000];
        let (output, truncated) = truncate_output_bytes(&large_data);
        assert_eq!(output.len(), MAX_OUTPUT_SIZE);
        assert!(truncated);
    }

    #[tokio::test]
    async fn test_invalid_command() {
        let executor = ShellExecutor::new();
        let config = TaskExecutorConfig::Shell(ShellConfig {
            command: "/nonexistent/command".to_string(),
            args: vec![],
            workdir: None,
            env: None,
        });

        let result = executor.execute(&config).await;
        assert!(result.is_err());
    }
}
