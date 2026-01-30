# PicoFlow Architecture

## Overview

PicoFlow is designed as a modular, async-first workflow orchestrator optimized for edge devices. The architecture prioritizes minimal memory footprint, fast startup times, and production-grade reliability.

## Design Principles

1. **Memory Efficiency**: <20MB idle, <50MB with 10 parallel tasks
2. **Async-First**: All I/O operations are async using tokio
3. **Fail-Fast**: Detect errors early (DAG validation, YAML parsing)
4. **Observable**: Structured logging and metrics from day one
5. **Modular**: Pluggable executors, configurable components
6. **Edge-Ready**: Target Raspberry Pi Zero 2 W (512MB RAM baseline)
7. **Secure-by-Default**: Input validation, command injection prevention, resource limits

## Input Validation Limits

To prevent DoS attacks and resource exhaustion on edge devices, enforce these limits:

| Limit | Value | Rationale |
|-------|-------|-----------|
| MAX_YAML_SIZE | 1 MB | Prevent OOM during parsing |
| MAX_TASK_COUNT | 1,000 | Memory budget for DAG engine |
| MAX_TASK_NAME_LEN | 64 chars | Prevent excessive string allocations |
| MAX_COMMAND_LEN | 4 KB | Reasonable command path limit |
| MAX_ARG_COUNT | 256 | Prevent argument explosion |
| MAX_ARG_LEN | 4 KB | Per-argument size limit |
| MAX_OUTPUT_SIZE | 10 MB | Prevent OOM from chatty commands |
| MAX_RESPONSE_SIZE | 10 MB | HTTP executor response limit |

All limits are enforced at parse/execution time with clear error messages.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI Layer                           │
│  (clap-based: run, validate, status, logs, daemon)         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Workflow Parser                         │
│           (YAML → WorkflowConfig validation)                │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                        DAG Engine                            │
│  - Topological sort (petgraph)                              │
│  - Cycle detection                                          │
│  - Dependency resolution                                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Task Scheduler                          │
│  - Cron-based scheduling (tokio-cron-scheduler)             │
│  - Daemon mode with signal handling                         │
│  - Sequential/parallel execution engine                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Executor Interface                        │
│              (async trait ExecutorTrait)                    │
└─────────────────────────────────────────────────────────────┘
           │                  │                  │
           ▼                  ▼                  ▼
    ┌──────────┐      ┌──────────┐      ┌──────────┐
    │  Shell   │      │   SSH    │      │   HTTP   │
    │ Executor │      │ Executor │      │ Executor │
    └──────────┘      └──────────┘      └──────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      State Manager                           │
│  - SQLite persistence                                       │
│  - Task state tracking (Pending→Running→Success/Failed)    │
│  - Execution history                                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  Observability Layer                         │
│  - Structured logging (tracing + JSON)                      │
│  - Prometheus metrics endpoint                              │
│  - Execution history queries                                │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. CLI Layer (`src/cli.rs`)

Entry point for all user interactions. Uses `clap` for argument parsing.

**Commands:**
- `run <workflow.yaml>`: Execute workflow once
- `validate <workflow.yaml>`: Validate YAML and DAG
- `daemon <workflow.yaml>`: Run in background with scheduling
- `status`: Show running workflows and task status
- `logs --workflow <name> --task <task>`: Query execution logs
- `history --workflow <name>`: Show execution history

### 2. Workflow Parser (`src/parser.rs`)

Parses YAML files into strongly-typed Rust structs with comprehensive validation.

**Key Types:**
```rust
pub struct WorkflowConfig {
    pub name: String,
    pub description: Option<String>,
    pub schedule: Option<String>,  // Cron expression
    pub config: WorkflowGlobalConfig,
    pub tasks: Vec<TaskConfig>,
}

pub struct TaskConfig {
    pub name: String,
    pub task_type: TaskType,
    pub depends_on: Vec<String>,
    pub config: TaskExecutorConfig,
    pub retry: Option<u32>,
    pub timeout: Option<u64>,
    pub continue_on_failure: bool,
}

pub enum TaskType {
    Shell,
    Ssh,
    Http,
}
```

