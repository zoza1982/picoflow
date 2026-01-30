//! CLI interface for PicoFlow

use crate::dag::DagEngine;
use crate::logging::{init_logging, LogConfig, LogFormat, LogLevel};
use crate::parser::parse_workflow_file;
use crate::scheduler::TaskScheduler;
use crate::state::StateManager;
use crate::templates;
use clap::{Parser, Subcommand, ValueEnum};
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

    /// Workflow management commands
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommands,
    },

    /// Daemon management commands
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },

    /// Show workflow execution history
    History {
        /// Workflow name
        #[arg(short, long)]
        workflow: String,

        /// Filter by status (success, failed, timeout)
        #[arg(short, long)]
        status: Option<String>,

        /// Number of records to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show workflow execution statistics
    Stats {
        /// Workflow name
        #[arg(short, long)]
        workflow: String,
    },

    /// Show task execution logs
    Logs {
        /// Workflow name
        #[arg(short, long)]
        workflow: String,

        /// Execution ID (optional, shows latest if not specified)
        #[arg(short, long)]
        execution_id: Option<i64>,

        /// Task name filter (optional, shows all if not specified)
        #[arg(short, long)]
        task: Option<String>,
    },

    /// Generate example workflow YAML templates
    Template {
        /// Template type (omit to list available templates)
        #[arg(short = 't', long = "type")]
        template_type: Option<TemplateType>,

        /// Write output to a file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// Available template types for the `template` subcommand.
#[derive(Debug, Clone, ValueEnum)]
pub enum TemplateType {
    /// Single shell task, no dependencies
    Minimal,
    /// Multiple shell tasks with dependencies, retry, timeout
    Shell,
    /// SSH remote execution with key auth
    Ssh,
    /// HTTP API calls (GET/POST) with headers
    Http,
    /// All executor types combined with DAG dependencies
    Full,
}

#[derive(Subcommand, Debug)]
pub enum WorkflowCommands {
    /// List all workflows with execution statistics
    List,
}

#[derive(Subcommand, Debug)]
pub enum DaemonCommands {
    /// Start daemon in background with scheduled workflows
    Start {
        /// Path to workflow YAML file (must have schedule defined)
        workflow: PathBuf,

        /// Path to PID file
        #[arg(long, default_value = "/tmp/picoflow.pid")]
        pid_file: PathBuf,
    },

    /// Stop running daemon
    Stop {
        /// Path to PID file
        #[arg(long, default_value = "/tmp/picoflow.pid")]
        pid_file: PathBuf,
    },

    /// Check daemon status
    Status {
        /// Path to PID file
        #[arg(long, default_value = "/tmp/picoflow.pid")]
        pid_file: PathBuf,
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
                self.show_status(workflow.as_deref(), *limit).await?;
            }
            Commands::Workflow { command } => {
                self.handle_workflow_command(command).await?;
            }
            Commands::Daemon { command } => {
                self.handle_daemon_command(command).await?;
            }
            Commands::History {
                workflow,
                status,
                limit,
            } => {
                self.show_history(workflow, status.as_deref(), *limit)
                    .await?;
            }
            Commands::Stats { workflow } => {
                self.show_stats(workflow).await?;
            }
            Commands::Logs {
                workflow,
                execution_id,
                task,
            } => {
                self.show_logs(workflow, *execution_id, task.as_deref())
                    .await?;
            }
            Commands::Template {
                template_type,
                output,
            } => {
                self.handle_template(template_type.as_ref(), output.as_ref())?;
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
        let state_manager = Arc::new(StateManager::new(&self.db_path).await?);

        // Check for crashed executions
        let crashed = state_manager.recover_from_crash().await?;
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

    /// Handle the `template` subcommand.
    fn handle_template(
        &self,
        template_type: Option<&TemplateType>,
        output: Option<&PathBuf>,
    ) -> anyhow::Result<()> {
        let Some(tt) = template_type else {
            // No type specified — list available templates.
            println!("Available templates:\n");
            let header_type = "TYPE";
            let header_desc = "DESCRIPTION";
            println!("{header_type:<12} {header_desc}");
            println!("{}", "-".repeat(60));
            for info in templates::list_templates() {
                println!("{:<12} {}", info.name, info.description);
            }
            println!();
            println!("Usage: picoflow template --type <TYPE> [-o <FILE>]");
            return Ok(());
        };

        let type_name = match tt {
            TemplateType::Minimal => "minimal",
            TemplateType::Shell => "shell",
            TemplateType::Ssh => "ssh",
            TemplateType::Http => "http",
            TemplateType::Full => "full",
        };

        let content = templates::get_template(type_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown template type: {}", type_name))?;

        if let Some(path) = output {
            use std::fs::OpenOptions;
            use std::io::Write as _;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::AlreadyExists {
                        anyhow::anyhow!(
                            "File '{}' already exists. Remove it first or choose a different name.",
                            path.display()
                        )
                    } else {
                        e.into()
                    }
                })?;
            file.write_all(content.as_bytes())?;
            println!("Template written to {}", path.display());
        } else {
            print!("{content}");
        }

        Ok(())
    }

    /// Show execution status
    async fn show_status(&self, workflow_name: Option<&str>, limit: usize) -> anyhow::Result<()> {
        let state_manager = StateManager::new(&self.db_path).await?;

        if let Some(name) = workflow_name {
            // Show status for specific workflow
            let history = state_manager.get_execution_history(name, limit).await?;

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
                let tasks = state_manager.get_task_executions(exec.id).await?;
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

    /// Handle workflow management commands
    async fn handle_workflow_command(&self, command: &WorkflowCommands) -> anyhow::Result<()> {
        match command {
            WorkflowCommands::List => self.list_workflows().await?,
        }
        Ok(())
    }

    /// List all workflows with execution statistics
    async fn list_workflows(&self) -> anyhow::Result<()> {
        let state_manager = StateManager::new(&self.db_path).await?;
        let workflows = state_manager.list_workflows().await?;

        if workflows.is_empty() {
            println!("No workflows found");
            return Ok(());
        }

        println!("Workflows:");
        println!();
        println!(
            "{:<30} {:<12} {:<12} {:<10} {:<10} {:<20}",
            "Name", "Type", "Total", "Success", "Failed", "Last Execution"
        );
        println!("{}", "-".repeat(110));

        for workflow in workflows {
            let workflow_type = if workflow.schedule.is_some() {
                "Cron"
            } else {
                "On-Demand"
            };

            let last_exec = workflow
                .last_execution
                .map(|dt| {
                    // Convert UTC to local timezone
                    let local_time = dt.with_timezone(&chrono::Local);
                    local_time.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_else(|| "Never".to_string());

            println!(
                "{:<30} {:<12} {:<12} {:<10} {:<10} {:<20}",
                workflow.name,
                workflow_type,
                workflow.execution_count,
                workflow.success_count,
                workflow.failed_count,
                last_exec
            );
        }

        Ok(())
    }

    /// Handle daemon management commands
    async fn handle_daemon_command(&self, command: &DaemonCommands) -> anyhow::Result<()> {
        use crate::daemon::{check_daemon_running, stop_daemon, Daemon};

        match command {
            DaemonCommands::Start { workflow, pid_file } => {
                info!("Starting daemon with workflow: {:?}", workflow);

                // Parse workflow
                let config = parse_workflow_file(workflow)?;

                // Validate workflow has a schedule
                if config.schedule.is_none() {
                    error!("Workflow '{}' has no schedule defined", config.name);
                    return Err(anyhow::anyhow!(
                        "Cannot start daemon with workflow '{}': no schedule defined. \
                         Add a 'schedule' field with a cron expression.",
                        config.name
                    ));
                }

                info!(
                    "Workflow '{}' loaded with schedule: {}",
                    config.name,
                    config.schedule.as_ref().unwrap()
                );

                // Create state manager
                let state_manager = Arc::new(StateManager::new(&self.db_path).await?);

                // Create daemon
                let mut daemon = Daemon::new(state_manager, pid_file.clone()).await?;

                // Add workflow
                daemon.add_workflow(config).await?;

                println!("Starting PicoFlow daemon (PID file: {:?})", pid_file);
                println!("Press Ctrl+C to stop");

                // Run daemon (blocks until signal)
                daemon.run().await?;

                println!("Daemon stopped");
            }

            DaemonCommands::Stop { pid_file } => {
                info!("Stopping daemon (PID file: {:?})", pid_file);

                match check_daemon_running(pid_file)? {
                    Some(pid) => {
                        println!("Stopping daemon (PID: {})", pid);
                        stop_daemon(pid_file)?;
                        println!("Daemon stopped");
                    }
                    None => {
                        println!("Daemon is not running");
                    }
                }
            }

            DaemonCommands::Status { pid_file } => {
                info!("Checking daemon status (PID file: {:?})", pid_file);

                match check_daemon_running(pid_file)? {
                    Some(pid) => {
                        println!("Daemon is running (PID: {})", pid);
                    }
                    None => {
                        println!("Daemon is not running");
                    }
                }
            }
        }

        Ok(())
    }

    /// Show execution history with optional status filter
    async fn show_history(
        &self,
        workflow_name: &str,
        status_filter: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<()> {
        let state_manager = StateManager::new(&self.db_path).await?;

        let executions = state_manager
            .get_execution_history_filtered(workflow_name, status_filter, limit)
            .await?;

        if executions.is_empty() {
            println!(
                "No execution history found for workflow '{}'",
                workflow_name
            );
            return Ok(());
        }

        println!("\nExecution History for '{}'", workflow_name);
        if let Some(status) = status_filter {
            println!("Filtered by status: {}", status);
        }
        println!();
        println!(
            "{:<8} {:<20} {:<20} {:<10} {:<12}",
            "ID", "Started", "Completed", "Status", "Duration"
        );
        println!("{:-<78}", "");

        for exec in &executions {
            let started = exec.started_at.format("%Y-%m-%d %H:%M:%S");
            let completed = exec
                .completed_at
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "N/A".to_string());

            let duration = if let Some(completed_at) = exec.completed_at {
                let duration = (completed_at - exec.started_at).num_seconds();
                format_duration(duration)
            } else {
                "N/A".to_string()
            };

            println!(
                "{:<8} {:<20} {:<20} {:<10} {:<12}",
                exec.id,
                started,
                completed,
                exec.status.to_string(),
                duration
            );
        }

        println!();
        Ok(())
    }

    /// Show workflow execution statistics
    async fn show_stats(&self, workflow_name: &str) -> anyhow::Result<()> {
        let state_manager = StateManager::new(&self.db_path).await?;

        let stats = state_manager.get_workflow_statistics(workflow_name).await?;

        println!("\nStatistics for workflow '{}'", workflow_name);
        println!("{:-<50}", "");
        println!("Total Executions:      {}", stats.total_executions);
        println!("Success Count:         {}", stats.success_count);
        println!("Failed Count:          {}", stats.failed_count);
        println!("Success Rate:          {:.1}%", stats.success_rate);
        println!("Failure Rate:          {:.1}%", stats.failure_rate);

        if let Some(avg_duration) = stats.avg_duration_seconds {
            println!(
                "Average Duration:      {}",
                format_duration(avg_duration as i64)
            );
        } else {
            println!("Average Duration:      N/A");
        }

        println!("Last 24h Executions:   {}", stats.last_24h_count);

        if let Some(last_exec) = stats.last_execution {
            println!(
                "Last Execution:        {}",
                last_exec.format("%Y-%m-%d %H:%M:%S")
            );
        } else {
            println!("Last Execution:        N/A");
        }

        println!();
        Ok(())
    }

    /// Show task execution logs
    async fn show_logs(
        &self,
        workflow_name: &str,
        execution_id: Option<i64>,
        task_filter: Option<&str>,
    ) -> anyhow::Result<()> {
        let state_manager = StateManager::new(&self.db_path).await?;

        // Get execution ID if not provided
        let exec_id = if let Some(id) = execution_id {
            id
        } else {
            // Get latest execution
            let history = state_manager
                .get_execution_history(workflow_name, 1)
                .await?;
            if history.is_empty() {
                println!(
                    "No execution history found for workflow '{}'",
                    workflow_name
                );
                return Ok(());
            }
            history[0].id
        };

        // Get task executions
        let tasks = state_manager.get_task_executions(exec_id).await?;

        if tasks.is_empty() {
            println!("No task executions found for execution ID {}", exec_id);
            return Ok(());
        }

        // Filter by task name if specified
        let filtered_tasks: Vec<_> = if let Some(task_name) = task_filter {
            tasks
                .into_iter()
                .filter(|t| t.task_name == task_name)
                .collect()
        } else {
            tasks
        };

        if filtered_tasks.is_empty() {
            if let Some(task_name) = task_filter {
                println!(
                    "No tasks found matching '{}' for execution ID {}",
                    task_name, exec_id
                );
            }
            return Ok(());
        }

        println!("\nTask Logs for execution ID: {}", exec_id);
        if let Some(task_name) = task_filter {
            println!("Filtered by task: {}", task_name);
        }
        println!();

        for task in &filtered_tasks {
            println!("{:-<80}", "");
            println!("Task: {}", task.task_name);
            println!("Status: {}", task.status);
            println!("Started: {}", task.started_at.format("%Y-%m-%d %H:%M:%S"));
            if let Some(completed) = task.completed_at {
                println!("Completed: {}", completed.format("%Y-%m-%d %H:%M:%S"));
                let duration = (completed - task.started_at).num_seconds();
                println!("Duration: {}", format_duration(duration));
            }
            if let Some(exit_code) = task.exit_code {
                println!("Exit Code: {}", exit_code);
            }
            println!("Attempt: {} / {}", task.attempt, task.retry_count + 1);

            if let Some(stdout) = &task.stdout {
                if !stdout.is_empty() {
                    println!("\nStdout:");
                    println!("{}", stdout);
                }
            }

            if let Some(stderr) = &task.stderr {
                if !stderr.is_empty() {
                    println!("\nStderr:");
                    println!("{}", stderr);
                }
            }

            println!();
        }

        Ok(())
    }
}

/// Format duration in seconds to human-readable string
fn format_duration(seconds: i64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!(
            "{}h {}m {}s",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
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

    #[test]
    fn test_cli_template_list() {
        let cli = Cli::parse_from(["picoflow", "template"]);
        assert!(matches!(
            cli.command,
            Commands::Template {
                template_type: None,
                output: None,
            }
        ));
    }

    #[test]
    fn test_cli_template_with_type() {
        let cli = Cli::parse_from(["picoflow", "template", "--type", "shell"]);
        if let Commands::Template {
            template_type,
            output,
        } = &cli.command
        {
            assert!(template_type.is_some());
            assert!(output.is_none());
        } else {
            panic!("Expected Template command");
        }
    }

    #[test]
    fn test_cli_template_with_output() {
        let cli = Cli::parse_from(["picoflow", "template", "--type", "full", "-o", "out.yaml"]);
        if let Commands::Template {
            template_type,
            output,
        } = &cli.command
        {
            assert!(template_type.is_some());
            assert_eq!(output.as_ref().unwrap(), &PathBuf::from("out.yaml"));
        } else {
            panic!("Expected Template command");
        }
    }

    #[test]
    fn test_cli_workflow_list() {
        let cli = Cli::parse_from(["picoflow", "workflow", "list"]);
        assert!(matches!(
            cli.command,
            Commands::Workflow {
                command: WorkflowCommands::List
            }
        ));
    }
}
