//! YAML parser with validation for workflow configurations

use crate::error::{PicoFlowError, Result};
use crate::models::*;
use std::fs;
use std::path::Path;

/// Parse workflow configuration from YAML file
pub fn parse_workflow_file<P: AsRef<Path>>(path: P) -> Result<WorkflowConfig> {
    let content = fs::read_to_string(path)?;
    parse_workflow_yaml(&content)
}

/// Parse workflow configuration from YAML string
pub fn parse_workflow_yaml(content: &str) -> Result<WorkflowConfig> {
    // Validate YAML size limit
    if content.len() > MAX_YAML_SIZE {
        return Err(PicoFlowError::YamlSizeExceeded(content.len()));
    }

    // Parse YAML
    let mut config: WorkflowConfig = serde_yaml::from_str(content)?;

    // Validate task count
    if config.tasks.len() > MAX_TASK_COUNT {
        return Err(PicoFlowError::TaskCountExceeded {
            count: config.tasks.len(),
            limit: MAX_TASK_COUNT,
        });
    }

    // Validate task names
    for task in &config.tasks {
        validate_task_name(&task.name)?;
    }

    // Validate task dependencies exist
    validate_dependencies(&config)?;

    // Apply global defaults to tasks
    apply_defaults(&mut config);

    Ok(config)
}

/// Validate task name format and length
fn validate_task_name(name: &str) -> Result<()> {
    // Check length
    if name.len() > MAX_TASK_NAME_LEN {
        return Err(PicoFlowError::TaskNameTooLong {
            name: name.to_string(),
            max: MAX_TASK_NAME_LEN,
        });
    }

    // Check format: alphanumeric + underscore + dash only
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(PicoFlowError::InvalidTaskName {
            name: name.to_string(),
        });
    }

    Ok(())
}

