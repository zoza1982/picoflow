//! Core data models for PicoFlow workflow orchestrator

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// Input validation limits (from ARCHITECTURE.md)
pub const MAX_YAML_SIZE: usize = 1_048_576; // 1 MB
pub const MAX_TASK_COUNT: usize = 1_000;
pub const MAX_TASK_NAME_LEN: usize = 64;
pub const MAX_COMMAND_LEN: usize = 4_096; // 4 KB
pub const MAX_ARG_COUNT: usize = 256;
pub const MAX_ARG_LEN: usize = 4_096; // 4 KB
pub const MAX_OUTPUT_SIZE: usize = 10_485_760; // 10 MB
pub const MAX_RESPONSE_SIZE: usize = 10_485_760; // 10 MB

/// Workflow configuration parsed from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>, // Cron expression
    #[serde(default)]
    pub config: WorkflowGlobalConfig,
    pub tasks: Vec<TaskConfig>,
}

/// Global workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGlobalConfig {
    #[serde(default = "default_max_parallel")]
    pub max_parallel: usize,
    #[serde(default = "default_retry")]
    pub retry_default: u32,
    #[serde(default = "default_timeout")]
    pub timeout_default: u64, // seconds
}

impl Default for WorkflowGlobalConfig {
    fn default() -> Self {
        Self {
            max_parallel: default_max_parallel(),
            retry_default: default_retry(),
            timeout_default: default_timeout(),
        }
    }
}

fn default_max_parallel() -> usize {
    4
}

fn default_retry() -> u32 {
    3
}

fn default_timeout() -> u64 {
    300
}

/// Individual task configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub task_type: TaskType,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub config: TaskExecutorConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>, // seconds
    #[serde(default)]
    pub continue_on_failure: bool,
}

/// Task type variants
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Shell,
    Ssh,
    Http,
}

/// Executor-specific configuration (enum for different task types)
///
/// Note: Uses untagged deserialization since the task type is specified
/// separately in TaskConfig. Variants are ordered by specificity (most
/// unique required fields first) for reliable deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskExecutorConfig {
    /// SSH config requires host and user (most specific)
    Ssh(SshConfig),
    /// HTTP config requires url (specific)
    Http(HttpConfig),
    /// Shell config only requires command (least specific, must be last)
    Shell(ShellConfig),
}

/// Shell executor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellConfig {
    pub command: String, // Absolute path to binary
    #[serde(default)]
    pub args: Vec<String>, // Arguments as list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>, // Working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>, // Environment variables
}

/// SSH executor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SshConfig {
    pub host: String,
    pub user: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>, // Path to SSH private key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>, // Default: 22
    #[serde(default = "default_verify_host_key")]
    pub verify_host_key: bool, // Default: true (for security)
}

fn default_verify_host_key() -> bool {
    true
}

/// HTTP executor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpConfig {
    pub url: String,
    #[serde(default = "default_http_method")]
    pub method: HttpMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_yaml::Value>, // JSON body
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// HTTP request timeout in seconds.
    ///
    /// Note: This timeout is independent of the task-level timeout. The scheduler
    /// applies the task-level timeout (from TaskConfig.timeout) as an outer timeout,
    /// while this timeout applies specifically to the HTTP request. The effective
    /// timeout will be whichever occurs first.
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_allow_private_ips")]
    pub allow_private_ips: bool, // Default: false (for security, blocks SSRF)
}

fn default_allow_private_ips() -> bool {
    false
}

fn default_http_method() -> HttpMethod {
    HttpMethod::Get
}

/// HTTP methods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

/// Task execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Success,
    Failed,
    Retrying,
    Timeout,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Success => write!(f, "success"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Retrying => write!(f, "retrying"),
            TaskStatus::Timeout => write!(f, "timeout"),
        }
    }
}

/// Result of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub duration: Duration,
    #[serde(default)]
    pub output_truncated: bool, // True if output exceeded MAX_OUTPUT_SIZE
}

/// Workflow execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub id: i64,
    pub workflow_id: i64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: TaskStatus,
}

/// Task execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    pub id: i64,
    pub execution_id: i64,
    pub task_name: String,
    pub status: TaskStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub attempt: i32,
    pub retry_count: i32,
    pub next_retry_at: Option<DateTime<Utc>>,
}

/// Workflow summary with execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSummary {
    pub name: String,
    pub schedule: Option<String>,
    pub execution_count: i64,
    pub success_count: i64,
    pub failed_count: i64,
    pub last_execution: Option<DateTime<Utc>>,
}

/// Detailed workflow execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatistics {
    pub total_executions: i64,
    pub success_count: i64,
    pub failed_count: i64,
    pub success_rate: f64, // Percentage (0-100)
    pub failure_rate: f64, // Percentage (0-100)
    pub avg_duration_seconds: Option<f64>,
    pub last_24h_count: i64,
    pub last_execution: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_config_default() {
        let config = WorkflowGlobalConfig::default();
        assert_eq!(config.max_parallel, 4);
        assert_eq!(config.retry_default, 3);
        assert_eq!(config.timeout_default, 300);
    }

    #[test]
    fn test_task_status_display() {
        assert_eq!(TaskStatus::Pending.to_string(), "pending");
        assert_eq!(TaskStatus::Running.to_string(), "running");
        assert_eq!(TaskStatus::Success.to_string(), "success");
        assert_eq!(TaskStatus::Failed.to_string(), "failed");
        assert_eq!(TaskStatus::Retrying.to_string(), "retrying");
        assert_eq!(TaskStatus::Timeout.to_string(), "timeout");
    }

    #[test]
    fn test_task_type_serde() {
        let yaml = r#"shell"#;
        let task_type: TaskType = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(task_type, TaskType::Shell);

        let yaml = r#"ssh"#;
        let task_type: TaskType = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(task_type, TaskType::Ssh);

        let yaml = r#"http"#;
        let task_type: TaskType = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(task_type, TaskType::Http);
    }

    #[test]
    fn test_http_method_serde() {
        let yaml = r#"GET"#;
        let method: HttpMethod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(method, HttpMethod::Get);

        let yaml = r#"POST"#;
        let method: HttpMethod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(method, HttpMethod::Post);
    }

    #[test]
    fn test_shell_config_serde() {
        let yaml = r#"
command: "/usr/bin/ls"
args:
  - "-la"
  - "/tmp"
workdir: "/home/user"
"#;
        let config: ShellConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.command, "/usr/bin/ls");
        assert_eq!(config.args.len(), 2);
        assert_eq!(config.args[0], "-la");
        assert_eq!(config.workdir, Some("/home/user".to_string()));
    }
}
