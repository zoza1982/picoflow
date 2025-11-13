# Quick Start: Cross-Compilation

## TL;DR - I Just Want to Build

### Using Docker (Easiest - No Setup Required)

```bash
# Build for all platforms
./scripts/docker-build.sh

# Find binaries in:
ls -lh target/release-binaries/
```

### Using Native Toolchains (Ubuntu/Debian)

```bash
# One-time setup
sudo apt-get install gcc-arm-linux-gnueabihf gcc-aarch64-linux-gnu
rustup target add armv7-unknown-linux-gnueabihf aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu

# Build
./scripts/build-all.sh
```

### Using Native Toolchains (macOS)

```bash
# One-time setup
brew tap messense/macos-cross-toolchains
brew install arm-unknown-linux-gnueabihf aarch64-unknown-linux-gnu
rustup target add armv7-unknown-linux-gnueabihf aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu

# Build (or use Docker)
./scripts/docker-build.sh
```

## I Want to Create a Release

```bash
# 1. Build binaries with version
./scripts/build-release.sh v1.0.0

# 2. Package for distribution
./scripts/package-release.sh v1.0.0

# 3. Tag and push (triggers GitHub Actions)
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# GitHub Actions will:
# - Build all platforms
# - Create release
# - Upload binaries
```

## I Want to Test on Raspberry Pi

### Method 1: Copy Binary (Fastest)

```bash
# Build for ARM 32-bit
cargo build --release --target armv7-unknown-linux-gnueabihf

# Copy to Pi
scp target/armv7-unknown-linux-gnueabihf/release/picoflow pi@raspberrypi:~/

# Test on Pi
ssh pi@raspberrypi
./picoflow --version
```

### Method 2: Install from Package

```bash
# After building release packages
scp target/packages/picoflow-v1.0.0-arm32-linux.tar.gz pi@raspberrypi:~/

# On Pi
tar -xzf picoflow-v1.0.0-arm32-linux.tar.gz
cd picoflow-v1.0.0-arm32-linux
sudo ./install.sh
picoflow --version
```

### Method 3: Test with QEMU (No Pi Required)

```bash
# Install QEMU
sudo apt-get install qemu-user-static

# Test ARM binary on x86_64
qemu-arm-static -L /usr/arm-linux-gnueabihf/ \
    target/armv7-unknown-linux-gnueabihf/release/picoflow --version
```

## Build Commands Reference

| Command | What It Does | Output |
|---------|-------------|--------|
| `./scripts/build-all.sh` | Build for all platforms | `target/*/release/picoflow` |
| `./scripts/build-release.sh v1.0.0` | Build with version info | `target/release-binaries/*` |
| `./scripts/package-release.sh v1.0.0` | Create .tar.gz packages | `target/packages/*.tar.gz` |
| `./scripts/docker-build.sh` | Docker build (no setup) | `target/release-binaries/*` |

## Platform Reference

| Platform | Target | Devices | Command |
|----------|--------|---------|---------|
| ARM 32-bit | `armv7-unknown-linux-gnueabihf` | Pi Zero 2 W, Pi 3/4 | `cargo build --target armv7-unknown-linux-gnueabihf --release` |
| ARM 64-bit | `aarch64-unknown-linux-gnu` | Pi 4/5 | `cargo build --target aarch64-unknown-linux-gnu --release` |
| x86_64 | `x86_64-unknown-linux-gnu` | Servers, VMs | `cargo build --target x86_64-unknown-linux-gnu --release` |

## Troubleshooting

### "Linker not found"
→ Use Docker: `./scripts/docker-build.sh`

### "Binary too large"
→ Check `.cargo/config.toml` has `strip = true`

### "Permission denied" when running binary
→ `chmod +x picoflow`

### "Cannot connect to Docker daemon"
→ Start Docker: `sudo systemctl start docker` (Linux) or Docker Desktop (macOS/Windows)

## Full Documentation

- [Complete Cross-Compilation Guide](docs/cross-compilation.md)
- [Build Scripts Reference](scripts/README.md)
- [Setup Summary](CROSS_COMPILATION_SETUP.md)
