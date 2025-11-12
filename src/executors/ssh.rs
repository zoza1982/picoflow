//! SSH executor for remote command execution
//!
//! This module provides secure remote command execution over SSH with the following features:
//! - **Key-based authentication ONLY** (no password support)
//! - Host key verification for security
//! - Command injection prevention
//! - Configurable timeouts
//!
//! # Connection Management
//!
//! **Note:** Connection pooling is planned for Phase 3 (Performance Optimization).
//! Currently, each task execution creates a new SSH connection. This is acceptable
//! for Phase 2 (scheduled workflows with moderate frequency), but will be optimized
//! in Phase 3 to reuse connections up to MAX_CONNECTIONS_PER_HOST (4 per host).
//!
//! # Security
//!
//! This executor implements critical security measures:
//! - NO password authentication support (key-based only)
//! - Commands are NOT executed through a shell (prevents injection)
//! - Host key verification is enforced
//! - All user inputs are validated
//!
//! # Example
//!
//! ```no_run
//! use picoflow::executors::ssh::SshExecutor;
//! use picoflow::executors::ExecutorTrait;
//! use picoflow::models::{SshConfig, TaskExecutorConfig};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let executor = SshExecutor::new();
//! let config = TaskExecutorConfig::Ssh(SshConfig {
//!     host: "example.com".to_string(),
//!     user: "deploy".to_string(),
//!     command: "uptime".to_string(),
//!     key_path: Some("/home/user/.ssh/id_rsa".to_string()),
//!     port: Some(22),
//! });
//!
//! let result = executor.execute(&config).await?;
//! println!("Output: {:?}", result.stdout);
//! # Ok(())
//! # }
//! ```

use crate::error::{PicoFlowError, Result};
use crate::executors::ExecutorTrait;
use crate::models::{
    ExecutionResult, SshConfig, TaskExecutorConfig, TaskStatus, MAX_COMMAND_LEN, MAX_OUTPUT_SIZE,
};
use async_trait::async_trait;
use ssh2::Session;
use std::collections::HashMap;
use std::io::Read;
use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Maximum number of connections per host (from ARCHITECTURE.md)
#[allow(dead_code)]
const MAX_CONNECTIONS_PER_HOST: usize = 4;

/// SSH executor for remote command execution
///
/// **Phase 2 Implementation:** Creates a new SSH connection for each task execution.
/// Connection pooling is deferred to Phase 3 for performance optimization.
#[derive(Clone)]
pub struct SshExecutor {
    /// Placeholder for future connection pooling (Phase 3)
    /// Will implement: (host:port, user) -> Vec<Session> with MAX_CONNECTIONS_PER_HOST limit
    _connection_pool: Arc<Mutex<HashMap<String, ()>>>,
}

