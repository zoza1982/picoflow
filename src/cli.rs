//! CLI interface for PicoFlow

use crate::dag::DagEngine;
use crate::logging::{init_logging, LogConfig, LogFormat, LogLevel};
use crate::parser::parse_workflow_file;
use crate::scheduler::TaskScheduler;
use crate::state::StateManager;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

/// PicoFlow - Lightweight DAG workflow orchestrator for edge devices
#[derive(Parser, Debug)]
#[command(name = "picoflow")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Lightweight DAG workflow orchestrator for edge devices", long_about = None)]
pub struct Cli {
    /// Log level (error, warn, info, debug, trace)
    #[arg(short, long, default_value = "info", global = true)]
    pub log_level: String,

    /// Log format (json or pretty)
    #[arg(long, default_value = "json", global = true)]
    pub log_format: String,

    /// Database path for state persistence
    #[arg(long, default_value = "picoflow.db", global = true)]
    pub db_path: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Execute a workflow once
    Run {
        /// Path to workflow YAML file
        workflow: PathBuf,
    },

    /// Validate workflow YAML and DAG
    Validate {
        /// Path to workflow YAML file
        workflow: PathBuf,
    },

    /// Show workflow execution status
    Status {
        /// Workflow name (optional, shows all if not specified)
        #[arg(short, long)]
        workflow: Option<String>,

        /// Number of recent executions to show
        #[arg(long, default_value = "10")]
        limit: usize,
    },
}

impl Cli {
    /// Initialize logging based on CLI arguments
    pub fn init_logging(&self) -> anyhow::Result<()> {
        let log_level: LogLevel = self.log_level.as_str().into();
        let log_format = match self.log_format.as_str() {
            "pretty" => LogFormat::Pretty,
            _ => LogFormat::Json,
        };

        let config = LogConfig {
            level: log_level,
            format: log_format,
        };

        init_logging(&config)
    }

    /// Execute the CLI command
    pub async fn execute(&self) -> anyhow::Result<()> {
        match &self.command {
            Commands::Run { workflow } => {
                self.run_workflow(workflow).await?;
            }
            Commands::Validate { workflow } => {
                self.validate_workflow(workflow)?;
            }
            Commands::Status { workflow, limit } => {
                self.show_status(workflow.as_deref(), *limit)?;
            }
        }
        Ok(())
    }

    /// Run a workflow once
    async fn run_workflow(&self, workflow_path: &PathBuf) -> anyhow::Result<()> {
        info!("Loading workflow from: {:?}", workflow_path);

        // Parse workflow
        let config = parse_workflow_file(workflow_path)?;
        info!("Workflow '{}' loaded successfully", config.name);

        // Validate DAG
        let dag = DagEngine::build(&config.tasks)?;
        dag.validate_acyclic()?;
        info!("DAG validation successful");

        // Create state manager
        let state_manager = Arc::new(StateManager::new(&self.db_path)?);

        // Check for crashed executions
        let crashed = state_manager.recover_from_crash()?;
        if !crashed.is_empty() {
            info!("Recovered {} crashed executions", crashed.len());
        }

        // Create scheduler and execute
        let scheduler = TaskScheduler::new(state_manager);
        let success = scheduler.execute_workflow(&config).await?;

        if success {
            info!("Workflow completed successfully");
            Ok(())
        } else {
            error!("Workflow failed");
            std::process::exit(1);
        }
    }

    /// Validate a workflow without executing
    fn validate_workflow(&self, workflow_path: &PathBuf) -> anyhow::Result<()> {
        info!("Validating workflow: {:?}", workflow_path);

        // Parse workflow
        let config = parse_workflow_file(workflow_path)?;
        info!("Workflow '{}' parsed successfully", config.name);

        // Validate DAG
        let dag = DagEngine::build(&config.tasks)?;
        dag.validate_acyclic()?;

        let execution_order = dag.topological_sort()?;

        info!("Workflow validation successful");
        info!("Tasks: {}", config.tasks.len());
        info!("Execution order: {:?}", execution_order);

        println!("Workflow '{}' is valid", config.name);
        println!("Tasks: {}", config.tasks.len());
        println!("Execution order: {}", execution_order.join(" -> "));

        Ok(())
    }

    /// Show execution status
    fn show_status(&self, workflow_name: Option<&str>, limit: usize) -> anyhow::Result<()> {
        let state_manager = StateManager::new(&self.db_path)?;

        if let Some(name) = workflow_name {
            // Show status for specific workflow
            let history = state_manager.get_execution_history(name, limit)?;

            println!("Workflow: {}", name);
            println!("Recent executions (limit {}):", limit);
            println!();

            if history.is_empty() {
                println!("No executions found");
                return Ok(());
            }

            for exec in history {
                println!("Execution ID: {}", exec.id);
                println!("  Status: {}", exec.status);
                println!("  Started: {}", exec.started_at);
                if let Some(completed) = exec.completed_at {
                    println!("  Completed: {}", completed);
                    let duration = completed
                        .signed_duration_since(exec.started_at)
                        .to_std()
                        .unwrap_or_default();
                    println!("  Duration: {:?}", duration);
                }

                // Get task details
                let tasks = state_manager.get_task_executions(exec.id)?;
                println!("  Tasks:");
                for task in tasks {
                    println!(
                        "    - {} [{}] (attempt {})",
                        task.task_name, task.status, task.attempt
                    );
                }
                println!();
            }
        } else {
            println!("Use --workflow <name> to see execution history for a specific workflow");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse() {
        let cli = Cli::parse_from(["picoflow", "validate", "workflow.yaml"]);
        assert!(matches!(cli.command, Commands::Validate { .. }));
    }

    #[test]
    fn test_cli_run_command() {
        let cli = Cli::parse_from(["picoflow", "run", "workflow.yaml"]);
        assert!(matches!(cli.command, Commands::Run { .. }));
    }

    #[test]
    fn test_cli_status_command() {
        let cli = Cli::parse_from(["picoflow", "status"]);
        assert!(matches!(cli.command, Commands::Status { .. }));
    }

    #[test]
    fn test_cli_with_log_level() {
        let cli = Cli::parse_from(["picoflow", "--log-level", "debug", "validate", "test.yaml"]);
        assert_eq!(cli.log_level, "debug");
    }

    #[test]
    fn test_cli_with_db_path() {
        let cli = Cli::parse_from([
            "picoflow",
            "--db-path",
            "/tmp/test.db",
            "validate",
            "test.yaml",
        ]);
        assert_eq!(cli.db_path, PathBuf::from("/tmp/test.db"));
    }
}
