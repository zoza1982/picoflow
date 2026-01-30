//! Built-in workflow templates for `picoflow template` subcommand.

/// Metadata for a template type.
#[derive(Debug, Clone)]
pub struct TemplateInfo {
    /// Template name (matches the CLI `--type` value).
    pub name: &'static str,
    /// Short description shown in the listing table.
    pub description: &'static str,
}

/// Returns metadata for every available template.
pub fn list_templates() -> Vec<TemplateInfo> {
    vec![
        TemplateInfo {
            name: "minimal",
            description: "Single shell task, no dependencies",
        },
        TemplateInfo {
            name: "shell",
            description: "Multiple shell tasks with dependencies, retry, timeout",
        },
        TemplateInfo {
            name: "ssh",
            description: "SSH remote execution with key auth",
        },
        TemplateInfo {
            name: "http",
            description: "HTTP API calls (GET/POST) with headers",
        },
        TemplateInfo {
            name: "full",
            description: "All executor types combined with DAG dependencies",
        },
    ]
}

/// Returns the YAML content for a given template type.
///
/// The `template_type` must be one of: `minimal`, `shell`, `ssh`, `http`, `full`.
pub fn get_template(template_type: &str) -> Option<&'static str> {
    match template_type {
        "minimal" => Some(TEMPLATE_MINIMAL),
        "shell" => Some(TEMPLATE_SHELL),
        "ssh" => Some(TEMPLATE_SSH),
        "http" => Some(TEMPLATE_HTTP),
        "full" => Some(TEMPLATE_FULL),
        _ => None,
    }
}

/// Bare-minimum workflow: one shell task, no dependencies.
const TEMPLATE_MINIMAL: &str = r#"# PicoFlow Workflow — Minimal Example
# A single shell task with no dependencies.
name: minimal-workflow
description: "A minimal workflow with a single task"

tasks:
  - name: hello
    type: shell
    config:
      command: "echo 'Hello from PicoFlow!'"
"#;

/// Multiple shell tasks demonstrating dependencies, retry, and timeout.
const TEMPLATE_SHELL: &str = r#"# PicoFlow Workflow — Shell Tasks
# Demonstrates dependencies, retry logic, and timeouts.
name: shell-workflow
description: "Shell tasks with dependencies, retry, and timeout"

config:
  max_parallel: 2
  retry_default: 2
  timeout_default: 120

tasks:
  - name: check_disk_space
    type: shell
    config:
      command: "df -h / | tail -1"
    timeout: 30

  - name: create_output_dir
    type: shell
    depends_on: [check_disk_space]
    config:
      command: "mkdir -p /tmp/picoflow-output"

  - name: generate_report
    type: shell
    depends_on: [create_output_dir]
    config:
      command: "date > /tmp/picoflow-output/report.txt && echo 'Report generated'"
    retry: 3
    timeout: 60

  - name: cleanup
    type: shell
    depends_on: [generate_report]
    config:
      command: "rm -rf /tmp/picoflow-output"
    continue_on_failure: true
"#;

/// SSH remote execution with key-based authentication.
const TEMPLATE_SSH: &str = r#"# PicoFlow Workflow — SSH Tasks
# Demonstrates SSH remote execution with key-based auth.
# NOTE: Update host, user, and key_path to match your environment.
#
# Host key verification:
#   By default, picoflow verifies the remote host key against ~/.ssh/known_hosts.
#   If the host is not in known_hosts, the task will fail. To fix:
#     ssh-keyscan -p 22 <host> >> ~/.ssh/known_hosts
#   Or set verify_host_key: false per task (not recommended for production).
name: ssh-workflow
description: "SSH remote execution with key auth"

tasks:
  - name: remote_health_check
    type: ssh
    config:
      host: "192.168.1.100"
      port: 22
      user: "deploy"
      key_path: "~/.ssh/id_ed25519"
      command: "uptime"
      # verify_host_key: true  # default; set to false to skip host key check
    timeout: 30
    retry: 2

  - name: remote_backup
    type: ssh
    depends_on: [remote_health_check]
    config:
      host: "192.168.1.100"
      port: 22
      user: "deploy"
      key_path: "~/.ssh/id_ed25519"
      command: "tar czf /tmp/backup-$(date +%Y%m%d).tar.gz /var/data"
      # verify_host_key: true  # default; set to false to skip host key check
    timeout: 600
    retry: 1
"#;

/// HTTP API calls with GET/POST methods and custom headers.
const TEMPLATE_HTTP: &str = r#"# PicoFlow Workflow — HTTP Tasks
# Demonstrates HTTP API calls with GET and POST methods.
# NOTE: Replace URLs with your actual API endpoints.
name: http-workflow
description: "HTTP API calls (GET/POST) with headers"

