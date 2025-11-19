//! Daemon mode for PicoFlow
//!
//! This module provides background daemon functionality with:
//! - PID file management for single-instance enforcement
//! - Signal handling (SIGTERM for graceful shutdown, SIGHUP for reload)
//! - Cron scheduler integration for automated workflow execution
//! - Graceful shutdown that waits for running tasks to complete
//!
//! # Example
//!
//! ```no_run
//! use picoflow::daemon::Daemon;
//! use picoflow::models::WorkflowConfig;
//! use picoflow::state::StateManager;
//! use std::sync::Arc;
//! use std::path::PathBuf;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let state_manager = Arc::new(StateManager::new("picoflow.db").await?);
//! let pid_file = PathBuf::from("/var/run/picoflow.pid");
//!
//! let mut daemon = Daemon::new(state_manager, pid_file).await?;
//!
//! // Add workflow with schedule (6-field cron format: sec min hour day month dayofweek)
//! let workflow: WorkflowConfig = serde_yaml::from_str(r#"
//! name: example
//! schedule: "0 0 2 * * *"
//! tasks: []
//! "#)?;
//! daemon.add_workflow(workflow).await?;
//!
//! // Start daemon (blocks until shutdown signal)
//! daemon.run().await?;
//! # Ok(())
//! # }
//! ```

use crate::cron_scheduler::CronScheduler;
use crate::error::{PicoFlowError, Result};
use crate::models::WorkflowConfig;
use crate::state::StateManager;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

/// Daemon manager for PicoFlow background service
///
/// Manages the lifecycle of the PicoFlow daemon including:
/// - PID file creation and cleanup
/// - Signal handling for graceful shutdown
/// - Cron scheduler for automated workflow execution
pub struct Daemon {
    /// State manager for workflow execution tracking
    state_manager: Arc<StateManager>,
    /// Cron scheduler for automated workflow execution
    cron_scheduler: CronScheduler,
    /// Path to PID file
    pid_file: PathBuf,
    /// Shutdown signal sender
    shutdown_tx: watch::Sender<bool>,
    /// Shutdown signal receiver
    shutdown_rx: watch::Receiver<bool>,
}

impl Daemon {
    /// Create a new daemon instance
    ///
    /// # Arguments
    ///
    /// * `state_manager` - State manager for workflow execution tracking
    /// * `pid_file` - Path to PID file (e.g., /var/run/picoflow.pid)
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - New daemon instance
    ///
    /// # Errors
    ///
    /// * `PicoFlowError::Other` - If daemon is already running (PID file exists)
    pub async fn new(state_manager: Arc<StateManager>, pid_file: PathBuf) -> Result<Self> {
        info!("Initializing PicoFlow daemon");

        // Check if daemon is already running
        if pid_file.exists() {
            let existing_pid = fs::read_to_string(&pid_file)
                .unwrap_or_else(|_| "unknown".to_string())
                .trim()
                .to_string();

            return Err(PicoFlowError::Other(format!(
                "Daemon already running with PID {} (or stale PID file exists at {:?})",
                existing_pid, pid_file
            )));
        }

        // Create cron scheduler
        let cron_scheduler = CronScheduler::new(state_manager.clone()).await?;

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Ok(Self {
            state_manager,
            cron_scheduler,
            pid_file,
            shutdown_tx,
            shutdown_rx,
        })
    }

    /// Add a workflow to the daemon scheduler
    ///
    /// # Arguments
    ///
    /// * `workflow` - Workflow configuration with cron schedule
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error
    pub async fn add_workflow(&mut self, workflow: WorkflowConfig) -> Result<()> {
        self.cron_scheduler.add_workflow(workflow).await
    }

    /// Write PID file with current process ID
    fn write_pid_file(&self) -> Result<()> {
        let pid = std::process::id();
        info!("Writing PID file: {:?} (PID: {})", self.pid_file, pid);

        fs::write(&self.pid_file, pid.to_string()).map_err(|e| {
            PicoFlowError::Io(std::io::Error::other(format!(
                "Failed to write PID file: {}",
                e
            )))
        })?;

        Ok(())
    }

