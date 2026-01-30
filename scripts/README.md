# Build Scripts Reference

This directory contains scripts for building, packaging, and releasing PicoFlow across multiple platforms.

## Quick Start

```bash
# Build for all platforms using Docker (easiest)
./scripts/docker-build.sh

# Build release binaries
./scripts/build-release.sh v1.0.0

# Package for distribution
./scripts/package-release.sh v1.0.0
```

## Scripts Overview

### `build-all.sh`

Build PicoFlow for all target platforms (ARM32, ARM64, x86_64).

**Requirements:**
- Rust toolchain with targets installed
- Cross-compilation linkers (or use Docker alternative)

**Usage:**
```bash
# Build release binaries for all platforms
./scripts/build-all.sh

# Build debug binaries
./scripts/build-all.sh --debug

# Verbose output
./scripts/build-all.sh --verbose

# Help
./scripts/build-all.sh --help
```

**Output:**
- `target/armv7-unknown-linux-gnueabihf/release/picoflow`
- `target/aarch64-unknown-linux-gnu/release/picoflow`
- `target/x86_64-unknown-linux-gnu/release/picoflow`

**Features:**
- Automatic toolchain detection and installation
- Binary size verification (<10MB target)
- Color-coded output
- Error reporting

---

### `build-release.sh`

Build optimized release binaries with version embedding and metadata.

**Requirements:**
- Same as `build-all.sh`
- Git repository (for commit hash)

**Usage:**
```bash
# Use version from Cargo.toml
./scripts/build-release.sh

# Override version
./scripts/build-release.sh v1.0.0
```

**Output:**
```
target/release-binaries/
  ├── picoflow-v1.0.0-arm32
  ├── picoflow-v1.0.0-arm64
  ├── picoflow-v1.0.0-x86_64
  └── manifest.json
```

**Features:**
- Embeds version, git hash, and build date
- Creates unified output directory
- Generates JSON manifest with checksums
- Binary verification
- Size reporting

**Environment Variables Set:**
- `PICOFLOW_VERSION` - Version string
- `PICOFLOW_GIT_HASH` - Git commit hash
- `PICOFLOW_BUILD_DATE` - Build timestamp

---

### `package-release.sh`

Create distribution packages with installation scripts and documentation.

**Requirements:**
- Binaries built with `build-release.sh`
- tar, gzip, sha256sum (standard Unix tools)

**Usage:**
```bash
# Package all binaries
./scripts/package-release.sh v1.0.0

# Must match version used in build-release.sh
```

**Output:**
```
target/packages/
  ├── picoflow-v1.0.0-arm32-linux.tar.gz
  ├── picoflow-v1.0.0-arm32-linux.tar.gz.sha256
  ├── picoflow-v1.0.0-arm64-linux.tar.gz
  ├── picoflow-v1.0.0-arm64-linux.tar.gz.sha256
  ├── picoflow-v1.0.0-x86_64-linux.tar.gz
  ├── picoflow-v1.0.0-x86_64-linux.tar.gz.sha256
  └── RELEASE_NOTES.md
```

**Each package includes:**
- `picoflow` - Executable binary
- `install.sh` - Installation script
- `picoflow.service` - systemd service file
- `config.example.yaml` - Example configuration
- `README.md` - Quick start guide
- `LICENSE` - MIT license

**Features:**
- Automated package creation
- SHA256 checksums for integrity
- Installation scripts for easy deployment
- systemd service files
- Release notes generation

---

### `docker-build.sh`

Build PicoFlow using Docker for consistent cross-compilation environment.

**Requirements:**
- Docker installed and running
- No cross-compilation toolchains needed

**Usage:**
```bash
# Build for all platforms
./scripts/docker-build.sh

# Build for specific platform
./scripts/docker-build.sh arm32
./scripts/docker-build.sh arm64
./scripts/docker-build.sh x86_64
```

**Output:**
```
target/release-binaries/
  ├── picoflow-arm32
  ├── picoflow-arm64
  └── picoflow-x86_64
```

**Features:**
- No local toolchain setup required
- Consistent build environment
- Automatic Docker image building
- Volume mounting for output
- Cargo cache persistence

**Docker Image:**
- Name: `picoflow-builder`
- Base: `rust:1.75-slim`
- Includes: All cross-compilation toolchains
- Build: `docker build -f Dockerfile.build -t picoflow-builder .`

---

## Complete Release Workflow

### 1. Build Release Binaries

```bash
# Set version
VERSION=v1.0.0

# Build binaries with version info
./scripts/build-release.sh $VERSION
```

### 2. Test Binaries

```bash
# Test on x86_64 (if on Linux)
target/release-binaries/picoflow-$VERSION-x86_64 --version

# Test ARM binaries with QEMU
qemu-arm-static -L /usr/arm-linux-gnueabihf/ \
    target/release-binaries/picoflow-$VERSION-arm32 --version

qemu-aarch64-static -L /usr/aarch64-linux-gnu/ \
    target/release-binaries/picoflow-$VERSION-arm64 --version
```

