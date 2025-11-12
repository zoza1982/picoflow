# Claude Code Preferences - PicoFlow

## Project Context

**Product Name:** PicoFlow
**Description:** Lightweight DAG workflow orchestrator in Rust for edge devices
**Target Platform:** Raspberry Pi Zero 2 W (512MB RAM baseline), ARM/x86_64 Linux, macOS (dev)
**Language:** Rust
**Target Release:** v0.1.0 (MVP) - Q1 2026

## Session Initialization

**Run these commands at the start of every session:**
```bash
# 1. Configure git identity
git config user.name "Zoran Vukmirica"
git config user.email "zoza1982@users.noreply.github.com"

# 2. Add SSH key
ssh-add ~/.ssh/id_zoran_private

# 3. Check current branch
git branch --show-current

# 4. Check recent work
git log --oneline -10

# 5. Check current development phase
cat IMPLEMENTATION_PLAN.md | grep -A 5 "Phase"

# 6. Check for uncommitted work
git status
```

## Git Configuration

**Always use the following git identity:**
- **Name:** Zoran Vukmirica
- **Email:** zoza1982@users.noreply.github.com
- **SSH Key:** ~/.ssh/id_zoran_private

**Configure on every session:**
```bash
git config user.name "Zoran Vukmirica"
git config user.email "zoza1982@users.noreply.github.com"
ssh-add ~/.ssh/id_zoran_private
```

## Context Awareness

**Check status before starting work:**
- Run `git log --oneline -10` to see recent work
- Review PRD.md for requirements and priorities
- Check current phase in development roadmap (PRD.md lines 521-575)
- Review open tasks in current phase
- Mark tasks as completed when done (TodoWrite tool)

## Rust Development Standards

### Core Dependencies (from PRD.md)
```toml
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

### Code Quality Standards
- **Error Handling:** Use `anyhow::Result` for applications, `thiserror` for library errors
- **Async:** All I/O operations must be async with tokio
- **Logging:** Use `tracing` crate with structured logging (JSON format)
- **Testing:** Minimum 80% test coverage (unit + integration)
- **Documentation:** All public APIs must have rustdoc comments
- **Performance:** Always consider memory footprint (<20MB idle, <50MB with 10 tasks)

### Rust Best Practices
- Use `clippy` with strict lints: `cargo clippy -- -D warnings`
- Format with `rustfmt`: `cargo fmt --all`
- No unsafe code without explicit justification
- Prefer owned types over references in async contexts
- Use `Arc` for shared state across async tasks
- Implement `Debug`, `Clone`, `Serialize`, `Deserialize` where appropriate

## Agent Delegation

**ALWAYS delegate work to specialized agents:**
- **software-engineer**: Core Rust implementation, DAG engine, executors
- **systems-design-engineer**: Architecture decisions, system design, scalability
- **devops-engineer**: CI/CD, cross-compilation, Docker, deployment
- **security-engineer**: Security review, SSH authentication, secrets management
- **qa-engineer**: Test strategies, integration tests, edge cases
- **performance-tuning-engineer**: Memory optimization, benchmark analysis
- **code-reviewer**: Code review for every implementation

**Example:**
```
Task: Implement DAG topological sort
Action: @agent-software-engineer implement DAG validation and topological sort:
1. Use petgraph for graph operations
2. Detect cycles and fail fast
3. Return topologically sorted task list
4. Add comprehensive error handling
5. Target: <50ms for 100 tasks (PRD PERF-005)
```

## Quality Assurance Workflow

**After any implementation work is complete:**
1. **Code Review:** Engage `code-reviewer` agent to review for:
   - Rust best practices and idioms
   - Memory safety and performance
   - Error handling completeness
   - Security vulnerabilities (OWASP, SSH auth)
   - Binary size impact
2. **QA Validation:** Engage `qa-engineer` agent to:
   - Verify functionality
   - Test edge cases
   - Ensure test coverage targets met
3. **Performance Check:** For critical paths, engage `performance-tuning-engineer` to:
   - Verify memory footprint targets
   - Check task startup latency
   - Benchmark against PRD requirements
4. **Address Issues:** Fix all critical issues before considering work complete

## Definition of Done

**Work is only complete when ALL of these are true:**
- [ ] Code implemented by specialized agent
- [ ] Code reviewed by code-reviewer agent (no critical issues)
- [ ] Tests written and passing (80%+ coverage)
- [ ] Performance targets met (memory, latency per PRD Section 6.1)
- [ ] Security review passed (for executors, SSH, secrets)
- [ ] Documentation updated (rustdoc comments, README if needed)
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt` applied
- [ ] Commit created with conventional commit format
- [ ] Development phase tracker updated (PRD Section 10)