**Validation with Input Limits:**
```rust
impl WorkflowConfig {
    pub fn from_yaml(content: &str) -> Result<Self> {
        // Limit YAML size to prevent OOM
        if content.len() > MAX_YAML_SIZE {
            return Err(anyhow!("Workflow YAML exceeds 1MB limit"));
        }

        let config: Self = serde_yaml::from_str(content)?;

        // Validate task count
        if config.tasks.len() > MAX_TASK_COUNT {
            return Err(anyhow!("Task count {} exceeds limit of {}",
                config.tasks.len(), MAX_TASK_COUNT));
        }

        // Validate task names (alphanumeric + underscore/dash only)
        for task in &config.tasks {
            if task.name.len() > MAX_TASK_NAME_LEN {
                return Err(anyhow!("Task name '{}' exceeds {} chars",
                    task.name, MAX_TASK_NAME_LEN));
            }
            if !task.name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                return Err(anyhow!("Invalid task name '{}': only alphanumeric, _, - allowed",
                    task.name));
            }
        }

        Ok(config)
    }
}
```

**Validation Steps:**
- YAML size limit (1MB)
- YAML syntax validation (serde_yaml)
- Task count limit (1,000 tasks)
- Task name format validation (alphanumeric + _ -)
- Required fields presence
- Type checking (e.g., timeout is numeric)
- Defer DAG validation to engine

### 3. DAG Engine (`src/dag.rs`)

Converts task list into executable DAG.

**Responsibilities:**
- Build directed graph from `depends_on` relationships
- Detect cycles (fail-fast if cyclic)
- Topological sort for execution order
- Calculate task levels for parallelism

**Algorithm:**
```rust
pub struct DagEngine {
    graph: DiGraph<TaskNode, ()>,
}

impl DagEngine {
    pub fn build(tasks: Vec<TaskConfig>) -> Result<Self>;
    pub fn validate_acyclic(&self) -> Result<()>;
    pub fn topological_sort(&self) -> Result<Vec<TaskId>>;
    pub fn parallel_levels(&self) -> Vec<Vec<TaskId>>;
}
```

**Performance Target:**
- Parse and validate 100-task DAG in <50ms

### 4. Task Scheduler (`src/scheduler.rs`)

Manages workflow execution lifecycle with semaphore-based concurrency control.

**Modes:**
- **One-shot**: Run workflow once and exit
- **Daemon**: Background process with cron scheduling

**State Machine:**
```
Pending → Running → Success
              ↓
           Failed → Retrying → Running
              ↓
           Timeout
```

**Retry Logic:**
- Exponential backoff: `delay = base_delay * 2^attempt`
- Configurable max retries per task
- Default: 3 retries, 1s base delay

**Concurrency Control (Semaphore-Based):**
```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct TaskScheduler {
    max_parallel: usize,
    semaphore: Arc<Semaphore>,
    state_manager: Arc<StateManager>,
}

impl TaskScheduler {
    pub fn new(max_parallel: usize, state_manager: Arc<StateManager>) -> Self {
        Self {
            max_parallel,
            semaphore: Arc::new(Semaphore::new(max_parallel)),
            state_manager,
        }
    }

    pub async fn execute_level(&self, tasks: Vec<TaskConfig>) -> Result<()> {
        let mut handles = Vec::with_capacity(tasks.len().min(self.max_parallel));

        for task in tasks {
            // Acquire semaphore permit (blocks if at limit)
            let permit = self.semaphore.clone().acquire_owned().await?;
            let state = self.state_manager.clone();

            let handle = tokio::spawn(async move {
                let result = execute_task(task, &state).await;
                drop(permit); // Release semaphore on completion
                result
            });
            handles.push(handle);
        }

        // Wait for all tasks in this level to complete
        for handle in handles {
            handle.await??;
        }

        Ok(())
    }
}
```

**Memory Budget:**
- Semaphore overhead: ~64 bytes
- Task tracking: ~1MB per 10 tasks
- Backpressure prevents spawning unbounded tasks

### 5. Executor Interface (`src/executors/mod.rs`)

Trait-based pluggable execution backends with output size limits.

