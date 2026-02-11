# PicoFlow API Reference

**Version:** 0.1.1
**Last Updated:** February 11, 2026

---

## Table of Contents

1. [Workflow YAML Schema](#workflow-yaml-schema)
2. [Task Configuration Schemas](#task-configuration-schemas)
3. [Shell Executor](#shell-executor)
4. [SSH Executor](#ssh-executor)
5. [HTTP Executor](#http-executor)
6. [Task Status States](#task-status-states)
7. [Exit Codes](#exit-codes)
8. [Environment Variables](#environment-variables)
9. [CLI Command Reference](#cli-command-reference)
10. [Configuration File](#configuration-file)
11. [Database Schema](#database-schema)

---

## Workflow YAML Schema

### Top-Level Workflow Structure

```yaml
name: string                     # Required
description: string              # Optional
schedule: string                 # Optional (cron expression)
config:                          # Optional
  max_parallel: integer          # Optional (default: 4)
  retry_default: integer         # Optional (default: 3)
  timeout_default: integer       # Optional (default: 300)
tasks: [Task]                    # Required (minimum 1 task)
```

### Field Specifications

#### `name` (required)

- **Type:** String
- **Format:** Alphanumeric, hyphens, underscores (no spaces)
- **Length:** 1-64 characters
- **Example:** `"backup-workflow"`, `"data_pipeline"`, `"health-check"`
- **Description:** Unique identifier for the workflow

#### `description` (optional)

- **Type:** String
- **Length:** 0-256 characters
- **Example:** `"Daily database backup with verification"`
- **Description:** Human-readable description of workflow purpose

#### `schedule` (optional)

- **Type:** String
- **Format:** 6-field cron expression
- **Syntax:** `"sec min hour day month dayofweek"`
- **Fields:**
  - `sec`: 0-59
  - `min`: 0-59
  - `hour`: 0-23
  - `day`: 1-31
  - `month`: 1-12
  - `dayofweek`: 0-6 (0 = Sunday)
- **Examples:**
  - `"0 0 2 * * *"` - Daily at 2 AM
  - `"0 */5 * * * *"` - Every 5 minutes
  - `"0 0 9 * * 1"` - Every Monday at 9 AM
- **Description:** Cron schedule for automatic execution (requires daemon mode)

#### `config` (optional)

Global workflow configuration object.

**Fields:**

| Field | Type | Default | Range | Description |
|-------|------|---------|-------|-------------|
| `max_parallel` | integer | 4 | 1-256 | Maximum concurrent tasks |
| `retry_default` | integer | 3 | 0-100 | Default retry count for all tasks |
| `timeout_default` | integer | 300 | 0-86400 | Default timeout in seconds (0 = no timeout) |

**Example:**
```yaml
config:
  max_parallel: 4
  retry_default: 3
  timeout_default: 600
```

#### `tasks` (required)

Array of task definitions. Minimum 1 task required.

---

## Task Configuration Schemas

### Common Task Fields

All task types share these fields:

```yaml
name: string                     # Required
type: string                     # Required (shell, ssh, http)
depends_on: [string]             # Optional (list of task names)
config: object                   # Required (type-specific)
retry: integer                   # Optional (overrides retry_default)
timeout: integer                 # Optional (overrides timeout_default)
continue_on_failure: boolean     # Optional (default: false)
```

### Task Field Specifications

#### `name` (required)

- **Type:** String
- **Format:** Alphanumeric, hyphens, underscores
- **Length:** 1-64 characters
- **Uniqueness:** Must be unique within workflow
- **Example:** `"backup_database"`, `"health-check"`, `"deploy-app"`

#### `type` (required)

- **Type:** String (enum)
- **Values:** `"shell"`, `"ssh"`, `"http"`
- **Description:** Executor type for this task

#### `depends_on` (optional)

- **Type:** Array of strings
- **Format:** List of task names
- **Default:** `[]` (no dependencies)
- **Example:** `["task1", "task2"]`
- **Validation:**
  - Referenced tasks must exist in workflow
  - No circular dependencies allowed (DAG validation)
- **Description:** Tasks that must complete successfully before this task starts

#### `config` (required)

- **Type:** Object
- **Schema:** Type-specific (see executor sections below)
- **Description:** Configuration specific to the executor type

#### `retry` (optional)

- **Type:** Integer
- **Range:** 0-100
- **Default:** Inherited from `config.retry_default` (default: 3)
- **Example:** `3`
- **Description:** Number of retry attempts on failure
- **Behavior:** Exponential backoff between retries (2^(attempt-1) seconds, capped at 60s)

#### `timeout` (optional)

- **Type:** Integer (seconds)
- **Range:** 0-86400 (0 = no timeout)
- **Default:** Inherited from `config.timeout_default` (default: 300)
- **Example:** `600` (10 minutes)
- **Description:** Maximum execution time before task is killed

#### `continue_on_failure` (optional)

- **Type:** Boolean
- **Default:** `false`
- **Example:** `true`
- **Description:** If true, workflow continues even if this task fails
- **Use case:** Cleanup tasks, optional notifications

---

## Shell Executor

Execute commands on the local system.

### Type Identifier

```yaml
type: shell
```

### Configuration Schema

```yaml
config:
  command: string                # Required
  args: [string]                # Optional
  workdir: string           # Optional
  env: {string: string}         # Optional
```

### Configuration Fields

#### `command` (required)

- **Type:** String
- **Format:** Absolute path to executable (recommended) or command name
- **Example:** `"/usr/bin/tar"`, `"/opt/scripts/backup.sh"`
- **Security:** Always use absolute paths to prevent PATH injection
- **Description:** Command or script to execute

#### `args` (optional)

- **Type:** Array of strings
- **Default:** `[]`
- **Example:** `["-czf", "/backup/logs.tar.gz", "/var/log"]`
- **Description:** Command-line arguments passed to command
- **Security:** Arguments are passed safely (no shell interpolation)

#### `workdir` (optional)

- **Type:** String (absolute path)
- **Default:** PicoFlow's current working directory
- **Example:** `"/opt/app"`
- **Description:** Working directory for command execution

#### `env` (optional)

- **Type:** Object (key-value pairs)
- **Default:** `{}` (inherits parent environment)
- **Example:**
  ```yaml
  env:
    DB_HOST: "localhost"
    DB_NAME: "production"
    BACKUP_DIR: "/mnt/backup"
  ```
- **Description:** Environment variables for command execution
- **Inheritance:** Parent environment variables are inherited

### Complete Example

```yaml
- name: backup_logs
  type: shell
  config:
    command: "/usr/bin/tar"
    args:
      - "-czf"
      - "/backup/logs-$(date +%Y%m%d).tar.gz"
      - "/var/log/myapp"
    workdir: "/tmp"
    env:
      BACKUP_RETENTION_DAYS: "7"
  retry: 2
  timeout: 300
```

### Success Criteria

- Exit code 0
- No timeout
- Command execution completes

### Output Capture

- `stdout` and `stderr` captured and stored
- Accessible via: `picoflow logs --workflow <name> --task <task_name>`
- Storage limit: 10MB per task (truncated if exceeded)

---

## SSH Executor

Execute commands on remote systems via SSH.

### Type Identifier

```yaml
type: ssh
```

### Configuration Schema

```yaml
config:
  host: string                   # Required
  port: integer                  # Optional (default: 22)
  user: string                   # Required
  command: string                # Required
  key_path: string               # Optional (default: ~/.ssh/id_rsa)
  timeout: integer               # Optional (default: 30)
```

### Configuration Fields

#### `host` (required)

- **Type:** String
- **Format:** Hostname or IP address
- **Example:** `"db.example.com"`, `"192.168.1.100"`, `"server01"`
- **Description:** Remote host to connect to

#### `port` (optional)

- **Type:** Integer
- **Range:** 1-65535
- **Default:** `22`
- **Example:** `2222`
- **Description:** SSH port on remote host

#### `user` (required)

- **Type:** String
- **Format:** Valid Unix username
- **Example:** `"deploy"`, `"backup"`, `"admin"`
- **Description:** Username for SSH connection

#### `command` (required)

- **Type:** String
- **Format:** Shell command to execute remotely
- **Example:** `"pg_dump mydb | gzip > /backup/db.sql.gz"`
- **Description:** Command executed on remote host
- **Note:** Executed in remote shell (shell features available)

#### `key_path` (optional)

- **Type:** String (absolute path)
- **Default:** `"~/.ssh/id_rsa"` (expanded to user home directory)
- **Example:** `"/home/picoflow/.ssh/backup_key"`
- **Permissions:** Must be 600 (readable only by owner)
- **Description:** Path to SSH private key for authentication

#### `timeout` (optional)

- **Type:** Integer (seconds)
- **Range:** 1-3600
- **Default:** `30`
- **Example:** `60`
- **Description:** Connection timeout (not command timeout)
- **Note:** Use task-level `timeout` for command execution timeout

### Complete Example

```yaml
- name: remote_backup
  type: ssh
  config:
    host: "backup.example.com"
    port: 22
    user: "backup"
    command: "pg_dump -U postgres production | gzip > /backup/db-$(date +%Y%m%d).sql.gz"
    key_path: "/home/picoflow/.ssh/backup_key"
    timeout: 30
  retry: 3
  timeout: 1800  # 30-minute command timeout
```

### Authentication

- **Method:** Key-based authentication only
- **Password auth:** Not supported (security policy)
- **Key setup:**
  ```bash
  ssh-keygen -t ed25519 -f ~/.ssh/picoflow_key
  ssh-copy-id -i ~/.ssh/picoflow_key.pub user@remote-host
  chmod 600 ~/.ssh/picoflow_key
  ```

### Success Criteria

- SSH connection established
- Command exits with code 0
- No timeout

### Security

- Host key verification required (add to `~/.ssh/known_hosts` first)
- Private key must have 600 permissions
- No password authentication
- Command injection prevented (but shell features available in command string)

---

## HTTP Executor

Make HTTP/HTTPS requests to REST APIs.

### Type Identifier

```yaml
type: http
```

### Configuration Schema

```yaml
config:
  url: string                    # Required
  method: string                 # Required
  headers: {string: string}      # Optional
  body: object                   # Optional
  timeout: integer               # Optional (default: 30)
```

### Configuration Fields

#### `url` (required)

- **Type:** String
- **Format:** Valid HTTP/HTTPS URL
- **Example:**
  - `"https://api.example.com/health"`
  - `"http://192.168.1.100:8080/trigger"`
- **Description:** Full URL for HTTP request

#### `method` (required)

- **Type:** String (enum)
- **Values:** `"GET"`, `"POST"`, `"PUT"`, `"DELETE"`, `"PATCH"`, `"HEAD"`, `"OPTIONS"`
- **Case:** Insensitive
- **Example:** `"GET"`, `"post"`, `"Put"`
- **Description:** HTTP method

#### `headers` (optional)

- **Type:** Object (key-value pairs)
- **Default:** `{}`
- **Example:**
  ```yaml
  headers:
    Content-Type: "application/json"
    Authorization: "Bearer ${API_TOKEN}"
    User-Agent: "PicoFlow/1.0"
  ```
- **Description:** HTTP headers
- **Variable substitution:** Environment variables expanded (`${VAR_NAME}`)

#### `body` (optional)

- **Type:** Object (JSON)
- **Default:** None (no body)
- **Example:**
  ```yaml
  body:
    name: "New Resource"
    type: "server"
    region: "us-east-1"
  ```
- **Serialization:** Automatically serialized to JSON
- **Content-Type:** Automatically set to `application/json` if not specified
- **Description:** Request body (for POST, PUT, PATCH)

#### `timeout` (optional)

- **Type:** Integer (seconds)
- **Range:** 1-3600
- **Default:** `30`
- **Example:** `60`
- **Description:** Request timeout (connection + read)

### Complete Examples

#### GET Request

```yaml
- name: health_check
  type: http
  config:
    url: "https://api.example.com/health"
    method: GET
    timeout: 10
  retry: 2
```

#### POST with JSON Body

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
      type: "database"
      size: "large"
    timeout: 30
  retry: 3
```

#### PUT Request

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

#### DELETE with Custom Success Codes

```yaml
- name: cleanup_resource
  type: http
  config:
    url: "https://api.example.com/tmp-resources/xyz"
    method: DELETE
  continue_on_failure: true  # 2xx status = success, 4xx/5xx = failure
```

### Success Criteria

- HTTP response received
- Status code in 2xx range (200-299)
- No connection timeout

### Authentication

Use environment variables for secrets:

```yaml
headers:
  Authorization: "Bearer ${API_TOKEN}"
  X-API-Key: "${API_SECRET}"
```

Set before running:
```bash
export API_TOKEN="your-secret-token"
export API_SECRET="your-secret-key"
picoflow run workflow.yaml
```

### Security

- HTTPS recommended for production
- SSL certificate verification enabled by default
- Secrets via environment variables (not in YAML)
- No logging of Authorization headers

---

## Task Status States

Tasks progress through the following states:

```rust
enum TaskStatus {
    Pending,    // Waiting for dependencies or execution slot
    Running,    // Currently executing
    Success,    // Completed successfully (exit code 0 or HTTP 2xx)
    Failed,     // Completed with error
    Retrying,   // Failed but will retry
    Timeout,    // Exceeded timeout limit
}
```

### State Transitions

```
                    ┌─────────────────────┐
                    │      Pending        │
                    └──────────┬──────────┘
                               │
                               ▼
                    ┌─────────────────────┐
                    │      Running        │
                    └──────────┬──────────┘
                               │
                ┌──────────────┴──────────────┐
                │                             │
                ▼                             ▼
     ┌─────────────────────┐       ┌─────────────────────┐
     │      Success        │       │   Failed/Timeout    │
     └─────────────────────┘       └──────────┬──────────┘
                                               │
                                    retry_count < max_retry?
                                               │
                                    ┌──────────┴──────────┐
                                    │                     │
                                   Yes                   No
                                    │                     │
                                    ▼                     ▼
                         ┌─────────────────────┐   ┌──────────┐
                         │     Retrying        │   │  Failed  │
                         └──────────┬──────────┘   └──────────┘
                                    │
                                    ▼
                         ┌─────────────────────┐
                         │      Running        │
                         └─────────────────────┘
```

### State Descriptions

| State | Description | Next States |
|-------|-------------|-------------|
| `Pending` | Task waiting to start (dependencies not met or max_parallel limit reached) | `Running` |
| `Running` | Task currently executing | `Success`, `Failed`, `Timeout` |
| `Success` | Task completed successfully | N/A (terminal state) |
| `Failed` | Task failed and no more retries | N/A (terminal state) |
| `Retrying` | Task failed but will retry (transient state) | `Running` |
| `Timeout` | Task exceeded timeout and was killed | `Retrying` or `Failed` |

### Querying Status

```bash
# Via CLI
picoflow status --workflow myworkflow

# Via database
sqlite3 picoflow.db "
  SELECT task_name, status, started_at, finished_at
  FROM task_executions
  WHERE workflow_execution_id = 'abc123'
  ORDER BY started_at;
"
```

---

## Exit Codes

### PicoFlow CLI Exit Codes

| Exit Code | Meaning | Description |
|-----------|---------|-------------|
| `0` | Success | Command completed successfully |
| `1` | General error | Unspecified error occurred |
| `2` | Validation error | Workflow validation failed (cycles, missing tasks, etc.) |
| `3` | Execution error | Workflow execution failed (one or more tasks failed) |
| `4` | Configuration error | Invalid configuration file or CLI arguments |
| `5` | IO error | File not found, permission denied, etc. |
| `6` | Database error | SQLite error (corrupted DB, locked, etc.) |
| `7` | Network error | Connection failed (SSH, HTTP) |
| `8` | Timeout | Operation timed out |
| `9` | Interrupted | Signal received (SIGINT, SIGTERM) |

### Task Exit Codes

Task exit codes depend on the executor:

**Shell executor:**
- Exit code from executed command (0-255)
- 0 = success, non-zero = failure

**SSH executor:**
- Exit code from remote command (0-255)
- 255 = SSH connection failed
- 124 = Command timed out

**HTTP executor:**
- 0 = HTTP status in 2xx range
- 1 = HTTP error (wrong status, timeout, connection failed)

### Checking Exit Codes

```bash
# In shell scripts
picoflow run workflow.yaml
if [ $? -eq 0 ]; then
  echo "Success"
else
  echo "Failed with code $?"
fi

# Or
picoflow run workflow.yaml && echo "Success" || echo "Failed"
```

---

## Environment Variables

### PicoFlow Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PICOFLOW_DB_PATH` | String | `picoflow.db` | Database file path |
| `PICOFLOW_LOG_LEVEL` | String | `info` | Log level (error, warn, info, debug, trace) |
| `PICOFLOW_LOG_FORMAT` | String | `json` | Log format (json, pretty) |
| `PICOFLOW_METRICS_PORT` | Integer | `9090` | Prometheus metrics port |
| `RUST_LOG` | String | Inherited | Rust tracing filter |
| `RUST_BACKTRACE` | String | `0` | Enable backtraces (0, 1, full) |

### Usage in Workflows

Environment variables can be referenced in workflow YAML:

```yaml
- name: use_env_vars
  type: shell
  config:
    command: "/bin/echo"
    args: ["Database: ${DB_NAME}"]
    env:
      DB_HOST: "${DB_HOST}"
      DB_PORT: "5432"
```

### Variable Expansion

- **Syntax:** `${VAR_NAME}`
- **Scope:** Workflow file only (not task outputs)
- **Undefined variables:** Empty string (no error)
- **Escaping:** Use `$$` for literal `$`

### Setting Variables

```bash
# Single workflow run
export DB_NAME="production"
export API_TOKEN="secret"
picoflow run workflow.yaml

# Daemon mode
export DB_NAME="production"
picoflow daemon start workflow.yaml

# Systemd service
# /etc/systemd/system/picoflow.service
[Service]
Environment="DB_NAME=production"
Environment="API_TOKEN=secret"
```

---

## CLI Command Reference

### Global Flags

Available for all commands:

```bash
picoflow [FLAGS] <COMMAND>
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-l, --log-level <LEVEL>` | String | `info` | Log level: error, warn, info, debug, trace |
| `--log-format <FORMAT>` | String | `json` | Log format: json, pretty |
| `--db-path <PATH>` | String | `picoflow.db` | Database file path |
| `-h, --help` | Flag | - | Print help |
| `-V, --version` | Flag | - | Print version |

### Commands

#### `picoflow run`

Execute a workflow once.

```bash
picoflow run [OPTIONS] <WORKFLOW_FILE>
```

**Arguments:**
- `<WORKFLOW_FILE>`: Path to workflow YAML file

**Options:** (none specific, uses global flags)

**Examples:**
```bash
picoflow run backup.yaml
picoflow --log-level debug run backup.yaml
picoflow --db-path /data/picoflow.db run backup.yaml
```

**Exit codes:**
- 0: Workflow completed successfully
- 3: Workflow failed (one or more tasks failed)
- 2: Validation error
- 5: File not found

---

#### `picoflow validate`

Validate workflow YAML and DAG structure.

```bash
picoflow validate <WORKFLOW_FILE>
```

**Arguments:**
- `<WORKFLOW_FILE>`: Path to workflow YAML file

**Output:**
- Validation result (success or error)
- Task count
- Dependency validation
- DAG structure

**Examples:**
```bash
picoflow validate backup.yaml
```

**Exit codes:**
- 0: Valid workflow
- 2: Validation error

---

#### `picoflow status`

Show workflow execution status.

```bash
picoflow status [OPTIONS]
```

**Options:**
- `--workflow <NAME>`: Filter by workflow name
- `--running-only`: Show only running workflows

**Examples:**
```bash
picoflow status
picoflow status --workflow backup-workflow
picoflow status --running-only
```

**Exit codes:**
- 0: Success
- 6: Database error

---

#### `picoflow workflow list`

List all workflows with statistics.

```bash
picoflow workflow list [OPTIONS]
```

**Options:**
- `--all`: Show all workflows including inactive
- `--format <FORMAT>`: Output format (table, json)

**Examples:**
```bash
picoflow workflow list
picoflow workflow list --format json
```

**Exit codes:**
- 0: Success
- 6: Database error

---

#### `picoflow daemon start`

Start daemon with workflows.

```bash
picoflow daemon start <WORKFLOW_FILE>...
```

**Arguments:**
- `<WORKFLOW_FILE>...`: One or more workflow YAML files

**Behavior:**
- Loads and validates all workflows
- Schedules cron jobs for workflows with `schedule` field
- Runs in foreground (use systemd or nohup for background)
- Writes PID to `picoflow.pid`

**Examples:**
```bash
picoflow daemon start backup.yaml
picoflow daemon start backup.yaml monitoring.yaml
picoflow daemon start workflows/*.yaml
```

**Exit codes:**
- 0: Daemon stopped gracefully
- 1: Error starting daemon
- 9: Interrupted (SIGINT/SIGTERM)

---

#### `picoflow daemon stop`

Stop running daemon.

```bash
picoflow daemon stop
```

**Behavior:**
- Sends SIGTERM to daemon process
- Waits for graceful shutdown (60s timeout)
- Removes PID file

**Exit codes:**
- 0: Daemon stopped
- 1: Daemon not running or failed to stop

---

#### `picoflow daemon status`

Check daemon status.

```bash
picoflow daemon status
```

**Output:**
- Daemon running/stopped
- PID
- Uptime
- Loaded workflows
- Next scheduled run
- Memory usage

**Exit codes:**
- 0: Daemon running
- 1: Daemon not running

---

#### `picoflow history`

Query workflow execution history.

```bash
picoflow history [OPTIONS]
```

**Options:**
- `--workflow <NAME>`: Filter by workflow name
- `--status <STATUS>`: Filter by status (success, failed, running)
- `--limit <N>`: Limit results (default: 10)
- `--since <DATE>`: Show since date (YYYY-MM-DD)
- `--format <FORMAT>`: Output format (table, json)

**Examples:**
```bash
picoflow history
picoflow history --workflow backup-workflow --limit 20
picoflow history --status failed --since 2025-11-01
picoflow history --format json > history.json
```

**Exit codes:**
- 0: Success
- 6: Database error

---

#### `picoflow stats`

Show workflow statistics.

```bash
picoflow stats [OPTIONS]
```

**Options:**
- `--workflow <NAME>`: Filter by workflow name
- `--period <DAYS>`: Time period in days (default: 30)

**Examples:**
```bash
picoflow stats
picoflow stats --workflow backup-workflow
picoflow stats --period 7
```

**Exit codes:**
- 0: Success
- 6: Database error

---

#### `picoflow logs`

View task execution logs.

```bash
picoflow logs [OPTIONS]
```

**Options:**
- `--workflow <NAME>`: Workflow name (required)
- `--task <NAME>`: Task name (optional)
- `--execution-id <ID>`: Specific execution ID
- `--tail <N>`: Show last N lines (default: 100)
- `--follow`: Follow log output (like tail -f)

**Examples:**
```bash
picoflow logs --workflow backup-workflow
picoflow logs --workflow backup-workflow --task backup_database
picoflow logs --workflow backup-workflow --follow
picoflow logs --workflow backup-workflow --tail 50
```

**Exit codes:**
- 0: Success
- 5: Log file not found
- 6: Database error

---

## Configuration File

### File Locations

PicoFlow searches for configuration in this order:

1. `./picoflow.toml` (current directory)
2. `~/.config/picoflow/config.toml` (user config)
3. `/etc/picoflow/config.toml` (system-wide)

### Configuration Schema

```toml
# Database configuration
db_path = "/var/lib/picoflow/picoflow.db"

# Logging configuration
log_level = "info"  # error, warn, info, debug, trace
log_format = "json"  # json, pretty

# Metrics configuration
[metrics]
enabled = true
port = 9090
bind_address = "127.0.0.1"  # Bind to localhost only

# Retry configuration
[retry]
default_count = 3
max_backoff_seconds = 300
backoff_multiplier = 2.0

# Timeout configuration
[timeout]
default_seconds = 300
max_seconds = 3600

# Execution configuration
[execution]
max_parallel = 4

# Log retention
[logs]
retention_days = 30
max_size_mb = 1000
cleanup_enabled = true
```

### Example Configuration

```toml
# /etc/picoflow/config.toml

db_path = "/var/lib/picoflow/picoflow.db"
log_level = "info"
log_format = "json"

[metrics]
enabled = true
port = 9090
bind_address = "127.0.0.1"

[retry]
default_count = 5
max_backoff_seconds = 600
backoff_multiplier = 2.0

[timeout]
default_seconds = 600
max_seconds = 7200

[execution]
max_parallel = 4

[logs]
retention_days = 30
max_size_mb = 500
cleanup_enabled = true
```

---

## Database Schema

PicoFlow uses SQLite for state persistence.

### Tables

#### `workflow_executions`

Stores workflow execution records.

```sql
CREATE TABLE workflow_executions (
    id TEXT PRIMARY KEY,              -- UUID v4
    workflow_name TEXT NOT NULL,      -- Workflow name from YAML
    started_at TEXT NOT NULL,         -- ISO 8601 timestamp
    finished_at TEXT,                 -- ISO 8601 timestamp (NULL if running)
    status TEXT NOT NULL,             -- pending, running, success, failed
    duration_seconds REAL,            -- Execution duration
    total_tasks INTEGER NOT NULL,     -- Total task count
    successful_tasks INTEGER,         -- Number of successful tasks
    failed_tasks INTEGER,             -- Number of failed tasks
    created_at TEXT NOT NULL          -- Record creation timestamp
);

CREATE INDEX idx_workflow_name ON workflow_executions(workflow_name);
CREATE INDEX idx_started_at ON workflow_executions(started_at);
CREATE INDEX idx_status ON workflow_executions(status);
```

#### `task_executions`

Stores individual task execution records.

```sql
CREATE TABLE task_executions (
    id TEXT PRIMARY KEY,                      -- UUID v4
    workflow_execution_id TEXT NOT NULL,      -- FK to workflow_executions
    task_name TEXT NOT NULL,                  -- Task name from YAML
    status TEXT NOT NULL,                     -- pending, running, success, failed, timeout
    started_at TEXT NOT NULL,                 -- ISO 8601 timestamp
    finished_at TEXT,                         -- ISO 8601 timestamp (NULL if running)
    exit_code INTEGER,                        -- Command exit code
    stdout TEXT,                              -- Captured stdout (truncated at 10MB)
    stderr TEXT,                              -- Captured stderr (truncated at 10MB)
    attempt INTEGER NOT NULL DEFAULT 1,       -- Retry attempt number
    error_message TEXT,                       -- Error message if failed
    duration_seconds REAL,                    -- Execution duration
    created_at TEXT NOT NULL,                 -- Record creation timestamp
    FOREIGN KEY (workflow_execution_id) REFERENCES workflow_executions(id) ON DELETE CASCADE
);

CREATE INDEX idx_workflow_execution_id ON task_executions(workflow_execution_id);
CREATE INDEX idx_task_name ON task_executions(task_name);
CREATE INDEX idx_task_status ON task_executions(status);
CREATE INDEX idx_task_started_at ON task_executions(started_at);
```

### Query Examples

**Recent workflow executions:**

```sql
SELECT
  workflow_name,
  started_at,
  status,
  duration_seconds,
  successful_tasks || '/' || total_tasks as tasks
FROM workflow_executions
ORDER BY started_at DESC
LIMIT 10;
```

**Failed tasks in last 24 hours:**

```sql
SELECT
  we.workflow_name,
  te.task_name,
  te.started_at,
  te.error_message
FROM task_executions te
JOIN workflow_executions we ON te.workflow_execution_id = we.id
WHERE te.status = 'failed'
  AND te.started_at > datetime('now', '-1 day')
ORDER BY te.started_at DESC;
```

**Workflow success rate:**

```sql
SELECT
  workflow_name,
  COUNT(*) as total,
  SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) as success,
  ROUND(100.0 * SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) / COUNT(*), 2) as success_rate
FROM workflow_executions
WHERE started_at > datetime('now', '-30 days')
GROUP BY workflow_name
ORDER BY success_rate DESC;
```

**Average task duration:**

```sql
SELECT
  task_name,
  COUNT(*) as executions,
  ROUND(AVG(duration_seconds), 2) as avg_duration,
  ROUND(MIN(duration_seconds), 2) as min_duration,
  ROUND(MAX(duration_seconds), 2) as max_duration
FROM task_executions
WHERE finished_at IS NOT NULL
  AND started_at > datetime('now', '-30 days')
GROUP BY task_name
ORDER BY avg_duration DESC;
```

---

**Document Version:** 0.1.1
**Last Updated:** February 11, 2026
**Feedback:** Report issues at https://github.com/zoza1982/picoflow/issues
