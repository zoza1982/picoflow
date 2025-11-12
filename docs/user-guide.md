# PicoFlow User Guide

**Version:** 1.0.0
**Last Updated:** November 12, 2025

---

## Table of Contents

1. [Introduction](#introduction)
2. [Installation](#installation)
3. [Quick Start Tutorial](#quick-start-tutorial)
4. [Workflow YAML Syntax](#workflow-yaml-syntax)
5. [Task Types](#task-types)
6. [Configuration Options](#configuration-options)
7. [Scheduling with Cron](#scheduling-with-cron)
8. [CLI Commands Reference](#cli-commands-reference)
9. [Daemon Mode](#daemon-mode)
10. [Execution History and Monitoring](#execution-history-and-monitoring)
11. [Prometheus Metrics](#prometheus-metrics)
12. [Best Practices for Edge Devices](#best-practices-for-edge-devices)
13. [Performance Tuning](#performance-tuning)
14. [Troubleshooting](#troubleshooting)

---

## Introduction

PicoFlow is a lightweight DAG (Directed Acyclic Graph) workflow orchestrator designed specifically for resource-constrained edge devices like the Raspberry Pi Zero 2 W. Written in Rust, PicoFlow provides enterprise-grade workflow orchestration with minimal memory footprint.

### Key Features

- **Minimal Resource Footprint**: <20MB RAM idle, <50MB with 10 parallel tasks
- **DAG Support**: Define complex workflows with task dependencies
- **Multiple Executors**: Shell commands, SSH remote execution, HTTP requests
- **Scheduling**: Cron-based scheduling with daemon mode
- **Retry Logic**: Exponential backoff with configurable retry policies
- **Observability**: Structured JSON logging, Prometheus metrics
- **Edge-Ready**: Tested on Raspberry Pi Zero 2 W (512MB RAM)

### Why PicoFlow?

Traditional workflow orchestrators like Apache Airflow require 2GB+ RAM and are designed for datacenter environments. PicoFlow brings the same capabilities to edge devices with a 100x smaller footprint:

| Feature | PicoFlow | Airflow | Luigi | cron |
|---------|----------|---------|-------|------|
| Memory (idle) | <20MB | 2GB+ | 200MB | N/A |
| Binary size | 3.0MB | N/A | N/A | N/A |
| DAG support | Yes | Yes | Yes | No |
| Edge device ready | Yes | No | No | Yes |
| Retry logic | Yes | Yes | Yes | No |

---

## Installation

### From Pre-Built Binaries

Download the latest release for your platform:

```bash
# Linux ARM32 (Raspberry Pi Zero 2 W, Pi 3)
curl -L https://github.com/zoza1982/picoflow/releases/latest/download/picoflow-linux-armv7 -o picoflow
chmod +x picoflow
sudo mv picoflow /usr/local/bin/

# Linux ARM64 (Raspberry Pi 4/5, modern SBCs)
curl -L https://github.com/zoza1982/picoflow/releases/latest/download/picoflow-linux-aarch64 -o picoflow
chmod +x picoflow
sudo mv picoflow /usr/local/bin/

# Linux x86_64
curl -L https://github.com/zoza1982/picoflow/releases/latest/download/picoflow-linux-x86_64 -o picoflow
chmod +x picoflow
sudo mv picoflow /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/zoza1982/picoflow/releases/latest/download/picoflow-macos-x86_64 -o picoflow
chmod +x picoflow
sudo mv picoflow /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/zoza1982/picoflow/releases/latest/download/picoflow-macos-aarch64 -o picoflow
chmod +x picoflow
sudo mv picoflow /usr/local/bin/
```

Verify installation:

```bash
picoflow --version
```

### From Source

Prerequisites:
- Rust 1.70 or newer
- Git

```bash
# Clone the repository
git clone https://github.com/zoza1982/picoflow.git
cd picoflow

# Build release binary (optimized for size)
cargo build --release

# Install to /usr/local/bin
sudo cp target/release/picoflow /usr/local/bin/

# Or install to Cargo's bin directory
cargo install --path .
```

### Cross-Compilation for Raspberry Pi

If you're developing on a different platform and want to build for Raspberry Pi:

```bash
# Install cross-compilation tool
cargo install cross

# Build for ARM32 (Pi Zero 2 W, Pi 3)
cross build --release --target armv7-unknown-linux-gnueabihf

# Build for ARM64 (Pi 4/5)
cross build --release --target aarch64-unknown-linux-gnu

# Binary will be in target/<arch>/release/picoflow
```

### Docker Installation (Optional)

```bash
# Run PicoFlow in a container
docker run -v $(pwd)/workflows:/workflows \
  -v $(pwd)/data:/data \
  ghcr.io/zoza1982/picoflow:latest \
  run /workflows/example.yaml
```

---

## Quick Start Tutorial

This 5-minute tutorial will get you running your first workflow.

### Step 1: Create Your First Workflow

Create a file named `hello-world.yaml`:

```yaml
name: hello-world
description: "My first PicoFlow workflow"

tasks:
  - name: say_hello
    type: shell
    config:
      command: "echo"
      args: ["Hello from PicoFlow!"]

  - name: show_date
    type: shell
    depends_on: [say_hello]
    config:
      command: "date"
```

### Step 2: Validate the Workflow

Before running, validate that the workflow is correctly formed:

```bash
picoflow validate hello-world.yaml
```

You should see:
```
Workflow 'hello-world' is valid
Tasks: 2
Dependencies validated: No cycles detected
```

### Step 3: Run the Workflow

Execute the workflow:

```bash
picoflow run hello-world.yaml
```

Output:
```
Starting workflow execution: hello-world
Task 'say_hello' started
Task 'say_hello' completed successfully (exit code: 0)
Task 'show_date' started
Task 'show_date' completed successfully (exit code: 0)
Workflow 'hello-world' completed successfully in 0.5s
```

### Step 4: Check Execution History

View the execution history:

```bash
picoflow history --workflow hello-world
```

### Step 5: Add Dependencies

Let's create a more complex workflow with parallel execution. Create `parallel-example.yaml`:

```yaml
name: parallel-example
description: "Demonstrates parallel task execution"

config:
  max_parallel: 4

tasks:
  # These three tasks have no dependencies and will run in parallel
  - name: task_a
    type: shell
    config:
      command: "echo"
      args: ["Task A running"]

  - name: task_b
    type: shell
    config:
      command: "echo"
      args: ["Task B running"]

  - name: task_c
    type: shell
    config:
      command: "echo"
      args: ["Task C running"]

  # This task depends on all three and runs last
  - name: summary
    type: shell
    depends_on: [task_a, task_b, task_c]
    config:
      command: "echo"
      args: ["All parallel tasks completed!"]
```

Run it:

```bash
picoflow run parallel-example.yaml
```

Notice that tasks A, B, and C run simultaneously, and the summary task waits for all three to complete.

Congratulations! You've just orchestrated your first DAG workflow with PicoFlow.

---

## Workflow YAML Syntax

A workflow is defined in YAML format with the following top-level structure:

```yaml
name: workflow-name              # Required: Unique workflow identifier
description: "Description"       # Optional: Human-readable description
schedule: "0 0 2 * * *"         # Optional: Cron schedule (6-field format)

config:                          # Optional: Global workflow configuration
  max_parallel: 4               # Max tasks running simultaneously
  retry_default: 3              # Default retry count for tasks
  timeout_default: 300          # Default timeout in seconds

tasks:                           # Required: List of tasks
  - name: task1                 # Required: Unique task name
    type: shell                 # Required: Executor type
    depends_on: []              # Optional: Task dependencies
    config: {}                  # Required: Task-specific configuration
    retry: 3                    # Optional: Override retry count
    timeout: 600                # Optional: Override timeout
    continue_on_failure: false  # Optional: Continue workflow if this fails
```

### Field Reference

#### Top-Level Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Unique workflow identifier (alphanumeric, hyphens, underscores) |
| `description` | string | No | Human-readable workflow description |
| `schedule` | string | No | Cron expression for scheduled execution (see [Scheduling](#scheduling-with-cron)) |
| `config` | object | No | Global workflow configuration |
| `tasks` | array | Yes | List of task definitions (at least 1 task required) |

#### Config Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_parallel` | integer | 10 | Maximum number of tasks running simultaneously |
| `retry_default` | integer | 0 | Default retry count for all tasks |
| `timeout_default` | integer | 300 | Default timeout in seconds for all tasks |

#### Task Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Unique task identifier within workflow |
| `type` | string | Yes | Executor type: `shell`, `ssh`, or `http` |
| `depends_on` | array | No | List of task names this task depends on |
| `config` | object | Yes | Task-specific configuration (varies by executor) |
| `retry` | integer | No | Number of retry attempts (overrides `retry_default`) |
| `timeout` | integer | No | Task timeout in seconds (overrides `timeout_default`) |
| `continue_on_failure` | boolean | No | If true, workflow continues even if this task fails |

### DAG Rules

1. **No Cycles**: Task dependencies must form a directed acyclic graph (DAG). Circular dependencies are rejected during validation.

2. **Valid Dependencies**: All tasks referenced in `depends_on` must exist in the workflow.

3. **Unique Names**: Task names must be unique within a workflow.

4. **Execution Order**: Tasks are executed in topological order based on dependencies.

### Example: Complex DAG

```yaml
name: data-pipeline
description: "ETL pipeline with multiple stages"

config:
  max_parallel: 3
  retry_default: 2
  timeout_default: 600

tasks:
  # Stage 1: Parallel data extraction
  - name: extract_db1
    type: ssh
    config:
      host: "db1.example.com"
      user: "etl"
      command: "pg_dump production > /tmp/db1.sql"
    timeout: 1800

  - name: extract_db2
    type: ssh
    config:
      host: "db2.example.com"
      user: "etl"
      command: "pg_dump production > /tmp/db2.sql"
    timeout: 1800

  - name: extract_api
    type: http
    config:
      url: "https://api.example.com/export"
      method: GET
      timeout: 300
    retry: 3

  # Stage 2: Transform data (depends on extraction)
  - name: transform_db1
    type: shell
    depends_on: [extract_db1]
    config:
      command: "/opt/etl/transform.sh"
      args: ["/tmp/db1.sql", "/tmp/db1_clean.sql"]

  - name: transform_db2
    type: shell
    depends_on: [extract_db2]
    config:
      command: "/opt/etl/transform.sh"
      args: ["/tmp/db2.sql", "/tmp/db2_clean.sql"]

  # Stage 3: Load data (depends on all transforms)
  - name: load_warehouse
    type: ssh
    depends_on: [transform_db1, transform_db2]
    config:
      host: "warehouse.example.com"
      user: "etl"
      command: "psql warehouse < /tmp/merged.sql"
    timeout: 3600
    retry: 1

  # Stage 4: Cleanup (depends on load, continues even if it fails)
  - name: cleanup
    type: shell
    depends_on: [load_warehouse]
    config:
      command: "rm"
      args: ["-f", "/tmp/db1.sql", "/tmp/db2.sql", "/tmp/*.sql"]
    continue_on_failure: true
```

---

## Task Types

PicoFlow supports three types of task executors: Shell, SSH, and HTTP.

### Shell Executor

Execute commands on the local system where PicoFlow is running.

**Configuration:**

```yaml
type: shell
config:
  command: string        # Required: Command to execute (absolute path recommended)
  args: [string]        # Optional: Command arguments
  working_dir: string   # Optional: Working directory (default: picoflow's cwd)
  env: {}              # Optional: Environment variables
```

**Example: Basic Command**

```yaml
- name: backup_logs
  type: shell
  config:
    command: "/usr/bin/tar"
    args: ["-czf", "/backup/logs.tar.gz", "/var/log"]
```

**Example: With Environment Variables**

```yaml
- name: database_backup
  type: shell
  config:
    command: "/opt/backup.sh"
    args: ["daily"]
    env:
      DB_HOST: "localhost"
      DB_NAME: "production"
      BACKUP_DIR: "/mnt/backup"
```

**Example: Script Execution**

```yaml
- name: run_python_script
  type: shell
  config:
    command: "/usr/bin/python3"
    args: ["/opt/scripts/process_data.py", "--mode", "production"]
    working_dir: "/opt/scripts"
```

**Security Note:** Always use absolute paths for commands to prevent PATH injection attacks. Never construct commands from user input.

### SSH Executor

Execute commands on remote systems via SSH.

**Configuration:**

```yaml
type: ssh
config:
  host: string          # Required: Remote hostname or IP
  port: integer         # Optional: SSH port (default: 22)
  user: string          # Required: SSH username
  command: string       # Required: Command to execute remotely
  key_path: string      # Optional: Path to SSH private key (default: ~/.ssh/id_rsa)
  timeout: integer      # Optional: Connection timeout in seconds (default: 30)
```

**Example: Remote Backup**

```yaml
- name: remote_backup
  type: ssh
  config:
    host: "backup.example.com"
    user: "backup"
    command: "pg_dump mydb | gzip > /backup/db-$(date +%Y%m%d).sql.gz"
    key_path: "/home/picoflow/.ssh/backup_key"
  timeout: 1800
  retry: 2
```

**Example: Multi-Server Deployment**

```yaml
tasks:
  - name: deploy_web1
    type: ssh
    config:
      host: "web1.example.com"
      user: "deploy"
      command: "/opt/deploy.sh production"

  - name: deploy_web2
    type: ssh
    config:
      host: "web2.example.com"
      user: "deploy"
      command: "/opt/deploy.sh production"

  - name: verify_deployment
    type: http
    depends_on: [deploy_web1, deploy_web2]
    config:
      url: "https://example.com/health"
      method: GET
```

**Security Requirements:**

1. **Key-Based Authentication Only**: Password authentication is not supported.
2. **SSH Keys**: Place private keys in a secure location with `600` permissions.
3. **Host Key Verification**: First connection requires adding host to known_hosts.
4. **User Isolation**: Use dedicated service accounts with minimal privileges.

**Setup SSH Keys:**

```bash
# Generate a key pair for PicoFlow
ssh-keygen -t ed25519 -f ~/.ssh/picoflow_key -C "picoflow@$(hostname)"

# Copy public key to remote host
ssh-copy-id -i ~/.ssh/picoflow_key.pub user@remote-host

# Test connection
ssh -i ~/.ssh/picoflow_key user@remote-host "echo Connection successful"

# Use in workflow
config:
  key_path: "/home/picoflow/.ssh/picoflow_key"
```

### HTTP Executor

Make HTTP/HTTPS requests to REST APIs.

**Configuration:**

```yaml
type: http
config:
  url: string           # Required: Full URL (http:// or https://)
  method: string        # Required: HTTP method (GET, POST, PUT, DELETE, PATCH)
  headers: {}          # Optional: HTTP headers as key-value pairs
  body: {}             # Optional: Request body (JSON object)
  timeout: integer     # Optional: Request timeout in seconds (default: 30)
  expected_status: [int] # Optional: List of success status codes (default: [200-299])
```

**Example: GET Request**

```yaml
- name: health_check
  type: http
  config:
    url: "https://api.example.com/health"
    method: GET
    timeout: 10
  retry: 2
```

**Example: POST with JSON Body**

```yaml
- name: create_resource
  type: http
  config:
    url: "https://api.example.com/resources"
    method: POST
    headers:
      Content-Type: "application/json"
      Authorization: "Bearer ${API_TOKEN}"
    body:
      name: "New Resource"
      type: "server"
      region: "us-east-1"
    timeout: 30
```

**Example: PUT Request**

```yaml
- name: update_status
  type: http
  config:
    url: "https://api.example.com/deployments/12345"
    method: PUT
    headers:
      Content-Type: "application/json"
    body:
      status: "completed"
      timestamp: "2025-11-12T10:00:00Z"
```

**Example: DELETE Request**

```yaml
- name: cleanup_resources
  type: http
  config:
    url: "https://api.example.com/tmp-resources"
    method: DELETE
    expected_status: [200, 204, 404]
  continue_on_failure: true
```

**Example: API Workflow**

```yaml
name: api-deployment
description: "Deploy and verify via API"

tasks:
  - name: trigger_deployment
    type: http
    config:
      url: "https://deploy.example.com/api/deployments"
      method: POST
      headers:
        Authorization: "Bearer ${DEPLOY_TOKEN}"
      body:
        environment: "production"
        version: "v1.2.3"
    retry: 1

  - name: poll_deployment_status
    type: http
    depends_on: [trigger_deployment]
    config:
      url: "https://deploy.example.com/api/deployments/latest"
      method: GET
      headers:
        Authorization: "Bearer ${DEPLOY_TOKEN}"
    retry: 5
    timeout: 300

  - name: verify_health
    type: http
    depends_on: [poll_deployment_status]
    config:
      url: "https://app.example.com/health"
      method: GET
      expected_status: [200]
    retry: 3
```

**Success Criteria:**

- HTTP status code in range 200-299 (or matching `expected_status`)
- No connection timeout
- Valid response received

**Authentication:**

Use environment variables for secrets:

```yaml
headers:
  Authorization: "Bearer ${API_TOKEN}"

# Set environment variable before running:
# export API_TOKEN="your-secret-token"
# picoflow run workflow.yaml
```

---

## Configuration Options

### Global Configuration

Create a `picoflow.toml` configuration file in your workflow directory or `~/.config/picoflow/config.toml`:

```toml
# Database path for execution history
db_path = "/var/lib/picoflow/picoflow.db"

# Log configuration
log_level = "info"  # error, warn, info, debug, trace
log_format = "json"  # json or pretty

# Metrics configuration
[metrics]
enabled = true
port = 9090

# Default retry configuration
[retry]
default_count = 3
max_backoff_seconds = 300
backoff_multiplier = 2.0

# Default timeout configuration
[timeout]
default_seconds = 300
max_seconds = 3600

# Parallel execution limits
[execution]
max_parallel = 10
```

### Command-Line Options

Override configuration via CLI flags:

```bash
picoflow --log-level debug \
         --log-format pretty \
         --db-path /tmp/picoflow.db \
         run workflow.yaml
```

### Environment Variables

PicoFlow recognizes these environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `PICOFLOW_DB_PATH` | Database file path | `picoflow.db` |
| `PICOFLOW_LOG_LEVEL` | Logging level | `info` |
| `PICOFLOW_LOG_FORMAT` | Log output format | `json` |
| `PICOFLOW_METRICS_PORT` | Prometheus metrics port | `9090` |
| `RUST_LOG` | Rust tracing filter | Inherited from log_level |

---

## Scheduling with Cron

PicoFlow supports cron-based scheduling for automatic workflow execution.

### Cron Syntax

PicoFlow uses **6-field cron expressions**:

```
┌─────────── second (0-59)
│ ┌───────── minute (0-59)
│ │ ┌─────── hour (0-23)
│ │ │ ┌───── day of month (1-31)
│ │ │ │ ┌─── month (1-12)
│ │ │ │ │ ┌─ day of week (0-6, Sunday = 0)
│ │ │ │ │ │
* * * * * *
```

### Common Patterns

```yaml
# Every day at 2:00 AM
schedule: "0 0 2 * * *"

# Every hour at minute 0
schedule: "0 0 * * * *"

# Every 5 minutes
schedule: "0 */5 * * * *"

# Every Monday at 9:00 AM
schedule: "0 0 9 * * 1"

# First day of every month at midnight
schedule: "0 0 0 1 * *"

# Every weekday at 6:00 PM
schedule: "0 0 18 * * 1-5"

# Every 30 seconds
schedule: "*/30 * * * * *"

# Every Sunday at 3:30 AM
schedule: "0 30 3 * * 0"
```

### Special Expressions

```yaml
# Every second (use with caution!)
schedule: "* * * * * *"

# Business hours: Every hour from 9 AM to 5 PM on weekdays
schedule: "0 0 9-17 * * 1-5"

# Quarterly: First day of Jan, Apr, Jul, Oct at midnight
schedule: "0 0 0 1 1,4,7,10 *"
```

### Example: Scheduled Backup

```yaml
name: nightly-backup
description: "Automated nightly backup"
schedule: "0 0 2 * * *"  # 2:00 AM every day

tasks:
  - name: backup_database
    type: ssh
    config:
      host: "db.example.com"
      user: "backup"
      command: "pg_dump production | gzip > /backup/db-$(date +%Y%m%d).sql.gz"
    timeout: 3600

  - name: backup_files
    type: shell
    config:
      command: "rsync"
      args: ["-av", "/data", "backup:/mnt/backup/"]
    timeout: 7200

  - name: verify_backup
    type: shell
    depends_on: [backup_database, backup_files]
    config:
      command: "/opt/scripts/verify_backup.sh"
```

Run as daemon to enable scheduling:

```bash
picoflow daemon start nightly-backup.yaml
```

---

## CLI Commands Reference

### picoflow run

Execute a workflow once.

```bash
picoflow run [OPTIONS] <WORKFLOW_FILE>
```

**Options:**
- `--log-level <LEVEL>`: Set log level (error, warn, info, debug, trace)
- `--log-format <FORMAT>`: Set log format (json, pretty)
- `--db-path <PATH>`: Database file path

**Examples:**

```bash
# Run a workflow
picoflow run backup.yaml

# Run with debug logging
picoflow run --log-level debug backup.yaml

# Run with pretty output
picoflow run --log-format pretty backup.yaml
```

### picoflow validate

Validate workflow YAML and DAG structure.

```bash
picoflow validate <WORKFLOW_FILE>
```

**Output:**
```
Workflow 'backup-workflow' is valid
Tasks: 5
Dependencies validated: No cycles detected
Max parallel levels: 3
Estimated execution time: 15-20 minutes
```

**Examples:**

```bash
# Validate single workflow
picoflow validate backup.yaml

# Validate all workflows in directory
for f in workflows/*.yaml; do
  picoflow validate "$f"
done
```

### picoflow status

Show current workflow execution status.

```bash
picoflow status [OPTIONS]
```

**Options:**
- `--workflow <NAME>`: Filter by workflow name
- `--running-only`: Show only running workflows

**Examples:**

```bash
# Show all workflow statuses
picoflow status

# Show specific workflow
picoflow status --workflow backup-workflow

# Show only running workflows
picoflow status --running-only
```

**Output:**
```
Workflow: backup-workflow
Status: Running
Started: 2025-11-12 02:00:15
Elapsed: 5m 23s

Tasks:
  ✓ health_check       (completed in 2s)
  ✓ backup_database    (completed in 4m 12s)
  ⟳ verify_backup      (running for 1m 9s)
  ○ cleanup_old_backups (pending)
```

### picoflow workflow list

List all workflows with execution statistics.

```bash
picoflow workflow list [OPTIONS]
```

**Options:**
- `--all`: Show all workflows including inactive
- `--format <FORMAT>`: Output format (table, json)

**Examples:**

```bash
# List all workflows
picoflow workflow list

# List with JSON output
picoflow workflow list --format json
```

**Output:**
```
WORKFLOW          TYPE       SCHEDULE        LAST RUN             STATUS   SUCCESS  FAILED
backup-workflow   Cron       0 0 2 * * *     2025-11-12 02:00    Success      145       2
health-check      Cron       0 */5 * * * *   2025-11-12 10:15    Success     3201       0
deploy-app        On-Demand  -               2025-11-11 14:30    Success       12       1
```

### picoflow daemon

Manage daemon mode for scheduled workflows.

#### daemon start

Start daemon with workflow(s).

```bash
picoflow daemon start <WORKFLOW_FILE> [WORKFLOW_FILE...]
```

**Examples:**

```bash
# Start with single workflow
picoflow daemon start backup.yaml

# Start with multiple workflows
picoflow daemon start backup.yaml monitoring.yaml

# Start with custom config
picoflow --db-path /data/picoflow.db daemon start workflows/*.yaml
```

The daemon will:
1. Load and validate all workflows
2. Schedule cron jobs for workflows with `schedule` field
3. Run in background
4. Write PID to `picoflow.pid`

#### daemon stop

Stop running daemon.

```bash
picoflow daemon stop
```

Gracefully shuts down the daemon:
1. Stops accepting new workflow executions
2. Waits for running tasks to complete (with timeout)
3. Saves state to database
4. Removes PID file

#### daemon status

Check daemon status.

```bash
picoflow daemon status
```

**Output:**
```
Daemon Status: Running
PID: 12345
Uptime: 2d 5h 23m
Workflows: 3 loaded
  - backup-workflow (cron: 0 0 2 * * *)
  - health-check (cron: 0 */5 * * * *)
  - monitoring (cron: 0 */15 * * * *)
Next scheduled run: health-check at 2025-11-12 10:20:00
Memory usage: 18.5 MB
```

### picoflow history

Query workflow execution history.

```bash
picoflow history [OPTIONS]
```

**Options:**
- `--workflow <NAME>`: Filter by workflow name
- `--status <STATUS>`: Filter by status (success, failed, running)
- `--limit <N>`: Limit number of results (default: 10)
- `--since <DATE>`: Show executions since date (YYYY-MM-DD)
- `--format <FORMAT>`: Output format (table, json)

**Examples:**

```bash
# Show last 10 executions
picoflow history

# Show specific workflow history
picoflow history --workflow backup-workflow --limit 20

# Show failed executions in last 7 days
picoflow history --status failed --since 2025-11-05

# Export to JSON
picoflow history --workflow backup-workflow --format json > history.json
```

**Output:**
```
WORKFLOW          STARTED              DURATION  STATUS   TASKS  RESULT
backup-workflow   2025-11-12 02:00:15  6m 23s    Success  4/4    All tasks completed
backup-workflow   2025-11-11 02:00:10  6m 18s    Success  4/4    All tasks completed
backup-workflow   2025-11-10 02:00:05  6m 45s    Success  4/4    All tasks completed
health-check      2025-11-12 10:15:00  0m 2s     Success  1/1    All tasks completed
```

### picoflow stats

Show workflow execution statistics.

```bash
picoflow stats [OPTIONS]
```

**Options:**
- `--workflow <NAME>`: Filter by workflow name
- `--period <DAYS>`: Time period in days (default: 30)

**Examples:**

```bash
# Show stats for all workflows
picoflow stats

# Show specific workflow stats
picoflow stats --workflow backup-workflow

# Show stats for last 7 days
picoflow stats --period 7
```

**Output:**
```
Workflow: backup-workflow
Period: Last 30 days

Executions: 30
Success: 28 (93.3%)
Failed: 2 (6.7%)

Average duration: 6m 25s
Min duration: 5m 12s
Max duration: 8m 45s

Task statistics:
  health_check:       100% success, avg 2s
  backup_database:    96.7% success, avg 4m 15s
  verify_backup:      100% success, avg 1m 5s
  cleanup:            100% success, avg 3s

Failure breakdown:
  2025-11-08: backup_database failed (timeout)
  2025-11-03: backup_database failed (connection refused)
```

### picoflow logs

View task execution logs.

```bash
picoflow logs [OPTIONS]
```

**Options:**
- `--workflow <NAME>`: Workflow name (required)
- `--task <NAME>`: Task name (optional, shows all if omitted)
- `--execution-id <ID>`: Specific execution ID
- `--tail <N>`: Show last N lines (default: 100)
- `--follow`: Follow log output (like `tail -f`)

**Examples:**

```bash
# Show logs for latest execution
picoflow logs --workflow backup-workflow

# Show logs for specific task
picoflow logs --workflow backup-workflow --task backup_database

# Follow logs in real-time
picoflow logs --workflow backup-workflow --follow

# Show last 50 lines
picoflow logs --workflow backup-workflow --tail 50

# Show logs for specific execution
picoflow logs --workflow backup-workflow --execution-id abc123
```

**Output:**
```
[2025-11-12T02:00:15Z] INFO Starting task: backup_database
[2025-11-12T02:00:16Z] INFO Connecting to db.example.com:22
[2025-11-12T02:00:17Z] INFO Connected successfully
[2025-11-12T02:00:18Z] INFO Executing: pg_dump production | gzip > /backup/db-20251112.sql.gz
[2025-11-12T02:04:30Z] INFO Command completed with exit code: 0
[2025-11-12T02:04:30Z] INFO Task completed successfully
```

---

## Daemon Mode

Daemon mode allows PicoFlow to run continuously in the background, executing scheduled workflows automatically.

### Starting the Daemon

```bash
# Start with workflows
picoflow daemon start backup.yaml monitoring.yaml

# Start in foreground (for debugging)
picoflow --log-format pretty daemon start backup.yaml

# Start as systemd service (recommended for production)
sudo systemctl start picoflow
```

### Systemd Integration

Create `/etc/systemd/system/picoflow.service`:

```ini
[Unit]
Description=PicoFlow Workflow Orchestrator
After=network.target

[Service]
Type=simple
User=picoflow
Group=picoflow
WorkingDirectory=/opt/picoflow
ExecStart=/usr/local/bin/picoflow daemon start /opt/picoflow/workflows/*.yaml
Restart=on-failure
RestartSec=10s

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/picoflow /var/lib/picoflow

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable picoflow
sudo systemctl start picoflow
sudo systemctl status picoflow
```

View logs:

```bash
sudo journalctl -u picoflow -f
```

### Daemon Behavior

1. **Scheduling**: Workflows with `schedule` field are automatically scheduled
2. **State Persistence**: Execution state saved to SQLite database
3. **Crash Recovery**: Resumes from last saved state on restart
4. **Signal Handling**:
   - `SIGTERM`: Graceful shutdown (waits for tasks to complete)
   - `SIGINT` (Ctrl+C): Graceful shutdown
   - `SIGKILL`: Immediate termination (not recommended)

### Managing the Daemon

```bash
# Check status
picoflow daemon status

# Stop daemon
picoflow daemon stop

# Restart daemon
picoflow daemon stop && picoflow daemon start workflows/*.yaml

# View daemon logs
tail -f /var/log/picoflow.log

# With systemd
sudo systemctl restart picoflow
```

---

## Execution History and Monitoring

PicoFlow maintains a persistent execution history in SQLite for monitoring and debugging.

### Database Location

Default: `picoflow.db` in current directory

Override with:
```bash
picoflow --db-path /var/lib/picoflow/history.db run workflow.yaml
```

### Querying History

**View recent executions:**

```bash
picoflow history --limit 20
```

**Filter by workflow:**

```bash
picoflow history --workflow backup-workflow
```

**Filter by status:**

```bash
picoflow history --status failed
```

**Date range queries:**

```bash
# Executions in last 7 days
picoflow history --since $(date -d '7 days ago' +%Y-%m-%d)

# Specific date range (requires SQL query)
sqlite3 picoflow.db "
  SELECT workflow_name, started_at, status, duration_seconds
  FROM workflow_executions
  WHERE started_at BETWEEN '2025-11-01' AND '2025-11-30'
  ORDER BY started_at DESC;
"
```

### Database Schema

Key tables:

```sql
-- Workflow executions
CREATE TABLE workflow_executions (
    id TEXT PRIMARY KEY,
    workflow_name TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL,
    duration_seconds REAL
);

-- Task executions
CREATE TABLE task_executions (
    id TEXT PRIMARY KEY,
    workflow_execution_id TEXT NOT NULL,
    task_name TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    exit_code INTEGER,
    stdout TEXT,
    stderr TEXT,
    attempt INTEGER NOT NULL,
    FOREIGN KEY (workflow_execution_id) REFERENCES workflow_executions(id)
);
```

### Advanced Queries

**Task success rate:**

```sql
sqlite3 picoflow.db "
  SELECT
    task_name,
    COUNT(*) as total,
    SUM(CASE WHEN status = 'Success' THEN 1 ELSE 0 END) as success,
    ROUND(100.0 * SUM(CASE WHEN status = 'Success' THEN 1 ELSE 0 END) / COUNT(*), 2) as success_rate
  FROM task_executions
  WHERE started_at > datetime('now', '-30 days')
  GROUP BY task_name
  ORDER BY success_rate ASC;
"
```

**Average execution time by workflow:**

```sql
sqlite3 picoflow.db "
  SELECT
    workflow_name,
    COUNT(*) as executions,
    ROUND(AVG(duration_seconds), 2) as avg_duration_sec,
    ROUND(MIN(duration_seconds), 2) as min_duration_sec,
    ROUND(MAX(duration_seconds), 2) as max_duration_sec
  FROM workflow_executions
  WHERE finished_at IS NOT NULL
  GROUP BY workflow_name;
"
```

### Log Files

Task logs are stored in `logs/` directory:

```
logs/
├── backup-workflow/
│   ├── 2025-11-12_020015_abc123/
│   │   ├── health_check.log
│   │   ├── backup_database.log
│   │   └── verify_backup.log
│   └── 2025-11-11_020010_def456/
│       └── ...
```

**Log retention:**

Configure in `picoflow.toml`:

```toml
[logs]
retention_days = 30
max_size_mb = 1000
```

Cleanup old logs:

```bash
# Manual cleanup
find logs/ -type f -mtime +30 -delete

# Automatic cleanup (runs daily if daemon is running)
# Configured via retention_days setting
```

---

## Prometheus Metrics

PicoFlow exposes Prometheus-compatible metrics for monitoring.

### Enabling Metrics

Metrics are enabled by default on port 9090.

**Configuration:**

```toml
[metrics]
enabled = true
port = 9090
bind_address = "127.0.0.1"  # localhost only for security
```

**Start with metrics:**

```bash
picoflow daemon start workflow.yaml
```

**Access metrics:**

```bash
curl http://localhost:9090/metrics
```

### Available Metrics

#### Workflow Metrics

```
# HELP picoflow_workflow_executions_total Total workflow executions
# TYPE picoflow_workflow_executions_total counter
picoflow_workflow_executions_total{workflow="backup-workflow",status="success"} 145
picoflow_workflow_executions_total{workflow="backup-workflow",status="failed"} 2

# HELP picoflow_workflow_duration_seconds Workflow execution duration
# TYPE picoflow_workflow_duration_seconds histogram
picoflow_workflow_duration_seconds_bucket{workflow="backup-workflow",le="60"} 0
picoflow_workflow_duration_seconds_bucket{workflow="backup-workflow",le="300"} 120
picoflow_workflow_duration_seconds_bucket{workflow="backup-workflow",le="600"} 147
picoflow_workflow_duration_seconds_sum{workflow="backup-workflow"} 58230
picoflow_workflow_duration_seconds_count{workflow="backup-workflow"} 147

# HELP picoflow_workflow_running Current running workflows
# TYPE picoflow_workflow_running gauge
picoflow_workflow_running{workflow="backup-workflow"} 0
```

#### Task Metrics

```
# HELP picoflow_task_executions_total Total task executions
# TYPE picoflow_task_executions_total counter
picoflow_task_executions_total{workflow="backup-workflow",task="backup_database",status="success"} 143
picoflow_task_executions_total{workflow="backup-workflow",task="backup_database",status="failed"} 2

# HELP picoflow_task_duration_seconds Task execution duration
# TYPE picoflow_task_duration_seconds histogram
picoflow_task_duration_seconds_bucket{workflow="backup-workflow",task="backup_database",le="60"} 0
picoflow_task_duration_seconds_bucket{workflow="backup-workflow",task="backup_database",le="300"} 135
picoflow_task_duration_seconds_sum{workflow="backup-workflow",task="backup_database"} 36720
picoflow_task_duration_seconds_count{workflow="backup-workflow",task="backup_database"} 145

# HELP picoflow_task_retries_total Total task retry attempts
# TYPE picoflow_task_retries_total counter
picoflow_task_retries_total{workflow="backup-workflow",task="backup_database"} 8
```

#### System Metrics

```
# HELP picoflow_info PicoFlow version info
# TYPE picoflow_info gauge
picoflow_info{version="1.0.0"} 1

# HELP picoflow_uptime_seconds PicoFlow daemon uptime
# TYPE picoflow_uptime_seconds counter
picoflow_uptime_seconds 182345

# HELP picoflow_memory_bytes Memory usage in bytes
# TYPE picoflow_memory_bytes gauge
picoflow_memory_bytes 19456000
```

### Prometheus Configuration

Add to `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'picoflow'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
```

### Grafana Dashboard

Example queries for Grafana:

**Workflow Success Rate:**

```promql
sum(rate(picoflow_workflow_executions_total{status="success"}[5m])) by (workflow)
/
sum(rate(picoflow_workflow_executions_total[5m])) by (workflow)
```

**Average Task Duration:**

```promql
rate(picoflow_task_duration_seconds_sum[5m])
/
rate(picoflow_task_duration_seconds_count[5m])
```

**Failed Tasks Alert:**

```promql
increase(picoflow_task_executions_total{status="failed"}[1h]) > 0
```

---

## Best Practices for Edge Devices

### Memory Management

**1. Limit Parallel Execution**

On devices with 512MB RAM (like Pi Zero 2 W):

```yaml
config:
  max_parallel: 2  # Conservative for 512MB
  # max_parallel: 4  # For 1GB+ devices
```

**2. Control Log Size**

```yaml
# Limit stdout/stderr capture
# For long-running tasks, redirect output to temp files
- name: large_backup
  type: shell
  config:
    command: "/opt/backup.sh"
    args: ["> /tmp/backup.log 2>&1"]
```

**3. Clean Up Resources**

```yaml
# Always include cleanup tasks
- name: cleanup
  type: shell
  depends_on: [main_task]
  config:
    command: "rm"
    args: ["-rf", "/tmp/workflow_temp"]
  continue_on_failure: true
```

### Storage Management

**1. Database Location**

Use fast storage (SD card vs USB):

```bash
# Move database to USB drive (if available)
picoflow --db-path /mnt/usb/picoflow.db daemon start workflows/*.yaml
```

**2. Log Rotation**

```toml
[logs]
retention_days = 7  # Shorter on constrained devices
max_size_mb = 100   # Limit total log size
```

**3. Cleanup Old Logs**

```yaml
# Add periodic cleanup workflow
name: cleanup-logs
schedule: "0 0 3 * * 0"  # Weekly on Sunday at 3 AM

tasks:
  - name: cleanup_old_logs
    type: shell
    config:
      command: "find"
      args: ["/var/lib/picoflow/logs", "-mtime", "+7", "-delete"]
```

### Network Reliability

**1. Retry Configuration**

Edge devices often have unreliable connectivity:

```yaml
config:
  retry_default: 5  # More retries for edge devices

tasks:
  - name: api_call
    type: http
    config:
      url: "https://api.example.com/data"
      method: GET
    retry: 10  # Even more for critical tasks
    timeout: 60
```

**2. Timeout Settings**

```yaml
# Conservative timeouts for slow networks
config:
  timeout_default: 600  # 10 minutes default

tasks:
  - name: large_upload
    type: ssh
    config:
      host: "backup.example.com"
      user: "backup"
      command: "rsync -av /data/ backup:/"
    timeout: 3600  # 1 hour for large transfers
```

### Power Management

**1. Avoid Peak Hours**

Schedule heavy tasks during off-peak times:

```yaml
# Schedule during night (lower power consumption, cooler)
schedule: "0 0 2 * * *"  # 2 AM
```

**2. Graceful Degradation**

```yaml
# Use continue_on_failure for non-critical tasks
- name: optional_cleanup
  type: shell
  config:
    command: "/opt/cleanup.sh"
  continue_on_failure: true
```

### Temperature Management

**1. Avoid Overheating**

```yaml
# Limit parallel tasks to reduce CPU load
config:
  max_parallel: 1  # For Pi Zero in enclosed cases

# Add cooling breaks between tasks
- name: cooling_break
  type: shell
  depends_on: [cpu_intensive_task]
  config:
    command: "sleep"
    args: ["30"]
```

### Security on Edge Devices

**1. File Permissions**

```bash
# Restrict permissions on sensitive files
chmod 600 ~/.ssh/picoflow_key
chmod 700 ~/.config/picoflow
chmod 600 ~/.config/picoflow/config.toml
```

**2. Dedicated User**

```bash
# Run as dedicated user (not root!)
sudo useradd -r -s /bin/false picoflow
sudo -u picoflow picoflow daemon start workflows/*.yaml
```

**3. Network Isolation**

```bash
# Bind metrics to localhost only
[metrics]
bind_address = "127.0.0.1"  # Not 0.0.0.0
```

---

## Performance Tuning

### Binary Size Optimization

PicoFlow is already optimized for size (3.0MB), but you can further reduce:

```bash
# Strip debug symbols
strip target/release/picoflow

# Use UPX compression (optional, may affect startup time)
upx --best --lzma target/release/picoflow
```

### Memory Optimization

**1. Monitor Memory Usage**

```bash
# Check current memory
ps aux | grep picoflow | awk '{print $6/1024 " MB"}'

# Monitor continuously
watch -n 5 'ps aux | grep picoflow'
```

**2. Adjust Parallel Limits**

```yaml
# Reduce for low-memory devices
config:
  max_parallel: 1  # 512MB devices
  max_parallel: 2  # 1GB devices
  max_parallel: 4  # 2GB+ devices
```

**3. Database Optimization**

```bash
# Vacuum database periodically
sqlite3 picoflow.db "VACUUM;"

# Clean old history
sqlite3 picoflow.db "
  DELETE FROM workflow_executions
  WHERE started_at < datetime('now', '-90 days');
"
```

### Execution Performance

**1. Minimize Task Overhead**

```yaml
# Combine small tasks into one
- name: multiple_checks
  type: shell
  config:
    command: "/bin/sh"
    args: ["-c", "check1.sh && check2.sh && check3.sh"]

# Instead of 3 separate tasks
```

**2. Optimize DAG Structure**

```yaml
# Enable parallelism by minimizing dependencies
tasks:
  - name: independent_task_1
    type: shell
    # No depends_on = runs immediately

  - name: independent_task_2
    type: shell
    # These two run in parallel

  - name: final_task
    depends_on: [independent_task_1, independent_task_2]
    # Runs after both complete
```

**3. Cache Remote Data**

```yaml
# Cache frequently accessed data locally
- name: fetch_config
  type: http
  config:
    url: "https://api.example.com/config"
    method: GET
  # Consider caching response to file

- name: use_cached_config
  type: shell
  depends_on: [fetch_config]
  config:
    command: "process_config.sh"
    args: ["/tmp/cached_config.json"]
```

### Network Performance

**1. Connection Pooling**

For workflows with many HTTP tasks to same host:

```yaml
# Keep tasks sequential to reuse connection
tasks:
  - name: api_call_1
    type: http
    config:
      url: "https://api.example.com/endpoint1"

  - name: api_call_2
    type: http
    depends_on: [api_call_1]  # Sequential = connection reuse
    config:
      url: "https://api.example.com/endpoint2"
```

**2. Timeout Tuning**

```yaml
# Aggressive timeouts for fast failure
- name: health_check
  type: http
  config:
    url: "https://api.example.com/health"
    timeout: 5  # Fail fast
  retry: 3

# Generous timeouts for large transfers
- name: large_download
  type: http
  config:
    url: "https://files.example.com/large.bin"
    timeout: 3600  # 1 hour
```

---

## Troubleshooting

For common issues and solutions, see [Troubleshooting Guide](troubleshooting.md).

### Quick Diagnostics

**Check system requirements:**

```bash
# Memory available
free -h

# Disk space
df -h

# PicoFlow version
picoflow --version

# Test workflow syntax
picoflow validate workflow.yaml

# Check daemon status
picoflow daemon status
```

**Enable debug logging:**

```bash
picoflow --log-level debug --log-format pretty run workflow.yaml
```

**Check logs:**

```bash
# View task logs
picoflow logs --workflow myworkflow --task mytask

# View daemon logs (systemd)
sudo journalctl -u picoflow -f

# View all logs
tail -f logs/*/latest/*.log
```

### Common Issues

**1. "Workflow has cycles" Error**

Your DAG has circular dependencies. Use validation to find the cycle:

```bash
picoflow validate workflow.yaml
```

**2. SSH Connection Failed**

Check SSH key authentication:

```bash
# Test SSH manually
ssh -i ~/.ssh/picoflow_key user@host "echo Success"

# Check key permissions
ls -la ~/.ssh/picoflow_key  # Should be 600
```

**3. Out of Memory**

Reduce parallel execution:

```yaml
config:
  max_parallel: 1  # Reduce from default
```

**4. Task Timeout**

Increase timeout for long-running tasks:

```yaml
- name: long_task
  timeout: 3600  # 1 hour
```

For more troubleshooting, see the [Troubleshooting Guide](troubleshooting.md).

---

## Next Steps

- Read the [API Reference](api-reference.md) for detailed YAML schema
- Check [FAQ](faq.md) for common questions
- Explore [example workflows](../examples/) for real-world use cases
- Join the community on [GitHub Discussions](https://github.com/zoza1982/picoflow/discussions)

---

**Document Version:** 1.0.0
**Last Updated:** November 12, 2025
**Feedback:** Report issues at https://github.com/zoza1982/picoflow/issues
