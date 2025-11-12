# PicoFlow Implementation Plan

## Development Timeline

**Target Release:** v1.0.0 - Q1 2026 (14 weeks)  
**Baseline Platform:** Raspberry Pi Zero 2 W (512MB RAM, ARM Cortex-A53)

## Phase 0: Foundation (Weeks 1-2) ✅

**Goal:** Establish project infrastructure and core data models

### Tasks

- [x] Project initialization
  - [x] Git repository setup
  - [x] Cargo.toml with dependencies
  - [x] Directory structure (src/, tests/, benches/, examples/)
  - [x] CI/CD pipeline (GitHub Actions)
  
- [x] Development tooling
  - [x] rustfmt.toml configuration
  - [x] clippy.toml with strict lints
  - [x] .gitignore for Rust projects
  
- [x] Documentation foundation
  - [x] README.md with project overview
  - [x] ARCHITECTURE.md with design details
  - [x] IMPLEMENTATION_PLAN.md (this document)
  - [x] claude.md (Claude Code configuration)
  
- [ ] Core data models (`src/models.rs`)
  - [ ] `WorkflowConfig` struct with serde derives
  - [ ] `TaskConfig` with executor types
  - [ ] `TaskStatus` enum (Pending, Running, Success, Failed, Retrying, Timeout)
  - [ ] `ExecutionResult` struct
  - [ ] Unit tests for serialization/deserialization
  
- [ ] YAML parsing (`src/parser.rs`)
  - [ ] Parse YAML to `WorkflowConfig`
  - [ ] Validate required fields
  - [ ] Error handling with thiserror
  - [ ] Integration tests with sample YAML files
  
- [ ] SQLite schema design (`migrations/001_initial.sql`)
  - [ ] workflows table
  - [ ] executions table
  - [ ] task_executions table
  - [ ] Indexes for performance
  
- [ ] Basic CLI structure (`src/cli.rs`)
  - [ ] Clap-based argument parsing
  - [ ] Subcommands: run, validate, status
  - [ ] Help text and usage examples

**Exit Criteria:**
- ✅ Project compiles without errors
- ✅ Documentation complete and reviewed
- [ ] YAML parser works with test files
- [ ] SQLite schema migrations run successfully
- [ ] CLI `--help` displays correct usage

---

## Phase 1: MVP - Core Engine (Weeks 3-6)

**Goal:** Minimal viable workflow execution (sequential, local)

### Tasks

- [ ] DAG engine (`src/dag.rs`)
  - [ ] Build directed graph from task dependencies
  - [ ] Cycle detection (petgraph algorithms)
  - [ ] Topological sort for execution order
  - [ ] Error handling for invalid DAGs
  - [ ] Unit tests: acyclic, cyclic, disconnected graphs
  - [ ] Benchmark: <50ms for 100 tasks
  
- [ ] Task state machine (`src/state.rs`)
  - [ ] SQLite connection pool (rusqlite)
  - [ ] Insert workflow execution record
  - [ ] Update task status transitions
  - [ ] Query execution history
  - [ ] Unit tests with in-memory SQLite
  
- [ ] Shell command executor (`src/executors/shell.rs`)
  - [ ] Execute local commands with tokio::process
  - [ ] Capture stdout/stderr
  - [ ] Exit code handling
  - [ ] Timeout enforcement
  - [ ] Security: no shell string interpolation
  - [ ] Unit tests with mock commands
  
- [ ] Sequential execution engine (`src/scheduler.rs`)
  - [ ] Execute tasks in topological order
  - [ ] Wait for each task to complete
  - [ ] Update state after each task
  - [ ] Stop workflow on task failure (unless continue_on_failure)
  - [ ] Integration tests: multi-task workflows
  
- [ ] File-based logging (`src/logging.rs`)
  - [ ] Structured JSON logging (tracing crate)
  - [ ] Log to file and stdout
  - [ ] Log levels: INFO, WARN, ERROR
  - [ ] Contextual fields: workflow_id, task_name
  