/// Validate that all task dependencies reference existing tasks
fn validate_dependencies(config: &WorkflowConfig) -> Result<()> {
    let task_names: std::collections::HashSet<_> = config.tasks.iter().map(|t| &t.name).collect();

    for task in &config.tasks {
        for dep in &task.depends_on {
            if !task_names.contains(dep) {
                return Err(PicoFlowError::MissingDependency {
                    task: task.name.clone(),
                    dependency: dep.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Apply global defaults to task configurations
fn apply_defaults(config: &mut WorkflowConfig) {
    for task in &mut config.tasks {
        // Apply default retry if not specified
        if task.retry.is_none() {
            task.retry = Some(config.config.retry_default);
        }

        // Apply default timeout if not specified
        if task.timeout.is_none() {
            task.timeout = Some(config.config.timeout_default);
        }
    }
}

/// Validate shell executor configuration
pub fn validate_shell_config(config: &ShellConfig) -> Result<()> {
    // Validate command length
    if config.command.len() > MAX_COMMAND_LEN {
        return Err(PicoFlowError::CommandTooLong {
            limit: MAX_COMMAND_LEN,
        });
    }

    // Validate command is absolute path
    if !config.command.starts_with('/') {
        return Err(PicoFlowError::InvalidPath(
            "Command must be an absolute path".to_string(),
        ));
    }

    // Validate argument count
    if config.args.len() > MAX_ARG_COUNT {
        return Err(PicoFlowError::ArgCountExceeded {
            count: config.args.len(),
            limit: MAX_ARG_COUNT,
        });
    }

    // Validate argument lengths
    for arg in &config.args {
        if arg.len() > MAX_ARG_LEN {
            return Err(PicoFlowError::ArgTooLong { limit: MAX_ARG_LEN });
        }
    }

    // Validate working directory if specified
    if let Some(workdir) = &config.workdir {
        validate_path(workdir)?;
    }

    Ok(())
}

/// Validate path (no traversal, absolute path)
pub fn validate_path(path: &str) -> Result<()> {
    // Check for path traversal
    if path.contains("..") {
        return Err(PicoFlowError::PathTraversal(path.to_string()));
    }

    // Require absolute path
    if !path.starts_with('/') {
        return Err(PicoFlowError::InvalidPath(
            "Path must be absolute".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_workflow() {
        let yaml = r#"
name: test-workflow
description: "Test workflow"
tasks:
  - name: task1
    type: shell
    config:
      command: "/bin/echo"
      args: ["hello"]
"#;
        let config = parse_workflow_yaml(yaml).unwrap();
        assert_eq!(config.name, "test-workflow");
        assert_eq!(config.tasks.len(), 1);
        assert_eq!(config.tasks[0].name, "task1");
    }

    #[test]
    fn test_yaml_size_limit() {
        let large_yaml = "name: test\ntasks:\n".to_string() + &"  - name: x\n".repeat(100_000);
        let result = parse_workflow_yaml(&large_yaml);
        assert!(matches!(result, Err(PicoFlowError::YamlSizeExceeded(_))));
    }

    #[test]
    fn test_task_count_limit() {
        let mut yaml = "name: test\ntasks:\n".to_string();
        for i in 0..1001 {
            yaml.push_str(&format!(
                "  - name: task{}\n    type: shell\n    config:\n      command: /bin/true\n",
                i
            ));
        }
        let result = parse_workflow_yaml(&yaml);
        assert!(matches!(
            result,
            Err(PicoFlowError::TaskCountExceeded { .. })
        ));
    }

    #[test]
    fn test_task_name_validation() {
        // Valid names
        assert!(validate_task_name("task1").is_ok());
        assert!(validate_task_name("task_1").is_ok());
        assert!(validate_task_name("task-1").is_ok());
        assert!(validate_task_name("TASK_1").is_ok());

        // Invalid names
        assert!(matches!(
            validate_task_name("task 1"),
            Err(PicoFlowError::InvalidTaskName { .. })
        ));
        assert!(matches!(
            validate_task_name("task@1"),
            Err(PicoFlowError::InvalidTaskName { .. })
        ));
        assert!(matches!(
            validate_task_name("a".repeat(65).as_str()),
            Err(PicoFlowError::TaskNameTooLong { .. })
        ));
    }

    #[test]
    fn test_missing_dependency() {
        let yaml = r#"
name: test
tasks:
  - name: task1
    type: shell
    depends_on: [nonexistent]
    config:
      command: "/bin/true"
"#;
        let result = parse_workflow_yaml(yaml);
        assert!(matches!(
            result,
            Err(PicoFlowError::MissingDependency { .. })
        ));
    }

    #[test]
    fn test_apply_defaults() {
        let yaml = r#"
name: test
config:
  retry_default: 5
  timeout_default: 600
tasks:
  - name: task1
    type: shell
    config:
      command: "/bin/true"
"#;
        let config = parse_workflow_yaml(yaml).unwrap();
        assert_eq!(config.tasks[0].retry, Some(5));
        assert_eq!(config.tasks[0].timeout, Some(600));
    }

    #[test]
    fn test_validate_shell_config() {
        let config = ShellConfig {
            command: "/bin/echo".to_string(),
            args: vec!["hello".to_string()],
            workdir: Some("/tmp".to_string()),
            env: None,
        };
        assert!(validate_shell_config(&config).is_ok());

        // Invalid: relative path
        let config = ShellConfig {
            command: "echo".to_string(),
            args: vec![],
            workdir: None,
            env: None,
        };
        assert!(matches!(
            validate_shell_config(&config),
            Err(PicoFlowError::InvalidPath(_))
        ));

        // Invalid: path traversal in workdir
        let config = ShellConfig {
            command: "/bin/echo".to_string(),
            args: vec![],
            workdir: Some("/tmp/../etc".to_string()),
            env: None,
        };
        assert!(matches!(
            validate_shell_config(&config),
            Err(PicoFlowError::PathTraversal(_))
        ));
    }

    #[test]
    fn test_validate_path() {
        assert!(validate_path("/tmp").is_ok());
        assert!(validate_path("/usr/bin/ls").is_ok());

        // Path traversal
        assert!(matches!(
            validate_path("/tmp/../etc"),
            Err(PicoFlowError::PathTraversal(_))
        ));

        // Relative path
        assert!(matches!(
            validate_path("tmp"),
            Err(PicoFlowError::InvalidPath(_))
        ));
    }
}