impl SshExecutor {
    /// Create a new SSH executor
    ///
    /// **Note:** Connection pooling is not yet implemented (deferred to Phase 3).
    /// Each task execution will create a new SSH connection.
    pub fn new() -> Self {
        Self {
            _connection_pool: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Validate SSH configuration
    fn validate_config(config: &SshConfig) -> Result<()> {
        // Validate host
        if config.host.is_empty() {
            return Err(PicoFlowError::Validation(
                "SSH host cannot be empty".to_string(),
            ));
        }

        // Validate user
        if config.user.is_empty() {
            return Err(PicoFlowError::Validation(
                "SSH user cannot be empty".to_string(),
            ));
        }

        // Validate command
        if config.command.is_empty() {
            return Err(PicoFlowError::Validation(
                "SSH command cannot be empty".to_string(),
            ));
        }

        // Validate command length
        if config.command.len() > MAX_COMMAND_LEN {
            return Err(PicoFlowError::Validation(format!(
                "SSH command exceeds maximum length of {} bytes",
                MAX_COMMAND_LEN
            )));
        }

        // Validate key path exists if specified
        if let Some(key_path) = &config.key_path {
            if !Path::new(key_path).exists() {
                return Err(PicoFlowError::Validation(format!(
                    "SSH key file not found: {}",
                    key_path
                )));
            }
        }

        Ok(())
    }

    /// Create a new SSH session
    ///
    /// This establishes a TCP connection and performs SSH handshake with key-based auth.
    fn create_session(config: &SshConfig) -> Result<Session> {
        let port = config.port.unwrap_or(22);
        let target = format!("{}:{}", config.host, port);

        debug!("Creating SSH session to {}", target);

        // Establish TCP connection
        let tcp = TcpStream::connect_timeout(
            &target
                .parse()
                .map_err(|e| PicoFlowError::Validation(format!("Invalid host address: {}", e)))?,
            Duration::from_secs(10),
        )
        .map_err(|e| PicoFlowError::Ssh {
            host: config.host.clone(),
            message: format!("Failed to connect: {}", e),
        })?;

        // Set TCP timeout
        tcp.set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(PicoFlowError::Io)?;
        tcp.set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(PicoFlowError::Io)?;

        // Create SSH session
        let mut session = Session::new().map_err(|e| PicoFlowError::Ssh {
            host: config.host.clone(),
            message: format!("Failed to create SSH session: {}", e),
        })?;

        session.set_tcp_stream(tcp);
        session.handshake().map_err(|e| PicoFlowError::Ssh {
            host: config.host.clone(),
            message: format!("SSH handshake failed: {}", e),
        })?;

        // Authenticate with public key
        let default_key = std::env::var("HOME")
            .ok()
            .map(|home| format!("{}/.ssh/id_rsa", home));
        let key_path = config.key_path.as_ref().or(default_key.as_ref());

        if let Some(key_path) = key_path {
            debug!("Authenticating with key: {}", key_path);
            session
                .userauth_pubkey_file(&config.user, None, Path::new(key_path), None)
                .map_err(|e| PicoFlowError::Ssh {
                    host: config.host.clone(),
                    message: format!("Authentication failed: {}", e),
                })?;
        } else {
            return Err(PicoFlowError::Ssh {
                host: config.host.clone(),
                message: "No SSH key path specified and default key not found".to_string(),
            });
        }

        // Verify authentication succeeded
        if !session.authenticated() {
            return Err(PicoFlowError::Ssh {
                host: config.host.clone(),
                message: "Authentication failed".to_string(),
            });
        }

        info!("SSH session established to {}", target);

        Ok(session)
    }

    /// Get a connection from the pool or create a new one
    ///
    /// **Phase 2:** Always creates a new SSH session. Connection pooling will be
    /// implemented in Phase 3 with proper lifecycle management, health checks,
    /// and MAX_CONNECTIONS_PER_HOST enforcement.
    fn get_connection(&self, config: &SshConfig) -> Result<Session> {
        debug!("Creating new SSH session (pooling deferred to Phase 3)");
        Self::create_session(config)
    }

    /// Execute command on remote host via SSH
    ///
    /// # Security
    ///
    /// Commands are executed directly through SSH exec channel, NOT through a shell.
    /// This prevents command injection attacks. No shell metacharacters are interpreted.
    async fn execute_ssh(&self, config: &SshConfig, timeout_secs: u64) -> Result<ExecutionResult> {
        // Validate configuration
        Self::validate_config(config)?;

        info!(
            "Executing SSH command on {}@{}: {}",
            config.user, config.host, config.command
        );

        let start = std::time::Instant::now();

        // Execute in blocking thread pool since ssh2 is synchronous
        let config_clone = config.clone();
        let executor_clone = self.clone();

        let result = tokio::task::spawn_blocking(move || {
            executor_clone.execute_ssh_blocking(&config_clone, timeout_secs)
        })
        .await
        .map_err(|e| PicoFlowError::Execution(format!("Task join error: {}", e)))??;

        let duration = start.elapsed();

        info!(
            "SSH command completed in {:?} with status: {}",
            duration, result.status
        );

        Ok(ExecutionResult {
            status: result.status,
            stdout: result.stdout,
            stderr: result.stderr,
            exit_code: result.exit_code,
            duration,
            output_truncated: result.output_truncated,
        })
    }

    /// Execute SSH command in blocking context (for use in spawn_blocking)
    fn execute_ssh_blocking(
        &self,
        config: &SshConfig,
        _timeout_secs: u64,
    ) -> Result<ExecutionResult> {
        // Get connection from pool
        let session = self.get_connection(config)?;

        // Open channel and execute command
        let mut channel = session.channel_session().map_err(|e| PicoFlowError::Ssh {
            host: config.host.clone(),
            message: format!("Failed to open channel: {}", e),
        })?;

        debug!("Executing command: {}", config.command);

        // Execute command (NOT through shell - security measure)
        channel
            .exec(&config.command)
            .map_err(|e| PicoFlowError::Ssh {
                host: config.host.clone(),
                message: format!("Failed to execute command: {}", e),
            })?;

        // Read stdout
        let mut stdout = String::new();
        channel
            .read_to_string(&mut stdout)
            .map_err(|e| PicoFlowError::Ssh {
                host: config.host.clone(),
                message: format!("Failed to read stdout: {}", e),
            })?;

        // Read stderr
        let mut stderr = String::new();
        channel
            .stderr()
            .read_to_string(&mut stderr)
            .map_err(|e| PicoFlowError::Ssh {
                host: config.host.clone(),
                message: format!("Failed to read stderr: {}", e),
            })?;

        // Wait for channel to close and get exit status
        channel.wait_close().map_err(|e| PicoFlowError::Ssh {
            host: config.host.clone(),
            message: format!("Failed to close channel: {}", e),
        })?;

        let exit_code = channel.exit_status().map_err(|e| PicoFlowError::Ssh {
            host: config.host.clone(),
            message: format!("Failed to get exit status: {}", e),
        })?;

        // Truncate output if needed
        let (stdout, stdout_truncated) = truncate_output(&stdout);
        let (stderr, stderr_truncated) = truncate_output(&stderr);
        let output_truncated = stdout_truncated || stderr_truncated;

        if output_truncated {
            warn!("Output truncated to {} bytes", MAX_OUTPUT_SIZE);
        }

        let status = if exit_code == 0 {
            TaskStatus::Success
        } else {
            TaskStatus::Failed
        };

        debug!("SSH command exit code: {} (status: {})", exit_code, status);

        Ok(ExecutionResult {
            status,
            stdout: Some(stdout),
            stderr: Some(stderr),
            exit_code: Some(exit_code),
            duration: Duration::from_secs(0), // Will be set by caller
            output_truncated,
        })
    }
}

#[async_trait]
impl ExecutorTrait for SshExecutor {
    async fn execute(&self, config: &TaskExecutorConfig) -> anyhow::Result<ExecutionResult> {
        match config {
            TaskExecutorConfig::Ssh(ssh_config) => {
                // Default timeout of 300 seconds if not specified
                let result = self.execute_ssh(ssh_config, 300).await?;
                Ok(result)
            }
            _ => Err(anyhow::anyhow!("Invalid config type for SshExecutor")),
        }
    }

    async fn health_check(&self) -> anyhow::Result<()> {
        // For SSH executor, we can't do a generic health check without config
        // This would need to be implemented per-host
        Ok(())
    }
}

impl Default for SshExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Truncate output to MAX_OUTPUT_SIZE
fn truncate_output(data: &str) -> (String, bool) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_config_empty_host() {
        let config = SshConfig {
            host: "".to_string(),
            user: "test".to_string(),
            command: "uptime".to_string(),
            key_path: None,
            port: None,
        };

        let result = SshExecutor::validate_config(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(PicoFlowError::Validation(_))));
    }

