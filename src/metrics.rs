//! Prometheus metrics collection and HTTP endpoint
//!
//! This module provides Prometheus-format metrics for workflow and task execution.
//! Metrics are exposed via HTTP endpoint `/metrics` on a configurable port (default: 9090).
//!
//! # Available Metrics
//!
//! - `picoflow_workflow_executions_total{workflow, status}` - Counter of workflow executions
//! - `picoflow_task_executions_total{workflow, task, status}` - Counter of task executions
//! - `picoflow_task_duration_seconds{workflow, task}` - Histogram of task durations
//! - `picoflow_active_workflows` - Gauge of currently running workflows
//! - `picoflow_active_tasks` - Gauge of currently running tasks
//! - `picoflow_memory_bytes` - Gauge of process memory usage (RSS)
//!
//! # Performance
//!
//! Target: <5MB additional memory overhead (PRD Phase 3)
//!
//! # Example
//!
//! ```no_run
//! use picoflow::metrics::MetricsServer;
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let metrics = Arc::new(MetricsServer::new());
//!
//! // Start metrics server on port 9090
//! metrics.start(9090).await?;
//!
//! // Record workflow execution
//! metrics.record_workflow_execution("my-workflow", "success");
//!
//! // Record task execution
//! metrics.record_task_execution("my-workflow", "task1", "success", 1.5);
//! # Ok(())
//! # }
//! ```

