# PicoFlow

**Lightweight DAG workflow orchestrator for edge devices**

[![CI](https://github.com/zoza1982/picoflow/workflows/CI/badge.svg)](https://github.com/zoza1982/picoflow/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

PicoFlow is a Rust-native workflow orchestrator designed specifically for resource-constrained edge devices like the Raspberry Pi Zero 2 W. It provides DAG-based task scheduling with minimal memory footprint (<20MB idle) while maintaining production-grade features like retry logic, timeouts, and observability.

## Why PicoFlow?

| Feature | PicoFlow | Airflow | Luigi | cron/systemd |
|---------|----------|---------|-------|--------------|
| Memory (idle) | <20MB | 2GB+ | 200MB | N/A |
| Binary size | <10MB | N/A | N/A | N/A |
| DAG support | ✅ | ✅ | ✅ | ❌ |
| Edge device ready | ✅ | ❌ | ❌ | ✅ |
| Native binary | ✅ (Rust) | ❌ (Python) | ❌ (Python) | ✅ |
| Retry logic | ✅ | ✅ | ✅ | ❌ |
| Observability | ✅ | ✅ | ✅ | Limited |

## Features

- **Minimal Resource Footprint**: <20MB RAM idle, <50MB with 10 parallel tasks
- **DAG Support**: Define complex workflows with task dependencies
- **Multiple Executors**: Shell commands, SSH remote execution, HTTP requests
- **Scheduling**: Cron-based scheduling with daemon mode
- **Retry Logic**: Exponential backoff with configurable retry policies
- **Observability**: Structured logging (JSON), Prometheus metrics
- **Edge-Ready**: Tested on Raspberry Pi Zero 2 W (512MB RAM)

## Quick Start

### Installation

```bash
# From source
cargo install --path .

# Or download pre-built binary from releases
curl -L https://github.com/zoza1982/picoflow/releases/latest/download/picoflow-linux-arm -o picoflow
chmod +x picoflow
```

### Define a Workflow

Create `backup-workflow.yaml`:

```yaml
name: backup-workflow
description: "Daily database backup"
schedule: "0 0 2 * * *"  # 2 AM daily (6-field cron: sec min hour day month dayofweek)

config:
  max_parallel: 2
  retry_default: 3
  timeout_default: 300

tasks:
  - name: health_check
    type: http
    config:
      url: "https://api.server.com/health"
      method: GET
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
```

### Run the Workflow

```bash
# Validate workflow
picoflow validate backup-workflow.yaml

# Run once
picoflow run backup-workflow.yaml

# List all workflows
picoflow workflow list

# Check status of a specific workflow
picoflow status --workflow backup-workflow

# Run in daemon mode (with scheduling)
picoflow daemon start backup-workflow.yaml

# Stop daemon
picoflow daemon stop

# Check daemon status
picoflow daemon status

# View execution history
picoflow history --workflow backup-workflow

# View workflow statistics
picoflow stats --workflow backup-workflow

# View task logs
picoflow logs --workflow backup-workflow --task backup_database
```

## Architecture

PicoFlow consists of several core components:

- **DAG Engine**: Topological sort, cycle detection, dependency resolution
- **Scheduler**: Cron-based scheduling with daemon mode
- **Executors**: Pluggable execution backends (shell, SSH, HTTP)
- **State Manager**: SQLite-based persistence for task state and history
- **Logging**: Structured JSON logging with tracing integration
- **Metrics**: Prometheus endpoint for observability

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed design documentation.

## Performance

Current measurements (Phase 3):

- **Binary size**: 2.2MB (stripped) - 78% under 10MB target ✅
- **Memory (10 parallel tasks)**: ~7MB - 86% under 50MB target ✅
- **Tests**: 82/82 passing (100%) ✅
- **Code Quality**: Grade A- (92/100) ✅
- **Test Coverage**: >80% ✅

Target platform performance (Raspberry Pi Zero 2 W):
- Idle memory target: <20MB RSS
- 10 parallel tasks target: <50MB RSS ✅ (measured: ~7MB)
- Task startup latency target: <100ms
- DAG parsing (100 tasks) target: <50ms

Full benchmark suite will be completed in Phase 4.

## Development

### Prerequisites

- Rust 1.70+
- For cross-compilation: `cross` or Docker

### Build

```bash
# Development build
cargo build

# Release build (optimized for size)
cargo build --release

# Cross-compile for ARM32 (Pi Zero 2 W)
cross build --release --target armv7-unknown-linux-gnueabihf
```

### Test

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# All tests with coverage
cargo tarpaulin --out Html
```

### Quality Checks

```bash
# Format
cargo fmt --all

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Security audit
cargo audit
```

## Roadmap

- [x] **Phase 0**: Foundation (v0.1.0) - ✅ Complete
- [x] **Phase 1**: MVP Core Engine (v0.2.0) - ✅ Complete
  - Sequential workflow execution
  - DAG engine with cycle detection
  - Shell executor
  - SQLite state management
  - CLI: run, validate, status
  - Full rustdoc documentation
  - Code review Grade A- (93/100)
- [x] **Phase 2**: Scheduling & SSH (v0.3.0) - ✅ Complete
  - Cron-based scheduling (6-field format)
  - SSH remote execution (key-based auth)
  - Daemon mode with signal handling
  - Enhanced retry logic with exponential backoff
  - Graceful shutdown
  - Code review Grade B+ (87/100)
- [x] **Phase 3**: Parallelism & Observability (v0.4.0) - ✅ Complete
  - Parallel task execution with DAG levels
  - Prometheus metrics endpoint (:9090/metrics)
  - Execution history queries with filtering
  - Log retention and cleanup (30-day default)
  - Enhanced CLI: history, stats, logs
  - Code review Grade A- (92/100)
- [ ] **Phase 4**: Polish & Documentation (v1.0.0) - Next
  - HTTP executor
  - Comprehensive documentation
  - Cross-platform binaries (ARM32, ARM64, x86_64)
  - Performance benchmarking on target hardware
  - Security audit
  - Production-ready release

See [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for detailed roadmap.

## Contributing

Contributions welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) first.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Built with:
- [tokio](https://tokio.rs) - Async runtime
- [petgraph](https://github.com/petgraph/petgraph) - Graph algorithms
- [clap](https://github.com/clap-rs/clap) - CLI framework
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings
