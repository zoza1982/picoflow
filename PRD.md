# Product Requirements Document: PicoFlow

**Lightweight DAG Workflow Orchestrator in Rust**

---

## 1. Executive Summary

**Product Name:** PicoFlow

**Tagline:** "Enterprise-grade workflow orchestration for edge devices"

**Vision:** Enable infrastructure automation and workflow orchestration on resource-constrained devices with the performance and reliability of systems designed for high-end servers.

**Target Release:** v0.1.0 (MVP) - Q1 2026

---

## 2. Problem Statement

### Current Pain Points

**For edge/IoT infrastructure teams:**
- Modern workflow tools (Airflow, Prefect, Temporal) require 1-4GB RAM minimum
- Python-based tools have interpreter overhead and GC pauses
- No good options for running complex workflows on Raspberry Pi, edge devices, or minimal VMs
- Existing lightweight options lack DAG support, retry logic, or proper scheduling

**For infrastructure automation:**
- Need to orchestrate multi-step deployments, backups, health checks
- Tasks have dependencies and need proper ordering
- Require retry logic, error handling, and observability
- Want declarative configuration (YAML) not imperative code

**Market Gap:**
- Airflow: Too heavy (2GB+ RAM)
- Luigi: Python overhead, still ~200MB
- Cron: No DAG support
- Systemd timers: No dependency management
- **No Rust-native workflow orchestrator for edge computing**

---

## 3. Goals & Non-Goals

### Goals

**Primary:**
- Run comfortably on devices with 512MB RAM (Raspberry Pi Zero 2 W baseline)
- Support complex DAG workflows with dependencies
- Provide declarative YAML-based task definitions
- Enable infrastructure automation (SSH, HTTP APIs, shell commands)
- Achieve <10MB binary size, <20MB runtime memory footprint
- Support cron-style scheduling

**Secondary:**
- Parallel task execution where dependencies allow
- Retry logic with exponential backoff
- Task timeout controls
- Simple web UI for monitoring (optional, disabled by default)
- Metrics export (Prometheus format)
- Multiple backend executors (local shell, SSH, HTTP)

### Non-Goals

**V1 Explicitly Excludes:**
- Distributed execution across multiple nodes (future v2.0)
- Complex workflow patterns (dynamic DAGs, conditional branching - future)
- Built-in alerting/notifications (rely on external tools)
- Database backend (use filesystem + SQLite only)
- Python/JS task definitions (shell/binary only in v1)
- Web-based DAG editor (CLI only in v1)
- Multi-tenancy or user management

---

## 4. User Personas

### Primary: Edge Infrastructure Engineer

**Profile:**
- Name: Alex
- Role: DevOps Engineer managing 50+ Raspberry Pi devices for IoT fleet
- Pain: Can't use traditional orchestration tools on edge devices
- Needs: Automated backups, config updates, health monitoring across fleet
- Tech: Comfortable with YAML, shell scripts, SSH, basic Rust reading

### Secondary: Homelab Enthusiast

**Profile:**
- Name: Sam
- Role: Self-hoster running home automation on multiple Pis
- Pain: Complex cron jobs are hard to manage and debug
- Needs: Backup workflows, media processing pipelines, service health checks
- Tech: Proficient in Linux, Docker, basic scripting

### Tertiary: Embedded Systems Developer

**Profile:**
- Name: Jordan
- Role: Building commercial IoT products with automated testing/deployment
- Pain: CI/CD tools too heavy for embedded Linux devices
- Needs: Automated test suites, deployment pipelines on device
- Tech: Expert in embedded Linux, cross-compilation, resource optimization

---

## 5. Functional Requirements

### 5.1 Core DAG Engine

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| DAG-001 | Parse YAML workflow definitions | P0 | Single file or directory of files |
| DAG-002 | Validate DAG for cycles | P0 | Fail fast on invalid DAGs |
| DAG-003 | Execute tasks in topological order | P0 | Respect dependencies |
| DAG-004 | Support parallel execution of independent tasks | P1 | Configurable max parallelism |
| DAG-005 | Track task state (pending/running/success/failed) | P0 | Persist to disk |
| DAG-006 | Support task retries with configurable count | P0 | Per-task retry config |
| DAG-007 | Exponential backoff between retries | P1 | Configurable backoff multiplier |
| DAG-008 | Task timeout controls | P1 | Per-task timeout config |
| DAG-009 | Continue on failure mode (optional) | P2 | For cleanup tasks |