    /// Remove PID file
    fn remove_pid_file(&self) -> Result<()> {
        if self.pid_file.exists() {
            info!("Removing PID file: {:?}", self.pid_file);
            fs::remove_file(&self.pid_file).map_err(|e| {
                PicoFlowError::Io(std::io::Error::other(format!(
                    "Failed to remove PID file: {}",
                    e
                )))
            })?;
        }
        Ok(())
    }

    /// Run the daemon (blocks until shutdown signal)
    ///
    /// This starts the cron scheduler and waits for shutdown signals:
    /// - SIGTERM: Graceful shutdown
    /// - SIGINT: Graceful shutdown (Ctrl+C)
    /// - SIGHUP: Reload configuration (not yet implemented)
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting PicoFlow daemon");

        // Write PID file
        self.write_pid_file()?;

        // Ensure PID file cleanup on drop
        let pid_file = self.pid_file.clone();
        let _guard = PidFileGuard { pid_file };

        // Recover from any crashed executions
        let crashed = self.state_manager.recover_from_crash().await?;
        if !crashed.is_empty() {
            warn!("Recovered {} crashed executions on startup", crashed.len());
        }

        // Start cron scheduler
        self.cron_scheduler.start().await?;

        info!("Daemon started successfully, waiting for signals...");

        // Setup signal handlers
        let mut sigterm = signal(SignalKind::terminate())
            .map_err(|e| PicoFlowError::Other(format!("Failed to setup SIGTERM handler: {}", e)))?;

        let mut sigint = signal(SignalKind::interrupt())
            .map_err(|e| PicoFlowError::Other(format!("Failed to setup SIGINT handler: {}", e)))?;

        let mut sighup = signal(SignalKind::hangup())
            .map_err(|e| PicoFlowError::Other(format!("Failed to setup SIGHUP handler: {}", e)))?;

        // Wait for signals
        loop {
            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating graceful shutdown");
                    break;
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT, initiating graceful shutdown");
                    break;
                }
                _ = sighup.recv() => {
                    info!("Received SIGHUP, reload not yet implemented");
                    // TODO: Implement config reload
                    // For now, just log and continue waiting
                }
            }
        }

        // Initiate graceful shutdown
        self.shutdown().await?;

        info!("Daemon shutdown complete");

        Ok(())
    }

    /// Gracefully shutdown the daemon
    ///
    /// This stops the cron scheduler and waits for any running tasks to complete.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down daemon...");

        // Signal shutdown
        let _ = self.shutdown_tx.send(true);

        // Shutdown cron scheduler (this will wait for running jobs)
        self.cron_scheduler.shutdown().await?;

        // Remove PID file
        self.remove_pid_file()?;

        info!("Daemon shutdown complete");

        Ok(())
    }

    /// Check if shutdown has been requested
    pub fn is_shutdown_requested(&self) -> bool {
        *self.shutdown_rx.borrow()
    }
}

/// RAII guard for PID file cleanup
struct PidFileGuard {
    pid_file: PathBuf,
}

impl Drop for PidFileGuard {
    fn drop(&mut self) {
        if self.pid_file.exists() {
            debug!("PidFileGuard: Cleaning up PID file: {:?}", self.pid_file);
            if let Err(e) = fs::remove_file(&self.pid_file) {
                error!("Failed to remove PID file in guard: {}", e);
            }
        }
    }
}

/// Check if daemon is running by reading PID file
///
/// # Arguments
///
/// * `pid_file` - Path to PID file
///
/// # Returns
///
/// * `Result<Option<u32>>` - PID if running, None if not running, error if can't determine
pub fn check_daemon_running(pid_file: &Path) -> Result<Option<u32>> {
    if !pid_file.exists() {
        return Ok(None);
    }

    let pid_str = fs::read_to_string(pid_file).map_err(|e| {
        PicoFlowError::Io(std::io::Error::other(format!(
            "Failed to read PID file: {}",
            e
        )))
    })?;

    let pid: u32 = pid_str
        .trim()
        .parse()
        .map_err(|e| PicoFlowError::Other(format!("Invalid PID in file: {}", e)))?;

    // Check if process is actually running
    // On Unix, we can use kill(pid, 0) to check without actually killing
    #[cfg(unix)]
    {
        use std::io::ErrorKind;

        // SAFETY: Using libc::kill with signal 0 is safe for process existence checks.
        // This is a standard POSIX operation that checks if a process exists without
        // sending any actual signal. The PID is validated from our PID file and converted
        // to i32 which is the required type for POSIX kill(2).
        // Note: There is a small TOCTOU (time-of-check-time-of-use) race condition if
        // the PID is reused between reading the file and this check, but this is an
        // inherent limitation of PID-based process management.
        let result = unsafe { libc::kill(pid as i32, 0) };

        if result == 0 {
            // Process exists
            Ok(Some(pid))
        } else {
            let err = std::io::Error::last_os_error();
            match err.kind() {
                ErrorKind::PermissionDenied => {
                    // Process exists but we don't have permission
                    Ok(Some(pid))
                }
                ErrorKind::NotFound => {
                    // Process doesn't exist, stale PID file
                    warn!("Stale PID file found, removing");
                    let _ = fs::remove_file(pid_file);
                    Ok(None)
                }
                _ => Err(PicoFlowError::Other(format!(
                    "Error checking process: {}",
                    err
                ))),
            }
        }
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, just return the PID
        // (Windows doesn't have a direct equivalent to kill(pid, 0))
        Ok(Some(pid))
    }
}