## Development Phases (from PRD.md)

### Phase 0: Foundation (Weeks 1-2)
- Project setup, CI/CD pipeline
- Core data models and types
- YAML parsing and validation
- SQLite schema design
- Basic CLI structure

### Phase 1: MVP - Core Engine (Weeks 3-6)
- DAG topological sort and validation
- Task state machine implementation
- Shell command executor
- Sequential execution engine
- File-based logging
- CLI: `run`, `validate`, `status`

### Phase 2: Scheduling & SSH (Weeks 7-9)
- Cron scheduler integration
- Daemon mode with signal handling
- SSH executor with key auth
- Retry logic with exponential backoff
- Task timeout implementation
- Graceful shutdown

### Phase 3: Parallelism & Observability (Weeks 10-12)
- Parallel task execution
- Configurable concurrency limits
- Execution history queries
- Log retention and cleanup
- Prometheus metrics endpoint
- Enhanced CLI: `logs`, `history`

### Phase 4: Polish & Documentation (Weeks 13-14)
- HTTP executor
- Comprehensive documentation
- Example workflows repository
- Performance benchmarking
- Security audit
- Release v1.0

## Performance Requirements (Critical)

**Always validate against these targets (PRD Section 6.1):**
- Binary size: <10MB (stripped)
- Runtime memory (idle): <20MB
- Runtime memory (10 parallel tasks): <50MB
- Task startup latency: <100ms
- DAG parsing time (100 tasks): <50ms
- Support 1000+ tasks per DAG

**How to measure:**
```bash
# Binary size
ls -lh target/release/picoflow | awk '{print $5}'

# Memory usage on target platform (Pi Zero 2 W)
# Run on actual hardware or QEMU ARM emulation
ps aux | grep picoflow | awk '{print $6}'  # RSS in KB
```

## Security Requirements (from PRD Section 6.3)

**CRITICAL security rules:**
- SSH: Key-based auth ONLY (no password support)
- Secrets: Environment variables or file refs (NO plaintext in YAML)
- User/group isolation: Run tasks as specific user
- Input validation: All user inputs must be sanitized
- Command injection: Never use shell string interpolation for commands

## Git Commit Guidelines

**NEVER include Claude Code attribution in commits:**
- ❌ Do NOT add: "🤖 Generated with Claude Code"
- ❌ Do NOT add: "Co-Authored-By: Claude <noreply@anthropic.com>"
- ✅ Keep commit messages clean and professional without AI attribution

## Commit Message Format

Use conventional commits format:
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `perf`: Performance improvement
- `refactor`: Code refactoring
- `test`: Test additions/changes
- `docs`: Documentation updates
- `build`: Build system changes
- `ci`: CI/CD changes

**Examples:**
```
feat(dag): implement topological sort with cycle detection

- Added petgraph-based DAG validation
- Detect cycles and return detailed error
- Performance: <50ms for 100 tasks
- Tests: 95% coverage

Implements: PRD Phase 1, DAG-002, DAG-003

perf(executor): reduce task startup latency to 80ms

- Optimized tokio runtime initialization
- Reduced memory allocations in hot path
- Target: <100ms (PRD PERF-004)

fix(ssh): prevent command injection in SSH executor

- Sanitize all command inputs
- Use parameterized execution
- Security review: PASSED
```

## Branch Naming

- Use descriptive branch names: `feature/`, `fix/`, `perf/`, `docs/`, `refactor/`
- Never work directly on `main` branch
- All work goes through branches and PRs
- Branch naming examples:
  - `feature/dag-engine`
  - `feature/ssh-executor`
  - `perf/reduce-binary-size`
  - `fix/cron-scheduler-crash`

## Testing Strategy

### Unit Tests
```bash
cargo test --lib
```
- Test all core logic in isolation
- Mock external dependencies (SSH, HTTP, filesystem)
- Target: >80% coverage