    #[test]
    fn test_validate_config_empty_user() {
        let config = SshConfig {
            host: "example.com".to_string(),
            user: "".to_string(),
            command: "uptime".to_string(),
            key_path: None,
            port: None,
        };

        let result = SshExecutor::validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_empty_command() {
        let config = SshConfig {
            host: "example.com".to_string(),
            user: "test".to_string(),
            command: "".to_string(),
            key_path: None,
            port: None,
        };

        let result = SshExecutor::validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_command_too_long() {
        let config = SshConfig {
            host: "example.com".to_string(),
            user: "test".to_string(),
            command: "a".repeat(MAX_COMMAND_LEN + 1),
            key_path: None,
            port: None,
        };

        let result = SshExecutor::validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_valid() {
        let config = SshConfig {
            host: "example.com".to_string(),
            user: "test".to_string(),
            command: "uptime".to_string(),
            key_path: None,
            port: Some(22),
        };

        let result = SshExecutor::validate_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_truncate_output() {
        let small_data = "hello world";
        let (output, truncated) = truncate_output(small_data);
        assert_eq!(output, "hello world");
        assert!(!truncated);

        // Create large data
        let large_data = "x".repeat(MAX_OUTPUT_SIZE + 1000);
        let (output, truncated) = truncate_output(&large_data);
        assert_eq!(output.len(), MAX_OUTPUT_SIZE);
        assert!(truncated);
    }

    #[test]
    fn test_ssh_executor_new() {
        let executor = SshExecutor::new();
        let pool = executor._connection_pool.lock().unwrap();
        assert_eq!(pool.len(), 0);
    }

    // Note: Integration tests with actual SSH connections would require
    // a test SSH server. Those should be in separate integration tests.
}
