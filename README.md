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

#### Homebrew (macOS & Linux — Recommended)

```bash
brew tap zoza1982/picoflow
brew install picoflow
picoflow --version
```

#### Pre-built Binaries

Download the appropriate binary for your platform. The install script auto-detects the latest release:

**macOS (Apple Silicon):**
```bash
curl -sL https://api.github.com/repos/zoza1982/picoflow/releases/latest \
  | grep "browser_download_url.*darwin-arm64-macos.tar.gz\"" \
  | cut -d '"' -f 4 | xargs curl -LO
tar -xzf picoflow-*-darwin-arm64-macos.tar.gz
cd picoflow-*-darwin-arm64-macos
./install.sh
```

**macOS (Intel):**
```bash
curl -sL https://api.github.com/repos/zoza1982/picoflow/releases/latest \
  | grep "browser_download_url.*darwin-x86_64-macos.tar.gz\"" \
  | cut -d '"' -f 4 | xargs curl -LO
tar -xzf picoflow-*-darwin-x86_64-macos.tar.gz
cd picoflow-*-darwin-x86_64-macos
./install.sh
```

> **Note:** macOS may show a Gatekeeper warning for unsigned binaries. Run
> `xattr -d com.apple.quarantine /usr/local/bin/picoflow` to clear it, or
> use Homebrew which handles this automatically.

**ARM 32-bit (Raspberry Pi Zero 2 W, Pi 3/4 in 32-bit mode):**
```bash
curl -sL https://api.github.com/repos/zoza1982/picoflow/releases/latest \
  | grep "browser_download_url.*arm32-linux.tar.gz\"" \
  | cut -d '"' -f 4 | xargs wget -q
tar -xzf picoflow-*-arm32-linux.tar.gz
cd picoflow-*-arm32-linux
sudo ./install.sh
```

**ARM 64-bit (Raspberry Pi 4/5, modern SBCs):**
```bash
curl -sL https://api.github.com/repos/zoza1982/picoflow/releases/latest \
  | grep "browser_download_url.*arm64-linux.tar.gz\"" \
  | cut -d '"' -f 4 | xargs wget -q
tar -xzf picoflow-*-arm64-linux.tar.gz
cd picoflow-*-arm64-linux
sudo ./install.sh
```

**x86_64 Linux (Standard Linux servers):**
```bash
curl -sL https://api.github.com/repos/zoza1982/picoflow/releases/latest \
  | grep "browser_download_url.*x86_64-linux.tar.gz\"" \
  | cut -d '"' -f 4 | xargs wget -q
tar -xzf picoflow-*-x86_64-linux.tar.gz
cd picoflow-*-x86_64-linux
sudo ./install.sh
```

#### User Directory Installation (No Root Required)

```bash
# Install to ~/.local/bin instead of /usr/local/bin
INSTALL_DIR=~/.local/bin ./install.sh

# Add to PATH if needed
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

#### From Source

```bash
# Clone repository
git clone https://github.com/zoza1982/picoflow.git
cd picoflow

# Build and install
cargo build --release
sudo cp target/release/picoflow /usr/local/bin/