### Integration Tests
```bash
cargo test --test '*'
```
- Test end-to-end workflows
- Use real filesystem (temp directories)
- Test SQLite persistence
- Test YAML parsing with real files

### Performance Benchmarks
```bash
cargo bench
```
- Benchmark DAG parsing (target: <50ms for 100 tasks)
- Benchmark task startup (target: <100ms)
- Memory profiling with `valgrind` or `heaptrack`

### Target Platform Testing
```bash
# Cross-compile for ARM32 (Pi Zero 2 W)
cargo build --release --target armv7-unknown-linux-gnueabihf

# Test on actual hardware or QEMU
qemu-arm -L /usr/arm-linux-gnueabihf/ target/armv7-unknown-linux-gnueabihf/release/picoflow
```

## CI/CD Pipeline

**Required checks before merge:**
```bash
# 1. Format check
cargo fmt --all -- --check

# 2. Clippy (no warnings)
cargo clippy --all-targets --all-features -- -D warnings

# 3. Tests
cargo test --all-features

# 4. Build release binary
cargo build --release

# 5. Binary size check
test $(stat -f%z target/release/picoflow) -lt 10485760  # <10MB

# 6. Security audit
cargo audit
```

## Error Recovery

**When an agent produces errors or fails:**
1. Don't retry same agent immediately - analyze the error first
2. Check if issue is in prompt/requirements - clarify and retry
3. For Rust compilation errors:
   - Review error messages carefully (Rust errors are descriptive)
   - Check dependencies and features are correct
   - Verify async/await usage is correct
4. For performance issues:
   - Engage `performance-tuning-engineer` for analysis
   - Profile with `cargo flamegraph` or `perf`
5. Document the issue and workaround in commit message
6. Never push code that doesn't compile

## Documentation Updates

**Update these docs when relevant:**
- README.md: When adding CLI commands or major features
- PRD.md: When requirements change (rare, get approval first)
- ARCHITECTURE.md: When adding new components or changing design
- examples/: When adding new workflow examples
- Rustdoc: Always for public APIs

## Example Workflow (YAML) Reference

From PRD.md lines 276-319:
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

## Task State Machine (from PRD.md)

```rust
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

All task executors must implement proper state transitions.

## Priority Guidelines

**Use PRD priority levels when making decisions:**
- **P0 (Must Have):** Core functionality, blocking for v1.0
- **P1 (Should Have):** Important but not blocking
- **P2 (Nice to Have):** Future versions

When in conflict, always prioritize:
1. Performance targets (memory, latency)
2. Security requirements
3. P0 functional requirements
4. Code quality and maintainability

## Open Questions Tracking

**Reference PRD Section 11 for decisions:**
- Workflow parameterization: YES (env vars, P1 for v1.0)
- Secrets management: Env vars + file refs (v1.0), vault integration (v2.0)
- Task data passing: Env vars + temp files (P2 for v1.0)
- Web UI: NO for v1.0 (optional in v1.1)

## Session Startup Protocol

At the start of each session, automatically:

1. **Configure git identity:**
   ```bash
   git config user.name "Zoran Vukmirica"
   git config user.email "zoza1982@users.noreply.github.com"
   ssh-add ~/.ssh/id_zoran_private
   ```

2. **Check git status and history:**
   ```bash
   git status
   git log --oneline -10
   ```

3. **Identify current phase:**
   - Review PRD.md Section 10 (Development Roadmap)
   - Check which phase tasks are complete
   - Identify next task

4. **Announce current status** to user:
   ```
   PicoFlow Development Status:
   - Current Phase: [Phase N]
   - Last commit: [commit message]
   - In progress: [uncommitted files]
   - Next task: [task description]
   ```

5. **Wait for user confirmation** before proceeding with implementation

## Competitive Positioning

**Always keep in mind our unique value proposition:**
- Memory footprint: <20MB (vs Airflow 2GB+, Luigi 200MB)
- Edge device ready: Raspberry Pi Zero 2 W baseline
- Rust-native: No Python overhead, static binary
- DAG support: Unlike cron/systemd timers
- Production-grade: Retry logic, timeouts, observability

Reference PRD.md Section 13 Appendix A for competitive analysis.

---

**Document Status:** Active
**Last Updated:** November 11, 2025
**Project Phase:** Foundation (Phase 0)
**Owner:** Zoran Vukmirica