### 5.2 Task Executors

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| EXEC-001 | Shell command executor | P0 | Run local shell commands |
| EXEC-002 | SSH executor | P0 | Execute commands on remote hosts |
| EXEC-003 | HTTP executor | P1 | REST API calls with retries |
| EXEC-004 | Task environment variables | P1 | Pass context between tasks |
| EXEC-005 | Task output capture | P1 | Store stdout/stderr |
| EXEC-006 | Docker executor | P2 | Run tasks in containers (v1.1) |

### 5.3 Scheduling

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| SCHED-001 | Cron-style scheduling syntax | P0 | Standard cron expressions |
| SCHED-002 | One-time execution mode | P0 | Run DAG immediately and exit |
| SCHED-003 | Manual trigger via CLI | P0 | `picoflow run workflow.yaml` |
| SCHED-004 | Prevent overlapping runs | P1 | Configurable per workflow |
| SCHED-005 | Timezone support | P2 | Explicit timezone in cron specs |

### 5.4 Configuration & Storage

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| CFG-001 | YAML workflow definitions | P0 | Human-readable, version-controllable |
| CFG-002 | Global config file | P0 | Defaults, paths, resource limits |
| CFG-003 | SQLite for execution history | P0 | Lightweight, embedded |
| CFG-004 | Filesystem-based task logs | P0 | One file per task execution |
| CFG-005 | Configurable log retention | P1 | Auto-cleanup old logs |
| CFG-006 | Config validation command | P1 | `picoflow validate workflow.yaml` |

### 5.5 Observability & Monitoring

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| OBS-001 | Structured logging (JSON) | P0 | Machine-parseable |
| OBS-002 | CLI status command | P0 | Show running/recent workflows |
| OBS-003 | Prometheus metrics endpoint | P1 | Optional HTTP endpoint |
| OBS-004 | Task execution history query | P1 | CLI command to query SQLite |
| OBS-005 | Simple web UI (read-only) | P2 | View DAG, execution history |
| OBS-006 | Health check endpoint | P1 | For external monitoring |

### 5.6 CLI Interface

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| CLI-001 | `picoflow run <workflow>` | P0 | Execute workflow once |
| CLI-002 | `picoflow daemon` | P0 | Run scheduler in background |
| CLI-003 | `picoflow validate <workflow>` | P0 | Check workflow syntax |
| CLI-004 | `picoflow status` | P0 | Show running workflows |
| CLI-005 | `picoflow logs <workflow> <task>` | P1 | View task logs |
| CLI-006 | `picoflow history <workflow>` | P1 | Execution history |
| CLI-007 | `picoflow init` | P1 | Create example workflow |

---

## 6. Non-Functional Requirements

### 6.1 Performance

| ID | Requirement | Target | Priority |
|----|-------------|--------|----------|
| PERF-001 | Binary size | <10MB | P0 |
| PERF-002 | Runtime memory (idle) | <20MB | P0 |
| PERF-003 | Runtime memory (10 parallel tasks) | <50MB | P0 |
| PERF-004 | Task startup latency | <100ms | P1 |
| PERF-005 | DAG parsing time (100 tasks) | <50ms | P1 |
| PERF-006 | Support 1000+ tasks per DAG | Yes | P2 |

### 6.2 Reliability

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| REL-001 | Crash recovery | P0 | Resume from last checkpoint |
| REL-002 | Graceful shutdown (SIGTERM) | P0 | Complete running tasks |
| REL-003 | Persistent execution state | P0 | Survive process restarts |
| REL-004 | Atomic task state updates | P0 | No partial states |
| REL-005 | File lock for single instance | P1 | Prevent double execution |

### 6.3 Security

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| SEC-001 | SSH key-based auth only | P0 | No password support |
| SEC-002 | Secrets management | P1 | Env vars or file refs, no plaintext |
| SEC-003 | Read-only filesystem support | P2 | For immutable infrastructure |
| SEC-004 | User/group isolation | P1 | Run tasks as specific user |

