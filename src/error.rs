//! Error types for PicoFlow

use thiserror::Error;

/// PicoFlow error types
#[derive(Error, Debug)]
pub enum PicoFlowError {
    /// YAML parsing errors
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    /// YAML size exceeded limit
    #[error("Workflow YAML exceeds 1MB limit (size: {0} bytes)")]
    YamlSizeExceeded(usize),

    /// Task count exceeded limit
    #[error("Task count {count} exceeds limit of {limit}")]
    TaskCountExceeded { count: usize, limit: usize },

    /// Task name validation error
    #[error("Task name '{name}' exceeds {max} characters")]
    TaskNameTooLong { name: String, max: usize },

    /// Invalid task name format
    #[error("Invalid task name '{name}': only alphanumeric, underscore, and dash allowed")]
    InvalidTaskName { name: String },

    /// DAG errors
    #[error("Cycle detected in DAG: {0}")]
    CycleDetected(String),

    /// Missing task dependency
    #[error("Task '{task}' depends on non-existent task '{dependency}'")]
    MissingDependency { task: String, dependency: String },

    /// Database errors
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Task execution timeout
    #[error("Task '{task}' timed out after {timeout} seconds")]
    TaskTimeout { task: String, timeout: u64 },

    /// Command validation errors
    #[error("Command exceeds {limit} bytes")]
    CommandTooLong { limit: usize },

    /// Argument validation errors
    #[error("Argument count {count} exceeds limit of {limit}")]
    ArgCountExceeded { count: usize, limit: usize },

    #[error("Argument exceeds {limit} bytes")]
    ArgTooLong { limit: usize },

    /// Path validation errors
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Path traversal attempt
    #[error("Path traversal detected in: {0}")]
    PathTraversal(String),

    /// Output size exceeded
    #[error("Output size exceeded {limit} bytes")]
    OutputSizeExceeded { limit: usize },

    /// SSH errors
    #[error("SSH error: {0}")]
    Ssh(String),

    /// HTTP errors
    #[error("HTTP error: {0}")]
    Http(String),

    /// Generic error
    #[error("Error: {0}")]
    Other(String),
}

/// Result type alias using PicoFlowError
pub type Result<T> = std::result::Result<T, PicoFlowError>;