```rust
#[async_trait]
pub trait ExecutorTrait: Send + Sync {
    async fn execute(&self, config: &TaskExecutorConfig) -> Result<ExecutionResult>;
    async fn health_check(&self) -> Result<()>;
}

pub struct ExecutionResult {
    pub status: TaskStatus,
    pub stdout: Option<String>,      // None if capture disabled or truncated
    pub stderr: Option<String>,      // None if capture disabled or truncated
    pub exit_code: Option<i32>,
    pub duration: Duration,
    pub output_truncated: bool,      // True if output exceeded MAX_OUTPUT_SIZE
}

pub struct ExecutorConfig {
    pub max_output_size: usize,      // Default: 10MB, prevents OOM from chatty commands
    pub capture_output: bool,        // Default: true, can disable to save memory
}
```

#### Shell Executor (`src/executors/shell.rs`)

Executes local shell commands using `tokio::process::Command`.

**Secure Implementation:**
```rust
pub async fn execute_shell(config: &ShellConfig) -> Result<ExecutionResult> {
    // CRITICAL: Use Command::new with individual args, NOT shell string interpolation
    let mut cmd = Command::new(&config.command); // Binary path only

    // Add args individually (no shell expansion)
    for arg in &config.args {
        if arg.len() > MAX_ARG_LEN {
            return Err(anyhow!("Argument exceeds {} bytes", MAX_ARG_LEN));
        }
        cmd.arg(arg);
    }

    // Set working directory with path validation
    if let Some(workdir) = &config.workdir {
        let path = validate_path(workdir)?;  // Reject .. traversal
        cmd.current_dir(path);
    }

    // Enforce timeout
    let output = tokio::time::timeout(
        Duration::from_secs(config.timeout),
        cmd.output()
    ).await??;

    // Cap output size to prevent OOM
    let stdout = truncate_output(&output.stdout, MAX_OUTPUT_SIZE);
    let stderr = truncate_output(&output.stderr, MAX_OUTPUT_SIZE);

    Ok(ExecutionResult {
        status: if output.status.success() { TaskStatus::Success } else { TaskStatus::Failed },
        stdout: Some(stdout.to_string()),
        stderr: Some(stderr.to_string()),
        exit_code: output.status.code(),
        duration: /* measure execution time */,
        output_truncated: output.stdout.len() > MAX_OUTPUT_SIZE || output.stderr.len() > MAX_OUTPUT_SIZE,
    })
}
```

**YAML Schema (Secure):**
```yaml
tasks:
  - name: safe_command
    type: shell
    config:
      command: "/usr/bin/rsync"  # Absolute path to binary (required)
      args:                       # Args as list, NOT shell string
        - "-avz"
        - "/source/"
        - "/dest/"
      workdir: "/tmp"             # Optional working directory
      timeout: 300
```

**Security:**
- Absolute paths required for commands (prevent PATH injection)
- Individual arguments (no shell expansion)
- Path traversal validation (reject `..`)
- Timeout enforcement
- Output size limits
- Run as specific user/group (optional)

#### SSH Executor (`src/executors/ssh.rs`)

Remote command execution over SSH with connection pooling.

**Connection Pooling Design:**
```rust
use ssh2::Session;
use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct SshConnectionPool {
    pools: Arc<Mutex<HashMap<String, Vec<Session>>>>,
    max_per_host: usize,
}

impl SshConnectionPool {
    pub fn new(max_per_host: usize) -> Self {
        Self {
            pools: Arc::new(Mutex::new(HashMap::new())),
            max_per_host,
        }
    }

    pub async fn get_connection(&self, host: &str, user: &str, key_path: &Path) -> Result<Session> {
        let key = format!("{}@{}", user, host);
        let mut pools = self.pools.lock().await;

        // Try to reuse existing connection
        if let Some(pool) = pools.get_mut(&key) {
            if let Some(session) = pool.pop() {
                // Verify connection is still alive
                if session.authenticated() {
                    return Ok(session);
                }
            }
        }

        // Create new connection
        self.create_connection(host, user, key_path).await
    }

    pub async fn return_connection(&self, host: &str, user: &str, session: Session) {
        let key = format!("{}@{}", user, host);
        let mut pools = self.pools.lock().await;
        let pool = pools.entry(key).or_insert_with(Vec::new);

        if pool.len() < self.max_per_host {
            pool.push(session);
        }
        // Else drop connection (pool full)
    }

    async fn create_connection(&self, host: &str, user: &str, key_path: &Path) -> Result<Session> {
        let tcp = TcpStream::connect(format!("{}:22", host)).await?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;
        session.userauth_pubkey_file(user, None, key_path, None)?;

        if !session.authenticated() {
            return Err(anyhow!("SSH authentication failed for {}@{}", user, host));
        }

        Ok(session)
    }
}
```