### 6.4 Compatibility

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| COMPAT-001 | Linux ARM (32-bit) | P0 | Pi Zero 2 W baseline |
| COMPAT-002 | Linux ARM64 | P0 | Pi 4/5, modern SBCs |
| COMPAT-003 | Linux x86_64 | P0 | Standard servers |
| COMPAT-004 | macOS (dev only) | P1 | For development |
| COMPAT-005 | Static binary builds | P0 | No runtime dependencies |

---

## 7. Technical Architecture

### 7.1 System Components

```
┌─────────────────────────────────────────┐
│           CLI Interface                 │
│  (clap-based argument parsing)         │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│         Scheduler Service               │
│  (tokio-cron-scheduler)                │
│  - Cron parsing                        │
│  - Workflow triggering                 │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│          DAG Engine                     │
│  (petgraph for graph operations)       │
│  - Topological sort                    │
│  - Dependency resolution               │
│  - State management                    │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│        Task Executor Pool               │
│  (tokio async runtime)                 │
│  - Parallel execution                  │
│  - Retry logic                         │
│  - Timeout handling                    │
└────────────────┬────────────────────────┘
                 │
        ┌────────┴────────┐
        │                 │
┌───────▼─────┐  ┌────────▼────────┐
│   Shell     │  │   SSH Executor  │
│  Executor   │  │   (ssh2 crate)  │
└─────────────┘  └─────────────────┘
                          
┌─────────────────────────────────────────┐
│      Storage Layer                      │
│  - SQLite (rusqlite)                   │
│  - Filesystem logs                     │
│  - State persistence                   │
└─────────────────────────────────────────┘
```

### 7.2 Data Models

**Workflow Definition (YAML):**
```yaml
name: backup-workflow
description: "Daily backup with health check"
schedule: "0 2 * * *"  # 2 AM daily

config:
  max_parallel: 4
  retry_default: 3
  timeout_default: 300

tasks:
  - name: health_check
    type: http
    config:
      url: "https://api.server.com/health"
      method: GET
      timeout: 10
    retry: 2

  - name: backup_database
    type: ssh
    depends_on: [health_check]
    config:
      host: "db.server.com"
      command: "pg_dump mydb | gzip > /backup/db.sql.gz"
      user: backup
    retry: 3
    timeout: 600

  - name: verify_backup
    type: shell
    depends_on: [backup_database]
    config:
      command: "ssh backup@db.server.com 'test -f /backup/db.sql.gz'"
    retry: 1

  - name: cleanup_old_backups
    type: ssh
    depends_on: [verify_backup]
    config:
      host: "db.server.com"
      command: "find /backup -mtime +7 -delete"
    continue_on_failure: true
```