### 3. Package for Distribution

```bash
./scripts/package-release.sh $VERSION
```

### 4. Verify Packages

```bash
# Check package contents
tar -tzf target/packages/picoflow-$VERSION-arm32-linux.tar.gz

# Verify checksums
cd target/packages
sha256sum -c *.sha256
```

### 5. Test Installation (Optional)

```bash
# Extract and test install script
cd target/packages
tar -xzf picoflow-$VERSION-x86_64-linux.tar.gz
cd picoflow-$VERSION-x86_64-linux
./install.sh --help
```

### 6. Create Git Tag

```bash
git tag -a $VERSION -m "Release $VERSION"
git push origin $VERSION
```

### 7. Create GitHub Release

```bash
# Automated via GitHub Actions (triggered by tag push)
# Or manually with gh CLI:
gh release create $VERSION \
    --title "PicoFlow $VERSION" \
    --notes-file target/packages/RELEASE_NOTES.md \
    target/packages/*.tar.gz \
    target/packages/*.sha256
```

---

## Docker-Based Workflow

For consistent builds without local toolchain setup:

### 1. Build Docker Image (One-Time)

```bash
docker build -f Dockerfile.build -t picoflow-builder .
```

### 2. Build All Binaries

```bash
./scripts/docker-build.sh
```

### 3. Continue with Steps 2-7 Above

The Docker build produces identical binaries to native builds.

---

## Troubleshooting

### Missing Linkers

**Error:**
```
error: linker `arm-linux-gnueabihf-gcc` not found
```

**Solution:**
```bash
# Ubuntu/Debian
sudo apt-get install gcc-arm-linux-gnueabihf gcc-aarch64-linux-gnu

# Or use Docker builds
./scripts/docker-build.sh
```

### Binary Size Too Large

**Check:**
```bash
ls -lh target/release-binaries/
```

**Solution:**
- Verify `.cargo/config.toml` optimization settings
- Ensure `strip = true` in `[profile.release]`
- Check for debug features accidentally enabled

### Docker Build Fails

**Error:**
```
Cannot connect to Docker daemon
```

**Solution:**
```bash
# Start Docker daemon
sudo systemctl start docker  # Linux
# Or start Docker Desktop on macOS/Windows
```

### Version Mismatch

**Error:**
```
Binary not found: target/release-binaries/picoflow-v1.0.0-arm32
```

**Solution:**
- Ensure version in `build-release.sh` matches `package-release.sh`
- Run `build-release.sh` before `package-release.sh`

---

## CI/CD Integration

### GitHub Actions

The `.github/workflows/release.yml` workflow automates the entire build and release process:

**Triggers:**
- Git tags matching `v*.*.*` (e.g., `v1.0.0`)
- Manual workflow dispatch

**Jobs:**
1. **build** - Build for all platforms (Linux ARM32/ARM64/x86_64, macOS ARM64/Intel)
2. **package** - Create distribution packages (`.tar.gz` with install scripts)
3. **create-release** - Upload to GitHub Releases
4. **update-homebrew** - Update `zoza1982/homebrew-picoflow` tap formula with new version and SHA256 checksums (requires `HOMEBREW_TAP_TOKEN` secret)
5. **test-binaries** - Verify binaries on native runners (macOS) and QEMU (ARM)

**Manual Trigger:**
```bash
gh workflow run release.yml -f version=v1.0.0
```

---

## Performance Targets

All builds must meet these targets:

| Metric | Target | Command to Verify |
|--------|--------|-------------------|
| Binary size | <10MB | `ls -lh target/*/release/picoflow` |
| Build time (all) | <10 min | Time `./scripts/build-all.sh` |
| Package size | <2MB each | `ls -lh target/packages/*.tar.gz` |

---

## Advanced Usage

### Custom Optimization Profiles

Edit `.cargo/config.toml` to add custom profiles:

```toml
[profile.release-minimal]
inherits = "release"
opt-level = "z"
strip = true
lto = "fat"
```

Build with custom profile:
```bash
cargo build --profile release-minimal --target armv7-unknown-linux-gnueabihf
```

### Parallel Builds

Speed up builds with parallel compilation:

```bash
# Set number of parallel jobs
export CARGO_BUILD_JOBS=8
./scripts/build-all.sh
```

### Incremental Builds

For development, enable incremental compilation:

```bash
export CARGO_INCREMENTAL=1
cargo build --target armv7-unknown-linux-gnueabihf
```

---

## Resources

- [Cross-Compilation Guide](../docs/cross-compilation.md)
- [Rust Cross-Compilation Book](https://rust-lang.github.io/rustup/cross-compilation.html)
- [cargo-cross](https://github.com/cross-rs/cross)
- [Dockerfile.build](../Dockerfile.build)

---

## Support

For issues with build scripts:
1. Check this README
2. See [docs/cross-compilation.md](../docs/cross-compilation.md)
3. Open issue: https://github.com/zoza1982/picoflow/issues