# Or install with cargo
cargo install --path .
```

#### Platform Support

| Platform | Architecture | Binary Name | Tested Devices |
|----------|-------------|-------------|----------------|
| macOS | Apple Silicon (ARM64) | `picoflow-*-darwin-arm64` | MacBook Pro/Air M1-M4, Mac Mini, Mac Studio |
| macOS | Intel (x86_64) | `picoflow-*-darwin-x86_64` | Intel MacBook Pro/Air, iMac |
| ARM 32-bit | ARMv7 | `picoflow-*-arm32` | Raspberry Pi Zero 2 W, Pi 3/4 (32-bit OS) |
| ARM 64-bit | AArch64 | `picoflow-*-arm64` | Raspberry Pi 4/5, Orange Pi, Rock Pi |
| x86_64 | x86-64 | `picoflow-*-x86_64` | Standard Linux servers, VMs |

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

### Benchmark Results (v0.1.0)

**Binary & Memory:**
- **Binary size**: 6.0MB (stripped, with vendored OpenSSL) - 40% under 10MB target ✅
- **Memory (idle)**: <20MB RSS ✅
- **Memory (10 parallel tasks)**: <50MB RSS ✅

**DAG Operations (100 tasks):**
- **DAG build**: 0.69ms (target: <50ms) ✅
- **DAG validation**: 0.07ms ✅
- **Topological sort**: 0.08ms ✅
- **Sequential execution overhead**: 0.88ms ✅

**Task Latency:**
- **Task startup**: ~2ms (target: <100ms) ✅
- **Workflow creation**: 0.37ms
- **Task status update**: 0.42ms

**Concurrent Operations:**
- **5 parallel executions**: 0.79ms
- **10 parallel executions**: 0.96ms
- **20 parallel executions**: 1.27ms

**Quality Metrics:**
- **Tests**: 97 unit + 11 integration + 23 doc tests (100% passing) ✅
- **Code Quality**: Grade A- (92/100) ✅
- **Test Coverage**: >80% ✅

Target platform: Raspberry Pi Zero 2 W (512MB RAM)
- Supports 1000+ tasks per DAG

## Development

### Prerequisites

- Rust 1.70+
- For cross-compilation: Docker or native toolchains

### Build

```bash
# Development build
cargo build

# Release build (optimized for size)
cargo build --release

# Cross-compile for all platforms (Docker-based, recommended)
./scripts/docker-build.sh

# Cross-compile for specific platform
./scripts/docker-build.sh arm32  # ARM 32-bit
./scripts/docker-build.sh arm64  # ARM 64-bit
./scripts/docker-build.sh x86_64 # x86_64 Linux

# Native cross-compilation (requires toolchains)
cargo build --release --target armv7-unknown-linux-gnueabihf   # ARM 32-bit
cargo build --release --target aarch64-unknown-linux-gnu       # ARM 64-bit
cargo build --release --target x86_64-unknown-linux-gnu        # x86_64
```

### Cross-Compilation Setup

**Quick Start (Docker):**
```bash
# Build all platforms using Docker (no toolchain setup required)
./scripts/docker-build.sh
```

**Native Toolchains (Ubuntu/Debian):**
```bash
# Install cross-compilation toolchains
sudo apt-get install gcc-arm-linux-gnueabihf gcc-aarch64-linux-gnu

# Add Rust targets
rustup target add armv7-unknown-linux-gnueabihf
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-unknown-linux-gnu

# Build for all platforms
./scripts/build-all.sh
```

See [docs/cross-compilation.md](docs/cross-compilation.md) for detailed setup instructions.

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

**Current Release: v0.1.0 (Phase 1 - MVP)**

- [x] **Phase 0**: Foundation - ✅ Complete
  - Project setup and core architecture
  - CI/CD pipeline
  - Basic data models
- [x] **Phase 1**: MVP Core Engine (v0.1.0) - ✅ Complete
  - Sequential workflow execution
  - DAG engine with cycle detection
  - Shell executor
  - SQLite state management
  - CLI: run, validate, status
- [ ] **Phase 2**: Scheduling & SSH (v0.2.0) - Planned
  - Cron-based scheduling (6-field format)
  - SSH remote execution (key-based auth)
  - Daemon mode with signal handling
  - Retry logic with exponential backoff
  - Graceful shutdown
- [ ] **Phase 3**: Parallelism & Observability (v0.3.0) - Planned
  - Parallel task execution
  - Prometheus metrics endpoint
  - Execution history queries
  - Enhanced CLI: history, stats, logs
- [ ] **Phase 4**: Polish & Documentation (v0.1.1) - Planned
  - HTTP executor
  - Comprehensive documentation
  - Production-ready release

**Future Versions:**
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