**Memory Budget:**
- Pool size: 4 connections per host (matches max_parallel default)
- Each SSH session: ~1MB (ssh2 overhead + buffers)
- Total pool overhead: ~4MB (within memory budget)
- Connections timeout after 300s idle (configurable)

**Features:**
- Key-based authentication ONLY (no passwords)
- Connection pooling (reuse connections, max 4 per host)
- Health check before reuse (authenticated())
- Timeout per command
- Error propagation with context

**Security:**
- SSH agent forwarding disabled by default
- Host key verification (fail if unknown)
- Command parameterization (no injection)
- Key file permissions check (must be 0600)

#### HTTP Executor (`src/executors/http.rs`)

HTTP/HTTPS requests using `reqwest` with response size limits.

**Implementation with Streaming:**
```rust
pub async fn execute_http(config: &HttpConfig) -> Result<ExecutionResult> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout))
        .build()?;

    let response = client
        .request(config.method, &config.url)
        .json(&config.body)  // Optional JSON body
        .headers(config.headers.clone())
        .send()
        .await?;

    let status_code = response.status();

    // Stream response with size limit to prevent OOM
    let mut body = Vec::with_capacity(1024);
    let mut stream = response.bytes_stream();
    let mut truncated = false;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if body.len() + chunk.len() > MAX_RESPONSE_SIZE {
            truncated = true;
            break;
        }
        body.extend_from_slice(&chunk);
    }

    Ok(ExecutionResult {
        status: if status_code.is_success() { TaskStatus::Success } else { TaskStatus::Failed },
        stdout: Some(String::from_utf8_lossy(&body).to_string()),
        stderr: None,
        exit_code: Some(status_code.as_u16() as i32),
        duration: /* measure */,
        output_truncated: truncated,
    })
}
```

**Methods:**
- GET, POST, PUT, DELETE
- JSON body support (size-limited)
- Custom headers
- Timeout per request
- Response streaming with MAX_RESPONSE_SIZE limit (10MB)

**Success Criteria:**
- HTTP 2xx status codes = Success
- HTTP 4xx/5xx = Failed (with retry if configured)
- Response size exceeded = Failed with clear error

### 6. State Manager (`src/state.rs`)

SQLite-based persistence for task state and execution history.

**Schema with Performance Indexes:**
```sql
CREATE TABLE workflows (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE executions (
    id INTEGER PRIMARY KEY,
    workflow_id INTEGER NOT NULL,
    started_at TIMESTAMP NOT NULL,
    completed_at TIMESTAMP,
    status TEXT NOT NULL,
    FOREIGN KEY (workflow_id) REFERENCES workflows(id)
);

CREATE TABLE task_executions (
    id INTEGER PRIMARY KEY,
    execution_id INTEGER NOT NULL,
    task_name TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TIMESTAMP NOT NULL,
    completed_at TIMESTAMP,
    exit_code INTEGER,
    stdout TEXT,
    stderr TEXT,
    attempt INTEGER DEFAULT 1,
    retry_count INTEGER DEFAULT 0,
    next_retry_at TIMESTAMP,
    FOREIGN KEY (execution_id) REFERENCES executions(id)
);

CREATE TABLE retention_policy (
    workflow_name TEXT PRIMARY KEY,
    max_executions INTEGER DEFAULT 100,
    max_age_days INTEGER DEFAULT 30
);

-- Critical indexes for query performance
CREATE UNIQUE INDEX idx_workflows_name ON workflows(name);
CREATE INDEX idx_executions_workflow_started ON executions(workflow_id, started_at DESC);
CREATE INDEX idx_task_executions_status ON task_executions(status);
CREATE INDEX idx_task_executions_execution ON task_executions(execution_id);
CREATE INDEX idx_task_executions_started ON task_executions(started_at);
```