**Task Execution State:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskExecution {
    workflow_id: Uuid,
    task_name: String,
    status: TaskStatus,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    attempt: u32,
    exit_code: Option<i32>,
    stdout: Option<String>,
    stderr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum TaskStatus {
    Pending,
    Running,
    Success,
    Failed,
    Retrying,
    Timeout,
}
```

### 7.3 Core Dependencies

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# DAG & scheduling
petgraph = "0.6"
tokio-cron-scheduler = "0.9"

# Configuration
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
config = "0.14"

# CLI
clap = { version = "4", features = ["derive"] }

# Storage
rusqlite = { version = "0.31", features = ["bundled"] }

# Executors
ssh2 = "0.9"
reqwest = { version = "0.11", features = ["json"] }

# Logging & metrics
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }
prometheus = "0.13"

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
thiserror = "1"
```

---

## 8. User Stories & Acceptance Criteria

### Epic 1: Core DAG Execution

**Story 1.1: Simple Sequential Workflow**
```
As an infrastructure engineer
I want to define a workflow with dependent tasks
So that I can automate multi-step processes

Acceptance Criteria:
- [ ] Can define 3+ tasks in YAML with dependencies
- [ ] Tasks execute in correct order based on dependencies
- [ ] Failed task prevents downstream tasks from running
- [ ] Success status propagates through DAG
- [ ] Execution completes in <5 seconds for 5 simple tasks
```

**Story 1.2: Parallel Task Execution**
```
As a DevOps engineer
I want independent tasks to run in parallel
So that I can minimize total workflow execution time

Acceptance Criteria:
- [ ] Tasks with no dependencies start immediately
- [ ] Can configure max_parallel globally and per workflow
- [ ] Memory usage stays <50MB with 10 parallel tasks
- [ ] Dependent tasks wait for all parents to complete
```

### Epic 2: Task Executors

**Story 2.1: SSH Task Execution**
```
As a system administrator
I want to execute commands on remote servers via SSH
So that I can automate server maintenance

Acceptance Criteria:
- [ ] Can specify SSH host, user, command in task config
- [ ] Supports SSH key authentication
- [ ] Captures stdout/stderr from remote execution
- [ ] Returns proper exit codes
- [ ] Handles connection failures gracefully with retries
```

**Story 2.2: HTTP API Calls**
```
As an API integration developer
I want to call REST APIs as workflow tasks
So that I can orchestrate cloud resources

Acceptance Criteria:
- [ ] Supports GET, POST, PUT, DELETE methods
- [ ] Can send JSON payloads
- [ ] Configurable timeout per request
- [ ] Retry with exponential backoff on 5xx errors
- [ ] Success defined by 2xx status codes
```

### Epic 3: Scheduling

**Story 3.1: Cron Scheduling**
```
As a backup administrator
I want to schedule workflows using cron syntax
So that backups run automatically every night

Acceptance Criteria:
- [ ] Supports standard cron expressions (5 or 6 fields)
- [ ] Scheduler runs as daemon process
- [ ] Can schedule multiple workflows independently
- [ ] Prevents overlapping executions (optional flag)
- [ ] Survives process restarts (reloads schedules)
```

### Epic 4: Observability

**Story 4.1: Execution History**
```
As a DevOps engineer
I want to view workflow execution history
So that I can troubleshoot failures and track trends

Acceptance Criteria:
- [ ] CLI command shows last 10 executions per workflow
- [ ] Can query by date range, status, workflow name
- [ ] Shows task-level results with timing
- [ ] Can export to JSON for external analysis
- [ ] History retained for 30 days (configurable)
```

**Story 4.2: Real-time Monitoring**
```
As an operations team
I want to see currently running workflows
So that I can monitor system health

Acceptance Criteria:
- [ ] `picoflow status` shows active workflows
- [ ] Shows which tasks are running, pending, completed
- [ ] Updates in real-time (or with --watch flag)
- [ ] Shows resource usage (CPU, memory estimates)
- [ ] Prometheus metrics endpoint exposes key metrics
```

---

## 9. Success Metrics

### Primary KPIs

| Metric | Target | Measurement |
|--------|--------|-------------|
| Memory footprint (idle) | <20MB | RSS on Pi Zero 2 W |
| Memory footprint (10 tasks) | <50MB | Peak RSS during execution |
| Binary size | <10MB | Stripped release binary |
| Task startup overhead | <100ms | Time from trigger to exec |
| GitHub stars (6 months) | 500+ | Community interest |
| Production deployments | 100+ | Self-reported usage |

### Secondary Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Documentation completeness | 100% | All features documented |
| Test coverage | >80% | Unit + integration tests |
| Crash rate | <0.1% | Error tracking |
| Community contributions | 10+ PRs | GitHub activity |

---

## 10. Development Roadmap

### Phase 0: Foundation (Weeks 1-2)
- [ ] Project setup, CI/CD pipeline
- [ ] Core data models and types
- [ ] YAML parsing and validation
- [ ] SQLite schema design
- [ ] Basic CLI structure

### Phase 1: MVP - Core Engine (Weeks 3-6)
- [ ] DAG topological sort and validation
- [ ] Task state machine implementation
- [ ] Shell command executor
- [ ] Sequential execution engine
- [ ] File-based logging
- [ ] CLI: `run`, `validate`, `status`

**Deliverable:** Can run simple sequential workflows locally

### Phase 2: Scheduling & SSH (Weeks 7-9)
- [ ] Cron scheduler integration
- [ ] Daemon mode with signal handling
- [ ] SSH executor with key auth
- [ ] Retry logic with exponential backoff
- [ ] Task timeout implementation
- [ ] Graceful shutdown

**Deliverable:** Production-ready for basic infra automation

### Phase 3: Parallelism & Observability (Weeks 10-12)
- [ ] Parallel task execution
- [ ] Configurable concurrency limits
- [ ] Execution history queries
- [ ] Log retention and cleanup
- [ ] Prometheus metrics endpoint
- [ ] Enhanced CLI: `logs`, `history`

**Deliverable:** v1.0 release candidate

### Phase 4: Polish & Documentation (Weeks 13-14)
- [ ] HTTP executor
- [ ] Comprehensive documentation
- [ ] Example workflows repository
- [ ] Performance benchmarking
- [ ] Security audit
- [ ] Release v1.0

### Future (Post-v1.0)
- **v1.1:** Docker executor, web UI
- **v1.2:** Conditional execution, dynamic DAGs
- **v2.0:** Distributed execution, HA deployment
- **v2.1:** Python/JS task definitions via plugins

---

## 11. Open Questions & Risks

### Open Questions

1. **Should we support workflow parameterization?** (Pass variables at runtime)
   - Decision: Yes, via environment variables (P1 for v1.0)

2. **How to handle secrets management?**
   - Decision: Environment variables + file references, integrate with external tools (v1.0)
   - Future: Native integration with vault services (v2.0)

3. **Should tasks be able to pass data between them?**
   - Decision: Via environment variables + temp files (P2 for v1.0)
   - Future: Rich data passing with JSON (v1.1)

4. **Web UI required for v1.0?**
   - Decision: No, optional read-only UI for v1.1

### Risks & Mitigation

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Binary size bloat | High | Medium | Regular size audits, feature flags |
| SSH library limitations | Medium | Low | Evaluate alternatives (libssh) |
| Adoption in Python-heavy ecosystem | High | High | Excellent docs, migration guides |
| Performance on Pi Zero | High | Medium | Continuous benchmarking on target HW |
| Competing projects emerge | Medium | Medium | Fast iteration, unique value prop |

---

## 12. Go-to-Market Strategy

### Target Distribution Channels

1. **GitHub** - Primary open-source repository
2. **crates.io** - Rust package registry
3. **Homebrew** - macOS package manager
4. **APT/YUM repos** - Linux distributions
5. **Docker Hub** - Container images

### Documentation Strategy

- **Quick Start Guide** - 5-minute tutorial
- **Architecture Deep Dive** - For contributors
- **Workflow Examples** - 20+ real-world use cases
- **API Reference** - Generated from code
- **Migration Guides** - From cron, Luigi, Airflow

### Community Building

- Weekly office hours (video call)
- Active Discord/Slack community
- Monthly blog posts on use cases
- Conference talks at RustConf, KubeCon
- Integration showcase - Ansible, Terraform, etc.

---

## 13. Appendix

### A. Competitive Analysis

| Feature | PicoFlow | Airflow | Luigi | systemd | Cron |
|---------|----------|---------|-------|---------|------|
| Memory footprint | <20MB | 2GB+ | 200MB | <5MB | <1MB |
| DAG support | ✅ | ✅ | ✅ | ⚠️ | ❌ |
| Parallel execution | ✅ | ✅ | ✅ | ⚠️ | ❌ |
| Retry logic | ✅ | ✅ | ✅ | ⚠️ | ❌ |
| Web UI | v1.1 | ✅ | ✅ | ❌ | ❌ |
| Python-free | ✅ | ❌ | ❌ | ✅ | ✅ |
| Edge device ready | ✅ | ❌ | ❌ | ✅ | ✅ |

### B. Sample Benchmarks (Target)

**Test Environment:** Raspberry Pi Zero 2 W (512MB RAM, 1GHz quad-core)

```
Workflow: 10 tasks, 3 levels of dependencies, 4 SSH tasks

Memory Usage:
- Idle: 18MB
- During execution: 42MB
- Peak: 48MB

Timing:
- DAG parsing: 8ms
- Total execution: 45 seconds
- Overhead: <2%

Binary:
- Size: 8.2MB (stripped)
- Startup time: 120ms
```

### C. Example Workflows

See separate repository: `picoflow-examples`
- Raspberry Pi backup orchestration
- Multi-server deployment pipeline
- IoT data collection and processing
- Home automation routines
- Database maintenance tasks

---

**Document Status:** Draft v1.0  
**Last Updated:** November 11, 2025  
**Owner:** Open Source Community  
**Reviewers:** TBD