- [ ] CLI commands
  - [ ] `picoflow run <workflow.yaml>`: Execute workflow once
  - [ ] `picoflow validate <workflow.yaml>`: Validate YAML and DAG
  - [ ] `picoflow status`: Show current execution status
  - [ ] Integration tests for CLI

**Exit Criteria:**
- [ ] Can execute simple 3-task workflow (A → B → C)
- [ ] DAG cycle detection works (reject cyclic graphs)
- [ ] Task failures are logged and workflow stops
- [ ] Execution history persisted in SQLite
- [ ] Binary size <10MB (stripped)
- [ ] Memory usage <20MB idle

---

## Phase 2: Scheduling & SSH (Weeks 7-9)

**Goal:** Production features - cron scheduling, remote execution, retry logic

### Tasks

- [ ] Cron scheduler (`src/scheduler.rs` extension)
  - [ ] Parse cron expressions (tokio-cron-scheduler)
  - [ ] Schedule workflow execution
  - [ ] Handle multiple scheduled workflows
  - [ ] Unit tests: cron expression parsing
  
- [ ] Daemon mode (`src/daemon.rs`)
  - [ ] Background process with PID file
  - [ ] Signal handling: SIGTERM (graceful shutdown), SIGHUP (reload config)
  - [ ] Graceful shutdown: finish running tasks
  - [ ] Integration tests: start, stop, reload
  
- [ ] SSH executor (`src/executors/ssh.rs`)
  - [ ] SSH connection with key-based auth (ssh2 crate)
  - [ ] Execute remote commands
  - [ ] Connection pooling (reuse connections)
  - [ ] Host key verification
  - [ ] Timeout per command
  - [ ] Security: prevent command injection
  - [ ] Unit tests with local SSH server (docker)
  
- [ ] Retry logic (`src/retry.rs`)
  - [ ] Exponential backoff algorithm
  - [ ] Configurable max retries per task
  - [ ] Update task status to "Retrying"
  - [ ] Log retry attempts
  - [ ] Unit tests: retry count, backoff delays
  
- [ ] Task timeout implementation
  - [ ] Enforce timeout per task
  - [ ] Kill task process on timeout
  - [ ] Mark task as "Timeout" status
  - [ ] Integration tests: timeout scenarios
  
- [ ] CLI extensions
  - [ ] `picoflow daemon <workflow.yaml>`: Run in background
  - [ ] `picoflow daemon stop`: Stop daemon gracefully
  - [ ] `picoflow daemon reload`: Reload configuration

**Exit Criteria:**
- [ ] Cron-scheduled workflow executes at correct times
- [ ] SSH executor runs remote commands successfully
- [ ] Retry logic works with exponential backoff
- [ ] Task timeout kills long-running tasks
- [ ] Daemon mode handles signals correctly (SIGTERM, SIGHUP)
- [ ] No memory leaks (test with 100 executions)

---

## Phase 3: Parallelism & Observability (Weeks 10-12)

**Goal:** Performance optimizations and monitoring

### Tasks

- [ ] Parallel task execution (`src/scheduler.rs` refactor)
  - [ ] Calculate DAG levels (tasks at same level can run in parallel)
  - [ ] Spawn tokio tasks for parallel execution
  - [ ] Respect `max_parallel` limit
  - [ ] Wait for level completion before next level
  - [ ] Unit tests: parallel vs sequential timing
  - [ ] Benchmark: 10 parallel tasks <50MB memory
  
- [ ] Configurable concurrency limits
  - [ ] Global `max_parallel` setting
  - [ ] Per-workflow concurrency override
  - [ ] Task pool management
  - [ ] Backpressure when limit reached
  
- [ ] Execution history queries (`src/state.rs` extension)
  - [ ] Query last N executions
  - [ ] Filter by workflow name
  - [ ] Filter by status (success/failed)
  - [ ] Aggregate statistics (success rate, avg duration)
  
- [ ] Log retention and cleanup (`src/logging.rs` extension)
  - [ ] Configurable retention period (days)
  - [ ] Background cleanup task
  - [ ] Delete old executions from SQLite
  - [ ] Rotate log files
  