**SQLite Configuration for Edge Devices:**
```rust
use rusqlite::Connection;

pub fn create_connection(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;

    // Optimize for edge devices with limited resources
    conn.execute_batch("
        PRAGMA journal_mode = WAL;          -- Write-Ahead Logging for concurrency
        PRAGMA synchronous = NORMAL;        -- Balance safety vs performance
        PRAGMA cache_size = -2000;          -- 2MB cache (negative = KB)
        PRAGMA temp_store = MEMORY;         -- Temp tables in memory
        PRAGMA mmap_size = 0;               -- Disable mmap (safer on SD cards)
        PRAGMA foreign_keys = ON;           -- Enforce FK constraints
    ")?;

    Ok(conn)
}
```

**Operations:**
- Insert workflow execution start
- Update task status (Pending → Running → Success/Failed)
- Query execution history
- Cleanup old executions (retention policy)

### 7. Observability Layer

#### Logging (`src/logging.rs`)

Structured logging using `tracing` crate with memory-conscious configuration.

**Implementation:**
```rust
use tracing_subscriber::fmt::format::FmtSpan;

pub struct LogConfig {
    pub level: LogLevel,
    pub format: LogFormat,
}

pub fn init_logging(config: &LogConfig) -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_max_level(config.level)
        .with_span_events(FmtSpan::CLOSE)  // Only log on span close, reduce entries
        .with_writer(std::io::stderr)      // Write directly to stderr, no buffering
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
```

**Configuration:**
- JSON format for machine parsing
- Log levels: ERROR, WARN, INFO (default), DEBUG, TRACE
- Contextual fields: workflow_id, task_name, execution_id
- No buffering (direct write to reduce memory)
- Default: INFO level (not DEBUG, to save memory)

**Example:**
```json
{
  "timestamp": "2026-01-15T14:23:01.123Z",
  "level": "INFO",
  "message": "Task completed successfully",
  "workflow_id": "backup-workflow",
  "task_name": "backup_database",
  "execution_id": 42,
  "duration_ms": 1234
}
```

#### Metrics (`src/metrics.rs`)

Prometheus-compatible metrics endpoint (OPT-IN for edge devices).

**Configuration:**
```rust
pub struct ObservabilityConfig {
    pub enable_metrics: bool,       // Default: false (opt-in)
    pub metrics_port: u16,          // Default: 9090
    pub log_level: LogLevel,        // Default: INFO
}
```

**Key Metrics (when enabled):**
- `picoflow_task_executions_total{status="success|failed|timeout"}`
- `picoflow_task_duration_seconds{task_name="..."}`
- `picoflow_memory_usage_bytes`
- `picoflow_active_tasks`

**Endpoint:**
- HTTP server on `:9090/metrics` (configurable)
- **Disabled by default** on edge devices to save ~2MB memory
- Enable with `--enable-metrics` flag or `PICOFLOW_ENABLE_METRICS=true`

**Memory Impact:**
- Metrics disabled: 0MB
- Metrics enabled: ~2MB (histograms + HTTP server)

## Data Flow

### Workflow Execution Flow

1. **Parse**: YAML → `WorkflowConfig`
2. **Validate**: Check YAML structure, required fields
3. **Build DAG**: Tasks → directed graph
4. **Validate DAG**: Detect cycles, verify dependencies exist
5. **Sort**: Topological sort for execution order
6. **Schedule**: One-shot or cron-based
7. **Execute**: For each task in topological order:
   - Check dependencies (all succeeded?)
   - Select executor based on task type
   - Execute with retry logic
   - Update state in SQLite
   - Log result
8. **Complete**: Mark workflow execution finished

### Retry Flow

```
Task Failed
   ↓
Check retry count < max_retries?
   ↓ YES                    ↓ NO
Update status = Retrying    Mark as Failed
   ↓
Wait (exponential backoff)
   ↓
Execute again
```

## Parallelism Strategy

### Sequential Execution (Phase 1 MVP)

Tasks execute in topological order, one at a time.