/// Stop a running daemon by sending SIGTERM
///
/// # Arguments
///
/// * `pid_file` - Path to PID file
///
/// # Returns
///
/// * `Result<()>` - Success or error
pub fn stop_daemon(pid_file: &Path) -> Result<()> {
    let pid = check_daemon_running(pid_file)?
        .ok_or_else(|| PicoFlowError::Other("Daemon is not running".to_string()))?;

    info!("Stopping daemon (PID: {})", pid);

    #[cfg(unix)]
    {
        // SAFETY: Using libc::kill to send SIGTERM is safe for graceful process termination.
        // This is a standard POSIX signal (15) that requests the process to terminate gracefully.
        // The PID is validated by check_daemon_running() which confirms the process exists.
        // The kill(2) syscall is a well-defined operation in POSIX systems.
        // Note: If the PID was reused between the check and this signal, we may signal the
        // wrong process, but this is an inherent limitation of PID-based process management.
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }
    }

    #[cfg(not(unix))]
    {
        return Err(PicoFlowError::Other(
            "Stopping daemon not supported on this platform".to_string(),
        ));
    }

    // Wait for PID file to be removed (with timeout)
    let timeout = std::time::Duration::from_secs(30);
    let start = std::time::Instant::now();

    while pid_file.exists() && start.elapsed() < timeout {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    if pid_file.exists() {
        warn!("PID file still exists after timeout, daemon may not have stopped cleanly");
    } else {
        info!("Daemon stopped successfully");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_check_daemon_not_running() {
        let temp_dir = TempDir::new().unwrap();
        let pid_file = temp_dir.path().join("test.pid");

        let result = check_daemon_running(&pid_file).unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_write_and_remove_pid_file() {
        let temp_dir = TempDir::new().unwrap();
        let pid_file = temp_dir.path().join("test.pid");
        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());

        let daemon = Daemon::new(state_manager, pid_file.clone()).await.unwrap();

        // Write PID file
        daemon.write_pid_file().unwrap();
        assert!(pid_file.exists());

        // Verify PID is written
        let pid_str = fs::read_to_string(&pid_file).unwrap();
        let pid: u32 = pid_str.trim().parse().unwrap();
        assert_eq!(pid, std::process::id());

        // Remove PID file
        daemon.remove_pid_file().unwrap();
        assert!(!pid_file.exists());
    }

    #[tokio::test]
    async fn test_daemon_already_running() {
        let temp_dir = TempDir::new().unwrap();
        let pid_file = temp_dir.path().join("test.pid");

        // Create fake PID file
        fs::write(&pid_file, "12345").unwrap();

        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());
        let result = Daemon::new(state_manager, pid_file).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(PicoFlowError::Other(_))));
    }

    #[tokio::test]
    async fn test_daemon_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let pid_file = temp_dir.path().join("test.pid");
        let state_manager = Arc::new(StateManager::in_memory().await.unwrap());

        let mut daemon = Daemon::new(state_manager, pid_file.clone()).await.unwrap();

        // Write PID file
        daemon.write_pid_file().unwrap();
        assert!(pid_file.exists());

        // Shutdown
        daemon.shutdown().await.unwrap();
        assert!(!pid_file.exists());
    }
}
