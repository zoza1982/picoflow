# Cross-Compilation Guide

This guide covers building PicoFlow for multiple platforms, including ARM-based edge devices like Raspberry Pi.

## Table of Contents

1. [Supported Platforms](#supported-platforms)
2. [Quick Start](#quick-start)
3. [Prerequisites](#prerequisites)
4. [Building for All Platforms](#building-for-all-platforms)
5. [Building for Specific Platforms](#building-for-specific-platforms)
6. [Docker-Based Builds](#docker-based-builds)
7. [Testing Binaries](#testing-binaries)
8. [Troubleshooting](#troubleshooting)
9. [CI/CD Integration](#cicd-integration)

## Supported Platforms

PicoFlow supports the following target platforms:

| Platform | Target Triple | Use Case |
|----------|--------------|----------|
| **ARM 32-bit** | `armv7-unknown-linux-gnueabihf` | Raspberry Pi Zero 2 W, Pi 3/4 (32-bit) |
| **ARM 64-bit** | `aarch64-unknown-linux-gnu` | Raspberry Pi 4/5, Modern SBCs |
| **x86_64** | `x86_64-unknown-linux-gnu` | Standard Linux servers, development |

## Quick Start

### Using Docker (Recommended)

The easiest way to cross-compile for all platforms:

```bash
# Build for all platforms
./scripts/docker-build.sh

# Build for specific platform
./scripts/docker-build.sh arm32
./scripts/docker-build.sh arm64
./scripts/docker-build.sh x86_64
```

Binaries will be in `target/release-binaries/`.

### Using Native Toolchains

If you have cross-compilation toolchains installed:

```bash
# Build for all platforms
./scripts/build-all.sh

# Build release binaries with version info
./scripts/build-release.sh v1.0.0

# Package for distribution
./scripts/package-release.sh v1.0.0
```

## Prerequisites

### Rust Toolchain

Install Rust and add target platforms:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add target platforms
rustup target add armv7-unknown-linux-gnueabihf
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-unknown-linux-gnu
```

### Cross-Compilation Toolchains

#### Ubuntu/Debian

```bash
sudo apt-get update
sudo apt-get install -y \
    gcc-arm-linux-gnueabihf \
    g++-arm-linux-gnueabihf \
    gcc-aarch64-linux-gnu \
    g++-aarch64-linux-gnu \
    pkg-config \
    libssl-dev
```

#### macOS

```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install cross-compilation tools
brew tap messense/macos-cross-toolchains
brew install arm-unknown-linux-gnueabihf
brew install aarch64-unknown-linux-gnu
```

#### Fedora/RHEL

```bash
sudo dnf install -y \
    gcc-arm-linux-gnu \
    gcc-aarch64-linux-gnu \
    openssl-devel
```

### Docker (Alternative)

If you prefer not to install native toolchains:

```bash
# Install Docker
# Ubuntu/Debian
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# macOS
brew install --cask docker

# Verify installation
docker --version
```

## Building for All Platforms

### Method 1: Build Script

```bash
# Build debug binaries (faster, larger)
./scripts/build-all.sh --debug

# Build release binaries (optimized for size)
./scripts/build-all.sh

# Build with verbose output
./scripts/build-all.sh --verbose
```

Output:
```
target/
  armv7-unknown-linux-gnueabihf/release/picoflow
  aarch64-unknown-linux-gnu/release/picoflow
  x86_64-unknown-linux-gnu/release/picoflow
```

### Method 2: Cargo Directly

```bash
# ARM 32-bit
cargo build --release --target armv7-unknown-linux-gnueabihf

# ARM 64-bit
cargo build --release --target aarch64-unknown-linux-gnu

# x86_64
cargo build --release --target x86_64-unknown-linux-gnu
```

### Method 3: Docker Build

```bash
# Build Docker image (one-time setup)
docker build -f Dockerfile.build -t picoflow-builder .

# Build for all platforms
./scripts/docker-build.sh

# Or run Docker directly
docker run --rm -v $(pwd):/workspace picoflow-builder
```

## Building for Specific Platforms

### ARM 32-bit (Raspberry Pi Zero 2 W)

```bash
# Using script
./scripts/build-all.sh

# Using cargo
cargo build --release --target armv7-unknown-linux-gnueabihf

# Using Docker
./scripts/docker-build.sh arm32
```

Binary location: `target/armv7-unknown-linux-gnueabihf/release/picoflow`

### ARM 64-bit (Raspberry Pi 4/5)

```bash
# Using script
./scripts/build-all.sh

# Using cargo
cargo build --release --target aarch64-unknown-linux-gnu

# Using Docker
./scripts/docker-build.sh arm64
```

Binary location: `target/aarch64-unknown-linux-gnu/release/picoflow`

### x86_64 Linux

```bash
# Using script
./scripts/build-all.sh

# Using cargo
cargo build --release --target x86_64-unknown-linux-gnu

# Using Docker
./scripts/docker-build.sh x86_64
```

Binary location: `target/x86_64-unknown-linux-gnu/release/picoflow`

## Docker-Based Builds

### Building the Docker Image

```bash
docker build -f Dockerfile.build -t picoflow-builder .
```

### Using the Build Image

```bash
# Build all platforms
docker run --rm -v $(pwd):/workspace picoflow-builder

# Build specific platform
docker run --rm -v $(pwd):/workspace picoflow-builder arm32
docker run --rm -v $(pwd):/workspace picoflow-builder arm64
docker run --rm -v $(pwd):/workspace picoflow-builder x86_64
```

### Caching for Faster Builds

```bash
# Create a persistent cargo cache volume
docker volume create picoflow-cargo-cache

# Use cache in builds
docker run --rm \
    -v $(pwd):/workspace \
    -v picoflow-cargo-cache:/root/.cargo \
    picoflow-builder
```

## Testing Binaries

### On Native Platform (x86_64)

```bash
# Test the binary
./target/x86_64-unknown-linux-gnu/release/picoflow --version

# Run smoke test
./target/x86_64-unknown-linux-gnu/release/picoflow validate examples/hello-world.yaml
```

### On Target Hardware (Raspberry Pi)

Copy the binary to your Raspberry Pi:

```bash
# ARM 32-bit (Pi Zero 2 W)
scp target/armv7-unknown-linux-gnueabihf/release/picoflow pi@raspberrypi:~/

# ARM 64-bit (Pi 4/5)
scp target/aarch64-unknown-linux-gnu/release/picoflow pi@raspberrypi:~/
```

Test on the device:

```bash
ssh pi@raspberrypi
./picoflow --version
./picoflow validate my-workflow.yaml
```

### Using QEMU for Testing

Install QEMU user emulation:

```bash
# Ubuntu/Debian
sudo apt-get install qemu-user-static

# macOS
brew install qemu
```

Test ARM binaries:

```bash
# ARM 32-bit
qemu-arm-static -L /usr/arm-linux-gnueabihf/ \
    target/armv7-unknown-linux-gnueabihf/release/picoflow --version

# ARM 64-bit
qemu-aarch64-static -L /usr/aarch64-linux-gnu/ \
    target/aarch64-unknown-linux-gnu/release/picoflow --version
```

## Release Builds

### Create Release Binaries

```bash
# Build optimized binaries with version info
./scripts/build-release.sh v1.0.0

# Binaries will be in: target/release-binaries/
# - picoflow-v1.0.0-arm32
# - picoflow-v1.0.0-arm64
# - picoflow-v1.0.0-x86_64
```

### Package for Distribution

```bash
# Create installation packages
./scripts/package-release.sh v1.0.0

# Packages will be in: target/packages/
# - picoflow-v1.0.0-arm32-linux.tar.gz
# - picoflow-v1.0.0-arm64-linux.tar.gz
# - picoflow-v1.0.0-x86_64-linux.tar.gz
# - *.sha256 (checksums)
```

## Troubleshooting

### Missing Linker Errors

**Error:**
```
error: linker `arm-linux-gnueabihf-gcc` not found
```

**Solution:**
```bash
# Ubuntu/Debian
sudo apt-get install gcc-arm-linux-gnueabihf

# Or use Docker builds
./scripts/docker-build.sh
```

### OpenSSL Linking Errors

**Error:**
```
could not find native static library `ssl`
```

**Solution:**
```bash
# Install OpenSSL development libraries
sudo apt-get install libssl-dev pkg-config

# Or enable vendored OpenSSL in Cargo.toml
# Add to dependencies:
# openssl = { version = "0.10", features = ["vendored"] }
```

### Binary Size Too Large

**Check binary size:**
```bash
ls -lh target/*/release/picoflow
```

**Optimize further:**
```bash
# Strip debug symbols manually
strip target/armv7-unknown-linux-gnueabihf/release/picoflow

# Use UPX compression (optional)
upx --best --lzma target/armv7-unknown-linux-gnueabihf/release/picoflow
```

**Verify optimization settings** in `.cargo/config.toml`:
```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = "fat"         # Link-time optimization
codegen-units = 1   # Single codegen unit
strip = true        # Strip symbols
panic = "abort"     # Smaller panic handler
```

### Cross-Compilation Fails on macOS

**Issue:** Native cross-compilation on macOS can be complex.

**Solution:** Use Docker builds:
```bash
./scripts/docker-build.sh
```

### Permission Denied When Running Binary

**Error:**
```
bash: ./picoflow: Permission denied
```

**Solution:**
```bash
chmod +x target/*/release/picoflow
```

### QEMU Testing Fails

**Error:**
```
qemu-arm-static: Could not open '/lib/ld-linux-armhf.so.3'
```

**Solution:**
```bash
# Install ARM libraries
sudo apt-get install libc6-armhf-cross

# Use correct library path
qemu-arm-static -L /usr/arm-linux-gnueabihf/ <binary>
```

## CI/CD Integration

### GitHub Actions

PicoFlow includes a GitHub Actions workflow for automated releases:

```yaml
# Trigger on version tags
git tag v1.0.0
git push origin v1.0.0

# Or trigger manually
gh workflow run release.yml -f version=v1.0.0
```

The workflow:
1. Builds for all platforms
2. Verifies binary sizes
3. Runs tests with QEMU
4. Creates release packages
5. Uploads to GitHub Releases

### Local Release Workflow

```bash
# 1. Build release binaries
./scripts/build-release.sh v1.0.0

# 2. Package for distribution
./scripts/package-release.sh v1.0.0

# 3. Test on target platforms
# ... test binaries on actual hardware ...

# 4. Create Git tag
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# 5. Create GitHub release
gh release create v1.0.0 \
    --title "PicoFlow v1.0.0" \
    --notes-file target/packages/RELEASE_NOTES.md \
    target/packages/*.tar.gz \
    target/packages/*.sha256
```

## Performance Targets

PicoFlow maintains strict performance requirements across all platforms:

| Metric | Target | Measurement |
|--------|--------|-------------|
| Binary size | <10MB | `ls -lh target/*/release/picoflow` |
| Idle memory | <20MB | `ps aux \| grep picoflow` (RSS column) |
| Memory (10 tasks) | <50MB | Monitor during execution |
| Task startup | <100ms | See benchmarks in `benches/` |
| DAG parse (100 tasks) | <50ms | Run `cargo bench` |

### Verify Performance

```bash
# Binary size
./scripts/build-release.sh
ls -lh target/release-binaries/

# Runtime performance (on target hardware)
./picoflow run examples/benchmark-workflow.yaml

# Memory usage (on target hardware)
pidstat -r -p $(pidof picoflow) 1
```

## Advanced Configuration

### Custom Cargo Configuration

Edit `.cargo/config.toml` to customize build settings:

```toml
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
rustflags = [
    "-C", "target-cpu=cortex-a7",  # Specific CPU optimization
    "-C", "link-arg=-fuse-ld=lld",
]
```

### Profile-Specific Builds

```bash
# Size-optimized (default release)
cargo build --release --target armv7-unknown-linux-gnueabihf

# Performance-optimized
cargo build --profile release-perf --target armv7-unknown-linux-gnueabihf
```

### Custom Build Features

```bash
# Build without SSH support
cargo build --release --no-default-features --features "shell,http"

# Build with all features
cargo build --release --all-features
```

## Resources

- [Rust Cross-Compilation Guide](https://rust-lang.github.io/rustup/cross-compilation.html)
- [cargo-cross](https://github.com/cross-rs/cross) - Alternative cross-compilation tool
- [Raspberry Pi Documentation](https://www.raspberrypi.com/documentation/)
- [PicoFlow Examples](../examples/)

## Support

If you encounter issues with cross-compilation:

1. Check this troubleshooting guide
2. Search existing issues: https://github.com/zoza1982/picoflow/issues
3. Open a new issue with:
   - Your platform (OS, architecture)
   - Rust version (`rustc --version`)
   - Full error output
   - Steps to reproduce