**Pros:**
- Simple implementation
- Low memory footprint
- Predictable resource usage

**Cons:**
- Slower for independent tasks

### Parallel Execution (Phase 3)

Tasks at same DAG level execute concurrently.

**Example DAG:**
```
     A
    / \
   B   C    ← Level 1 (parallel)
    \ /
     D      ← Level 2 (after B & C)
```

**Execution:**
1. Run A (level 0)
2. Run B and C in parallel (level 1, max_parallel=2)
3. Wait for both B and C to complete
4. Run D (level 2)

**Implementation:**
- Use `tokio::spawn` for parallel tasks
- Use `join_all` to wait for level completion
- Respect global `max_parallel` limit

## Graceful Shutdown

Daemon mode handles SIGTERM/SIGINT signals for graceful shutdown.

**Implementation:**
```rust
use tokio::sync::broadcast;
use tokio::signal;

pub struct ShutdownHandler {
    shutdown_tx: broadcast::Sender<()>,
    max_shutdown_time: Duration,
}

impl ShutdownHandler {
    pub fn new(max_shutdown_time: Duration) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self { shutdown_tx, max_shutdown_time }
    }

    pub async fn wait_for_signal(&self) {
        tokio::select! {
            _ = signal::ctrl_c() => {
                tracing::info!("Received SIGINT, initiating graceful shutdown");
            }
            _ = Self::wait_sigterm() => {
                tracing::info!("Received SIGTERM, initiating graceful shutdown");
            }
        }

        // Signal all tasks to stop
        let _ = self.shutdown_tx.send(());

        // Wait for running tasks to finish
        tracing::info!("Waiting up to {:?} for tasks to complete", self.max_shutdown_time);
        tokio::time::sleep(self.max_shutdown_time).await;

        tracing::info!("Shutdown complete");
    }

    #[cfg(unix)]
    async fn wait_sigterm() {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).unwrap();
        term.recv().await;
    }
}
```

**Behavior:**
- SIGTERM/SIGINT triggers graceful shutdown
- Broadcast shutdown signal to all running tasks
- Wait up to 60 seconds (configurable) for tasks to complete
- Running tasks check shutdown signal and finish current operation
- SQLite state updated for incomplete tasks (marked as Failed)
- Exit cleanly with code 0

## Crash Recovery

On startup, check for crashed executions and handle appropriately.

**Implementation (MVP - Phase 1):**
```rust
pub async fn recover_from_crash(state: &StateManager) -> Result<()> {
    // Find executions that were Running when process died
    let crashed_executions = state.query(
        "SELECT * FROM executions WHERE status = 'Running'"
    )?;

    tracing::warn!("Found {} crashed executions", crashed_executions.len());

    for exec in crashed_executions {
        // Mark as Failed with crash context
        state.update_execution_status(exec.id, TaskStatus::Failed)?;
        state.add_execution_note(exec.id, "Process crashed during execution")?;

        tracing::warn!(
            execution_id = exec.id,
            workflow_id = exec.workflow_id,
            "Marked crashed execution as Failed"
        );
    }

    Ok(())
}
```

**Strategy:**
- **MVP (Phase 1)**: Mark crashed executions as Failed on startup
- **Future (v1.1)**: Resume execution from last checkpoint
  - Would require more complex state tracking (task-level checkpoints)
  - Idempotency requirements for tasks
  - Deferred to avoid complexity in MVP