use prometheus::{
    CounterVec, Encoder, Gauge, HistogramOpts, HistogramVec, Opts, Registry, TextEncoder,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

/// Histogram bucket boundaries for task duration metrics (in seconds)
const TASK_DURATION_BUCKETS: &[f64] = &[0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 300.0];

/// Prometheus metrics server
#[derive(Clone)]
pub struct MetricsServer {
    registry: Arc<Registry>,
    workflow_executions: Arc<CounterVec>,
    task_executions: Arc<CounterVec>,
    task_duration: Arc<HistogramVec>,
    active_workflows: Arc<Gauge>,
    active_tasks: Arc<Gauge>,
    memory_bytes: Arc<Gauge>,
}

impl MetricsServer {
    /// Create a new metrics server with default metrics
    pub fn new() -> Self {
        let registry = Registry::new();

        // Workflow execution counter
        let workflow_executions = CounterVec::new(
            Opts::new(
                "picoflow_workflow_executions_total",
                "Total number of workflow executions",
            ),
            &["workflow", "status"],
        )
        .unwrap();

        // Task execution counter
        let task_executions = CounterVec::new(
            Opts::new(
                "picoflow_task_executions_total",
                "Total number of task executions",
            ),
            &["workflow", "task", "status"],
        )
        .unwrap();

        // Task duration histogram
        let task_duration = HistogramVec::new(
            HistogramOpts::new(
                "picoflow_task_duration_seconds",
                "Task execution duration in seconds",
            )
            .buckets(TASK_DURATION_BUCKETS.to_vec()),
            &["workflow", "task"],
        )
        .unwrap();

        // Active workflows gauge
        let active_workflows =
            Gauge::with_opts(Opts::new("picoflow_active_workflows", "Active workflows")).unwrap();

        // Active tasks gauge
        let active_tasks =
            Gauge::with_opts(Opts::new("picoflow_active_tasks", "Active tasks")).unwrap();

        // Memory usage gauge
        let memory_bytes = Gauge::with_opts(Opts::new(
            "picoflow_memory_bytes",
            "Process memory usage in bytes (RSS)",
        ))
        .unwrap();

        // Register all metrics
        registry
            .register(Box::new(workflow_executions.clone()))
            .unwrap();
        registry
            .register(Box::new(task_executions.clone()))
            .unwrap();
        registry.register(Box::new(task_duration.clone())).unwrap();
        registry
            .register(Box::new(active_workflows.clone()))
            .unwrap();
        registry.register(Box::new(active_tasks.clone())).unwrap();
        registry.register(Box::new(memory_bytes.clone())).unwrap();

        Self {
            registry: Arc::new(registry),
            workflow_executions: Arc::new(workflow_executions),
            task_executions: Arc::new(task_executions),
            task_duration: Arc::new(task_duration),
            active_workflows: Arc::new(active_workflows),
            active_tasks: Arc::new(active_tasks),
            memory_bytes: Arc::new(memory_bytes),
        }
    }

    /// Start the HTTP metrics server on the specified port
    ///
    /// The server exposes `/metrics` endpoint in Prometheus text format.
    ///
    /// # Arguments
    ///
    /// * `port` - TCP port to listen on (default: 9090)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use picoflow::metrics::MetricsServer;
    /// # async fn example() -> anyhow::Result<()> {
    /// let metrics = MetricsServer::new();
    /// metrics.start(9090).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn start(&self, port: u16) -> anyhow::Result<()> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&addr).await?;
        info!("Metrics server listening on http://{}/metrics", addr);

        let registry = self.registry.clone();
        let memory_bytes = self.memory_bytes.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        let registry = registry.clone();
                        let memory_bytes = memory_bytes.clone();

                        tokio::spawn(async move {
                            if let Err(e) =
                                Self::handle_request(stream, registry, memory_bytes).await
                            {
                                error!("Error handling metrics request: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Error accepting connection: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Handle incoming HTTP request
    async fn handle_request(
        mut stream: tokio::net::TcpStream,
        registry: Arc<Registry>,
        memory_bytes: Arc<Gauge>,
    ) -> anyhow::Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await?;
        let request = String::from_utf8_lossy(&buffer[..n]);

        // Parse HTTP request (simple parser for GET /metrics)
        if request.starts_with("GET /metrics") {
            // Update memory usage before exporting
            if let Ok(memory) = Self::get_memory_usage() {
                memory_bytes.set(memory as f64);
            }

            // Gather metrics
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = Vec::new();
            encoder.encode(&metric_families, &mut buffer)?;

            // Send HTTP response
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\n\r\n{}",
                buffer.len(),
                String::from_utf8_lossy(&buffer)
            );

            stream.write_all(response.as_bytes()).await?;
        } else {
            // 404 for other paths
            let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\n\r\nNot Found";
            stream.write_all(response.as_bytes()).await?;
        }

        Ok(())
    }

    /// Get process memory usage in bytes (RSS)
    #[cfg(target_os = "macos")]
    fn get_memory_usage() -> anyhow::Result<u64> {
        use std::mem;

        unsafe {
            let mut info: libc::rusage = mem::zeroed();
            if libc::getrusage(libc::RUSAGE_SELF, &mut info) == 0 {
                // ru_maxrss is in bytes on macOS
                Ok(info.ru_maxrss as u64)
            } else {
                Err(anyhow::anyhow!("Failed to get memory usage"))
            }
        }
    }

    /// Get process memory usage in bytes (RSS)
    #[cfg(target_os = "linux")]
    fn get_memory_usage() -> anyhow::Result<u64> {
        use std::mem;

        unsafe {
            let mut info: libc::rusage = mem::zeroed();
            if libc::getrusage(libc::RUSAGE_SELF, &mut info) == 0 {
                // ru_maxrss is in kilobytes on Linux
                Ok((info.ru_maxrss as u64) * 1024)
            } else {
                Err(anyhow::anyhow!("Failed to get memory usage"))
            }
        }
    }

    /// Get process memory usage (fallback for unsupported platforms)
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn get_memory_usage() -> anyhow::Result<u64> {
        Err(anyhow::anyhow!(
            "Memory usage tracking not supported on this platform"
        ))
    }

    /// Record a workflow execution
    ///
    /// # Arguments
    ///
    /// * `workflow` - Workflow name
    /// * `status` - Execution status ("success", "failed", "timeout")
    pub fn record_workflow_execution(&self, workflow: &str, status: &str) {
        self.workflow_executions
            .with_label_values(&[workflow, status])
            .inc();
    }

    /// Record a task execution
    ///
    /// # Arguments
    ///
    /// * `workflow` - Workflow name
    /// * `task` - Task name
    /// * `status` - Execution status ("success", "failed", "timeout")
    /// * `duration_secs` - Task execution duration in seconds
    pub fn record_task_execution(
        &self,
        workflow: &str,
        task: &str,
        status: &str,
        duration_secs: f64,
    ) {
        self.task_executions
            .with_label_values(&[workflow, task, status])
            .inc();

        self.task_duration
            .with_label_values(&[workflow, task])
            .observe(duration_secs);
    }

    /// Increment active workflows counter
    pub fn inc_active_workflows(&self) {
        self.active_workflows.inc();
    }

    /// Decrement active workflows counter
    pub fn dec_active_workflows(&self) {
        self.active_workflows.dec();
    }

    /// Increment active tasks counter
    pub fn inc_active_tasks(&self) {
        self.active_tasks.inc();
    }

    /// Decrement active tasks counter
    pub fn dec_active_tasks(&self) {
        self.active_tasks.dec();
    }
}

impl Default for MetricsServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_server_creation() {
        let metrics = MetricsServer::new();
        assert!(Arc::strong_count(&metrics.registry) >= 1);
    }

    #[test]
    fn test_record_workflow_execution() {
        let metrics = MetricsServer::new();
        metrics.record_workflow_execution("test-workflow", "success");

        // Just verify metrics can be recorded without error
        // Actual metric gathering requires proto dependencies
    }

    #[test]
    fn test_record_task_execution() {
        let metrics = MetricsServer::new();
        metrics.record_task_execution("test-workflow", "task1", "success", 1.5);

        // Just verify metrics can be recorded without error
    }

    #[test]
    fn test_active_counters() {
        let metrics = MetricsServer::new();

        metrics.inc_active_workflows();
        metrics.inc_active_tasks();

        // Verify counters can be incremented/decremented without error
        metrics.dec_active_workflows();
        metrics.dec_active_tasks();
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn test_memory_usage() {
        let memory = MetricsServer::get_memory_usage();
        assert!(memory.is_ok());
        assert!(memory.unwrap() > 0);
    }
}