tasks:
  - name: health_check
    type: http
    config:
      url: "https://api.example.com/health"
      method: GET
      timeout: 10
    retry: 2

  - name: fetch_data
    type: http
    depends_on: [health_check]
    config:
      url: "https://api.example.com/data"
      method: GET
      headers:
        Authorization: "Bearer ${API_TOKEN}"
        Accept: "application/json"
      timeout: 30

  - name: post_results
    type: http
    depends_on: [fetch_data]
    config:
      url: "https://api.example.com/results"
      method: POST
      headers:
        Content-Type: "application/json"
        Authorization: "Bearer ${API_TOKEN}"
      body: '{"status": "completed", "source": "picoflow"}'
      timeout: 30
    retry: 3
"#;

/// Combined workflow using all executor types with DAG dependencies.
const TEMPLATE_FULL: &str = r#"# PicoFlow Workflow — Full Example
# Combines shell, SSH, and HTTP executors with DAG dependencies.
# NOTE: Update SSH hosts, HTTP URLs, and credentials for your environment.
# SSH host key verification is enabled by default (see ssh template for details).
name: full-workflow
description: "All executor types combined with DAG dependencies"
schedule: "0 2 * * *"  # Run daily at 2 AM (optional)

config:
  max_parallel: 4
  retry_default: 2
  timeout_default: 300

tasks:
  # Step 1: Health checks (run in parallel)
  - name: api_health_check
    type: http
    config:
      url: "https://api.example.com/health"
      method: GET
      timeout: 10
    retry: 2

  - name: server_health_check
    type: ssh
    config:
      host: "192.168.1.100"
      port: 22
      user: "deploy"
      key_path: "~/.ssh/id_ed25519"
      command: "systemctl is-active myservice"
    timeout: 30

  # Step 2: Backup (depends on both health checks)
  - name: backup_database
    type: ssh
    depends_on: [api_health_check, server_health_check]
    config:
      host: "192.168.1.100"
      port: 22
      user: "deploy"
      key_path: "~/.ssh/id_ed25519"
      command: "pg_dump mydb | gzip > /backup/db-$(date +%Y%m%d).sql.gz"
    timeout: 600
    retry: 3

  # Step 3: Verify backup locally
  - name: verify_backup
    type: shell
    depends_on: [backup_database]
    config:
      command: "ssh deploy@192.168.1.100 'test -f /backup/db-*.sql.gz && echo OK'"
    retry: 1

  # Step 4: Notify API
  - name: notify_complete
    type: http
    depends_on: [verify_backup]
    config:
      url: "https://api.example.com/notifications"
      method: POST
      headers:
        Content-Type: "application/json"
        Authorization: "Bearer ${API_TOKEN}"
      body: '{"event": "backup_complete", "workflow": "full-workflow"}'
      timeout: 15

  # Step 5: Cleanup old backups (allowed to fail)
  - name: cleanup_old_backups
    type: ssh
    depends_on: [verify_backup]
    config:
      host: "192.168.1.100"
      port: 22
      user: "deploy"
      key_path: "~/.ssh/id_ed25519"
      command: "find /backup -name '*.sql.gz' -mtime +7 -delete"
    continue_on_failure: true
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_templates_returns_all() {
        let templates = list_templates();
        assert_eq!(templates.len(), 5);
        let names: Vec<&str> = templates.iter().map(|t| t.name).collect();
        assert!(names.contains(&"minimal"));
        assert!(names.contains(&"shell"));
        assert!(names.contains(&"ssh"));
        assert!(names.contains(&"http"));
        assert!(names.contains(&"full"));
    }

    #[test]
    fn test_get_template_returns_content() {
        for name in &["minimal", "shell", "ssh", "http", "full"] {
            let content = get_template(name);
            assert!(content.is_some(), "template '{}' should exist", name);
            assert!(
                content.unwrap().contains("name:"),
                "template '{}' should contain a 'name:' field",
                name
            );
        }
    }

    #[test]
    fn test_get_template_unknown_returns_none() {
        assert!(get_template("nonexistent").is_none());
    }

    #[test]
    fn test_templates_contain_task_definitions() {
        for name in &["minimal", "shell", "ssh", "http", "full"] {
            let content = get_template(name).unwrap();
            assert!(
                content.contains("tasks:"),
                "template '{}' should contain 'tasks:'",
                name
            );
            assert!(
                content.contains("type:"),
                "template '{}' should contain 'type:'",
                name
            );
        }
    }

    #[test]
    fn test_all_templates_parse_as_valid_yaml() {
        use crate::parser::parse_workflow_yaml;
        for name in &["minimal", "shell", "ssh", "http", "full"] {
            let content = get_template(name).unwrap();
            let result = parse_workflow_yaml(content);
            assert!(
                result.is_ok(),
                "template '{}' failed to parse: {:?}",
                name,
                result.err()
            );
        }
    }

    #[test]
    fn test_template_descriptions_non_empty() {
        for info in list_templates() {
            assert!(
                !info.description.is_empty(),
                "template '{}' should have a description",
                info.name
            );
        }
    }
}
