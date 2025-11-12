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

Parses YAML files into strongly-typed Rust structs.

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

**Validation:**
- YAML syntax validation
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

Manages workflow execution lifecycle.

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

**Concurrency:**
- Global `max_parallel` limit (default: 4)
- Respect task dependencies (only run when deps complete)
- Use tokio task pool for parallel execution

### 5. Executor Interface (`src/executors/mod.rs`)

Trait-based pluggable execution backends.

```rust
#[async_trait]
pub trait ExecutorTrait: Send + Sync {
    async fn execute(&self, config: &TaskExecutorConfig) -> Result<ExecutionResult>;
    async fn health_check(&self) -> Result<()>;
}

pub struct ExecutionResult {
    pub status: TaskStatus,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration: Duration,
}
```

#### Shell Executor (`src/executors/shell.rs`)

Executes local shell commands using `tokio::process::Command`.

**Security:**
- Run as specific user/group (if configured)
- No shell string interpolation (prevent injection)
- Timeout enforcement
- Working directory isolation

#### SSH Executor (`src/executors/ssh.rs`)

Remote command execution over SSH.

**Features:**
- Key-based authentication ONLY (no passwords)
- Connection pooling (reuse connections)
- Timeout per command
- Error propagation with context

**Security:**
- SSH agent forwarding disabled by default
- Host key verification (fail if unknown)
- Command parameterization (no injection)

#### HTTP Executor (`src/executors/http.rs`)

HTTP/HTTPS requests using `reqwest`.

**Methods:**
- GET, POST, PUT, DELETE
- JSON body support
- Custom headers
- Timeout per request

**Success Criteria:**
- HTTP 2xx status codes = Success
- HTTP 4xx/5xx = Failed (with retry if configured)

### 6. State Manager (`src/state.rs`)

SQLite-based persistence for task state and execution history.

**Schema:**
```sql
CREATE TABLE workflows (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
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
    FOREIGN KEY (execution_id) REFERENCES executions(id)
);

CREATE INDEX idx_task_executions_status ON task_executions(status);
CREATE INDEX idx_task_executions_workflow ON task_executions(execution_id);
```

**Operations:**
- Insert workflow execution start
- Update task status (Pending → Running → Success/Failed)
- Query execution history
- Cleanup old executions (retention policy)

### 7. Observability Layer

#### Logging (`src/logging.rs`)

Structured logging using `tracing` crate.

**Configuration:**
- JSON format for machine parsing
- Log levels: ERROR, WARN, INFO, DEBUG, TRACE
- Contextual fields: workflow_id, task_name, execution_id

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

Prometheus-compatible metrics endpoint.

**Key Metrics:**
- `picoflow_task_executions_total{status="success|failed|timeout"}`
- `picoflow_task_duration_seconds{task_name="..."}`
- `picoflow_memory_usage_bytes`
- `picoflow_active_tasks`

**Endpoint:**
- HTTP server on `:9090/metrics` (configurable)

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

- Lazy initialization (don't load executors until needed)
- Connection pooling (SSH reuse)
- SQLite in-memory for temporary state
- Bounded task queues (backpressure)

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