- [ ] Prometheus metrics endpoint (`src/metrics.rs`)
  - [ ] HTTP server on :9090/metrics
  - [ ] Task execution counters (success/failed/timeout)
  - [ ] Task duration histograms
  - [ ] Memory usage gauge
  - [ ] Active tasks gauge
  - [ ] Integration tests: scrape metrics
  
- [ ] Enhanced CLI commands
  - [ ] `picoflow logs --workflow <name> --task <task>`: Query logs
  - [ ] `picoflow history --workflow <name>`: Show execution history
  - [ ] `picoflow stats --workflow <name>`: Aggregate statistics

**Exit Criteria:**
- [ ] Parallel execution works correctly (no race conditions)
- [ ] 10 parallel tasks consume <50MB memory
- [ ] Prometheus metrics endpoint responds correctly
- [ ] Log cleanup runs and removes old data
- [ ] CLI history command shows last 10 executions
- [ ] Task startup latency <100ms (benchmark)

---

## Phase 4: Polish & Documentation (Weeks 13-14)

**Goal:** Production-ready release with comprehensive docs

### Tasks

- [ ] HTTP executor (`src/executors/http.rs`)
  - [ ] HTTP/HTTPS requests (reqwest crate)
  - [ ] Methods: GET, POST, PUT, DELETE
  - [ ] JSON body support
  - [ ] Custom headers
  - [ ] Timeout per request
  - [ ] Success criteria: 2xx = success, 4xx/5xx = failed
  - [ ] Unit tests with mock HTTP server
  
- [ ] Comprehensive documentation
  - [ ] User guide (`docs/user-guide.md`)
  - [ ] API documentation (rustdoc comments)
  - [ ] Example workflows repository
  - [ ] Troubleshooting guide
  - [ ] FAQ
  
- [ ] Example workflows
  - [ ] examples/workflows/backup.yaml
  - [ ] examples/workflows/health-check.yaml
  - [ ] examples/workflows/data-pipeline.yaml
  - [ ] examples/workflows/parallel-tasks.yaml
  
- [ ] Performance benchmarking
  - [ ] Benchmark suite (criterion)
  - [ ] DAG parsing (100, 1000 tasks)
  - [ ] Task startup latency
  - [ ] Memory profiling
  - [ ] Test on Raspberry Pi Zero 2 W
  - [ ] Document results in docs/benchmarks.md
  
- [ ] Security audit
  - [ ] Review SSH executor (command injection)
  - [ ] Review shell executor (privilege escalation)
  - [ ] Review secrets handling
  - [ ] Dependency audit (`cargo audit`)
  - [ ] Address critical vulnerabilities
  
- [ ] Cross-compilation & packaging
  - [ ] Build for ARM32 (armv7-unknown-linux-gnueabihf)
  - [ ] Build for ARM64 (aarch64-unknown-linux-gnu)
  - [ ] Build for x86_64 (x86_64-unknown-linux-gnu)
  - [ ] Create release binaries
  - [ ] GitHub release with artifacts
  
- [ ] Final testing
  - [ ] End-to-end tests on target hardware
  - [ ] Stress testing (1000-task DAG)
  - [ ] Long-running daemon test (24h)
  - [ ] Memory leak detection

**Exit Criteria:**
- [ ] All PRD P0 requirements met
- [ ] Documentation complete and reviewed
- [ ] Performance targets met (binary <10MB, memory <20MB idle)
- [ ] Security audit passed with no critical issues
- [ ] Cross-compiled binaries for ARM32, ARM64, x86_64
- [ ] GitHub release published with binaries
- [ ] v1.0.0 tagged and released

---

## Testing Strategy

### Unit Tests

- **Target Coverage:** >80%
- **Tools:** `cargo test --lib`, `cargo tarpaulin`
- **Focus:**
  - Core logic (DAG, parser, state machine)
  - Executor implementations (mocked)
  - Retry logic edge cases

### Integration Tests

