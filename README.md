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

## Documentation

Comprehensive documentation is available:

- **[User Guide](docs/user-guide.md)** - Complete user documentation with tutorials
- **[API Reference](docs/api-reference.md)** - YAML schema, CLI commands, and configuration
- **[Troubleshooting](docs/troubleshooting.md)** - Common issues and solutions
- **[FAQ](docs/faq.md)** - Frequently asked questions
- **[Contributing Guide](CONTRIBUTING.md)** - How to contribute to PicoFlow
- **[Architecture](ARCHITECTURE.md)** - System design and technical details

## Performance

Current measurements (v1.0):

- **Binary size**: 3.0MB (stripped) - 70% under 10MB target ✅
- **Memory (idle)**: <20MB RSS ✅
- **Memory (10 parallel tasks)**: <50MB RSS ✅
- **Tests**: 100% passing ✅
- **Code Quality**: Grade A- (92/100) ✅
- **Test Coverage**: >80% ✅

Target platform: Raspberry Pi Zero 2 W (512MB RAM)
- Task startup latency: <100ms
- DAG parsing (100 tasks): <50ms
- Supports 1000+ tasks per DAG

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
- [x] **Phase 2**: Scheduling & SSH (v0.3.0) - ✅ Complete
  - Cron-based scheduling (6-field format)
  - SSH remote execution (key-based auth)
  - Daemon mode with signal handling
  - Retry logic with exponential backoff
  - Graceful shutdown
- [x] **Phase 3**: Parallelism & Observability (v0.4.0) - ✅ Complete
  - Parallel task execution
  - Prometheus metrics endpoint
  - Execution history queries
  - Enhanced CLI: history, stats, logs
- [x] **Phase 4**: Polish & Documentation (v1.0.0) - ✅ Complete
  - HTTP executor
  - Comprehensive documentation
  - Production-ready release

**What's Next:**
- **v1.1** (Q2 2026): Docker executor, Web UI, REST API
- **v1.2** (Q3 2026): Conditional execution, loops, templating
- **v2.0** (Q4 2026): Distributed multi-node execution

See [PRD.md](PRD.md) for detailed roadmap and [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for technical milestones.

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