**Crash Detection:**
- On startup, query `executions` table for status='Running'
- Any Running execution = crashed (process didn't exit cleanly)
- Mark as Failed and log for audit trail

## Security Considerations

### Input Validation

- All YAML inputs validated and sanitized
- No arbitrary code execution in YAML
- Task names must be alphanumeric + underscores

### Command Injection Prevention

- **Shell Executor**: Use `Command::args()`, never shell string interpolation
- **SSH Executor**: Parameterized commands, no shell expansion
- **HTTP Executor**: URL validation, header sanitization

### Secrets Management

- NO plaintext secrets in YAML
- Support environment variables: `${ENV_VAR}`
- Support file references: `file://path/to/secret`
- Future: HashiCorp Vault integration (v2.0)

### Isolation

- Run tasks as specific user/group (drop privileges)
- Working directory isolation
- Resource limits (CPU, memory via cgroups in future)

## Performance Optimizations

### Binary Size (<10MB target)

- `opt-level = "z"` in Cargo.toml
- Link-time optimization (LTO)
- Strip symbols
- `panic = "abort"` (remove unwinding overhead)
- Minimize dependencies (avoid bloat)

### Memory Footprint (<20MB idle)

**Memory Budget (Validated):**

| Component | Idle | 10 Parallel Tasks |
|-----------|------|-------------------|
| Tokio runtime | 5MB | 5MB |
| SQLite (WAL mode) | 2MB | 2MB |
| DAG engine | 1MB | 1MB |
| Task scheduler | 2MB | 2MB |
| SSH connection pool | 4MB | 4MB |
| HTTP client | 0.5MB | 0.5MB |
| Logging buffers | 1MB | 2MB |
| Task state tracking | 0.5MB | 5MB |
| Prometheus (disabled) | 0MB | 0MB |
| Task executor overhead | 0MB | 10MB |
| **TOTAL** | **16MB** | **31.5MB** |

**Target:** <20MB idle ✅ | <50MB with 10 tasks ✅

**Optimization Strategies:**
- Lazy initialization (don't load executors until needed)
- Connection pooling (SSH reuse, max 4 per host)
- SQLite optimized (2MB cache, WAL mode)
- Bounded task queues (backpressure via semaphore)
- Metrics disabled by default (saves 2MB)
- Output size limits (MAX_OUTPUT_SIZE = 10MB)

### Startup Latency (<100ms task startup)

- Fast YAML parsing (serde_yaml)
- Pre-compiled DAG validation
- Executor pool warmup
- Minimal async overhead

## Testing Strategy

### Unit Tests

Test each module in isolation:
- YAML parser with valid/invalid inputs
- DAG engine with cyclic/acyclic graphs
- Executors with mocked backends
- State manager with in-memory SQLite

### Integration Tests

End-to-end workflow execution:
- Real YAML files → execution
- Multi-task DAGs with dependencies
- Retry logic under failure injection
- SSH executor with local SSH server

### Benchmarks

Performance benchmarks using `criterion`:
- DAG parsing time (100, 1000 tasks)
- Task startup latency
- Memory usage under load

### Platform Testing

Test on actual target hardware:
- Raspberry Pi Zero 2 W
- ARM32 cross-compilation
- Memory profiling with `valgrind`

## Distribution & Packaging

### Homebrew (Primary — macOS & Linux)

PicoFlow is distributed via a Homebrew tap for macOS and Linux:

```bash
brew tap zoza1982/picoflow
brew install picoflow
```

The tap repository (`zoza1982/homebrew-picoflow`) contains a binary formula that downloads pre-built binaries from GitHub Releases. The formula is automatically updated by the `update-homebrew` job in the release workflow whenever a new version tag is pushed.

**Supported Homebrew platforms:**
- macOS Apple Silicon (ARM64)
- macOS Intel (x86_64)
- Linux x86_64, ARM64, ARM32 (via Linuxbrew)

### Pre-built Binaries

GitHub Releases provide `.tar.gz` archives with install scripts for all platforms (Linux ARM32/ARM64/x86_64, macOS ARM64/x86_64). Each archive includes the binary, an install script, SHA256 checksum, systemd service file (Linux), and example configuration.

### Release Workflow

The CI release pipeline (`.github/workflows/release.yml`) builds binaries for all five targets, packages them, creates a GitHub Release, and updates the Homebrew tap formula with new SHA256 checksums.

## Future Extensions (v2.0+)

- **Data Passing**: Task output → next task input (temp files, S3)
- **Conditional Execution**: If/else branches based on task results
- **Dynamic DAGs**: Generate tasks programmatically
- **Distributed Execution**: Multi-node orchestration
- **Web UI**: Optional dashboard for monitoring
- **Vault Integration**: Enterprise secrets management
- **Kubernetes Operator**: Deploy workflows as K8s CRDs

---

**Document Status:** Active  
**Last Updated:** November 11, 2025  
**Owner:** Zoran Vukmirica