- **Location:** `tests/` directory
- **Tools:** `cargo test --test '*'`
- **Focus:**
  - End-to-end workflow execution
  - Multi-task DAGs with dependencies
  - Retry and timeout scenarios
  - CLI commands (golden test outputs)

### Benchmarks

- **Tool:** `criterion` crate
- **Location:** `benches/` directory
- **Focus:**
  - DAG parsing time (100, 1000 tasks)
  - Task startup latency
  - Memory usage under load

### Platform Testing

- **Hardware:** Raspberry Pi Zero 2 W
- **Method:**
  - Cross-compile for ARM32
  - Deploy to device via SCP
  - Run integration tests on device
  - Memory profiling with `ps aux`

---

## Performance Targets (Critical)

| Metric | Target | Measured |
|--------|--------|----------|
| Binary size (stripped) | <10MB | TBD |
| Memory (idle) | <20MB | TBD |
| Memory (10 parallel tasks) | <50MB | TBD |
| Task startup latency | <100ms | TBD |
| DAG parsing (100 tasks) | <50ms | TBD |
| DAG parsing (1000 tasks) | <500ms | TBD |

**Measurement Plan:**
- Binary size: `ls -lh target/release/picoflow`
- Memory: `ps aux | grep picoflow` on Pi Zero 2 W
- Latency: `criterion` benchmarks
- DAG parsing: `criterion` benchmarks

---

## Risk Mitigation

### Risk: Binary size exceeds 10MB

**Mitigation:**
- Use `opt-level = "z"` and LTO in Cargo.toml
- Minimize dependencies (avoid bloat)
- Strip symbols: `strip = true`
- Benchmark after each dependency addition

### Risk: Memory footprint exceeds 20MB idle

**Mitigation:**
- Lazy initialization (don't load all executors)
- Use `Arc` for shared state, not clones
- SQLite in-memory for temp state
- Profile with `valgrind` or `heaptrack`

### Risk: Task startup latency >100ms

**Mitigation:**
- Pre-compile executors (avoid lazy init)
- Optimize tokio runtime config
- Minimize allocations in hot path
- Benchmark and profile with `perf`

### Risk: Security vulnerabilities in SSH/shell executors

**Mitigation:**
- Use parameterized commands (no string interpolation)
- Run security audit (`cargo audit`)
- Engage security-engineer agent for review
- Fuzz testing for input validation

---

## Dependencies Tracking

### Phase 0 (Foundation)
- tokio, serde, serde_yaml, clap, rusqlite, anyhow, thiserror

### Phase 1 (MVP)
- petgraph, tracing, tracing-subscriber

### Phase 2 (Scheduling & SSH)
- tokio-cron-scheduler, ssh2

### Phase 3 (Observability)
- prometheus

### Phase 4 (HTTP Executor)
- reqwest

**Total Dependencies:** ~15 direct deps (keep minimal)

---

## Git Branching Strategy

- `main`: Production-ready code
- `develop`: Integration branch for features
- `feature/*`: Feature branches (e.g., `feature/dag-engine`)
- `fix/*`: Bug fix branches
- `perf/*`: Performance optimization branches

**Workflow:**
1. Create feature branch from `develop`
2. Implement feature with tests
3. Code review by `code-reviewer` agent
4. Merge to `develop` via PR
5. After phase complete, merge `develop` → `main`

---

## Release Plan

### v0.1.0 (Phase 0 Complete)
- Project foundation
- Documentation skeleton
- No functional code yet

### v0.2.0 (Phase 1 Complete)
- MVP core engine
- Sequential execution
- Shell executor
- Basic CLI

### v0.3.0 (Phase 2 Complete)
- Cron scheduling
- SSH executor
- Retry logic
- Daemon mode

### v0.4.0 (Phase 3 Complete)
- Parallel execution
- Prometheus metrics
- Enhanced CLI

### v1.0.0 (Phase 4 Complete)
- HTTP executor
- Full documentation
- Cross-platform binaries
- Production-ready

---

**Document Status:** Active  
**Current Phase:** Phase 0 (Foundation) - In Progress  
**Last Updated:** November 11, 2025  
**Owner:** Zoran Vukmirica
