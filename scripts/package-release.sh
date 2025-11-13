#!/usr/bin/env bash
#
# package-release.sh - Package release binaries for distribution
#
# This script packages PicoFlow release binaries with:
# - Compressed archives (.tar.gz) for each platform
# - SHA256 checksums for integrity verification
# - Installation scripts
# - Documentation
# - systemd service files
#
# Usage:
#   ./scripts/package-release.sh [VERSION]
#
# Prerequisites:
#   - Run ./scripts/build-release.sh first
#
# Examples:
#   ./scripts/package-release.sh           # Use version from Cargo.toml
#   ./scripts/package-release.sh v1.0.0    # Override version
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VERSION="${1:-}"
BINARIES_DIR="target/release-binaries"
PACKAGES_DIR="target/packages"

PLATFORMS=(
    "arm32:armv7-unknown-linux-gnueabihf"
    "arm64:aarch64-unknown-linux-gnu"
    "x86_64:x86_64-unknown-linux-gnu"
)

# Print colored message
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Get version from Cargo.toml if not provided
get_version() {
    if [[ -z "$VERSION" ]]; then
        VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
        log_info "Using version from Cargo.toml: $VERSION"
    else
        log_info "Using provided version: $VERSION"
    fi
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    if [[ ! -d "$BINARIES_DIR" ]]; then
        log_error "Binaries directory not found: $BINARIES_DIR"
        echo "Please run ./scripts/build-release.sh first"
        exit 1
    fi

    local missing_binaries=()
    for platform_info in "${PLATFORMS[@]}"; do
        local platform_name="${platform_info%%:*}"
        local binary="$BINARIES_DIR/picoflow-$VERSION-$platform_name"

        if [[ ! -f "$binary" ]]; then
            missing_binaries+=("$platform_name")
        fi
    done

    if [[ ${#missing_binaries[@]} -gt 0 ]]; then
        log_error "Missing binaries for platforms:"
        for platform in "${missing_binaries[@]}"; do
            echo "  - $platform"
        done
        echo "Please run ./scripts/build-release.sh first"
        exit 1
    fi

    log_success "All prerequisites met"
}

# Prepare packaging directory
prepare_packaging() {
    log_info "Preparing packaging directory..."

    if [[ -d "$PACKAGES_DIR" ]]; then
        rm -rf "$PACKAGES_DIR"
    fi

    mkdir -p "$PACKAGES_DIR"

    log_success "Packaging directory ready: $PACKAGES_DIR"
}

# Create installation script
create_install_script() {
    local package_dir=$1

    cat > "$package_dir/install.sh" <<'INSTALL_SCRIPT'
#!/usr/bin/env bash
#
# PicoFlow Installation Script
#

set -euo pipefail

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
SERVICE_DIR="${SERVICE_DIR:-/etc/systemd/system}"

echo "PicoFlow Installation"
echo "====================="
echo ""

# Check if running as root for system-wide install
if [[ "$INSTALL_DIR" == "/usr/local/bin" ]] && [[ $EUID -ne 0 ]]; then
    echo "System-wide installation requires root privileges."
    echo "Run with sudo: sudo ./install.sh"
    echo ""
    echo "Or install to user directory:"
    echo "  INSTALL_DIR=~/.local/bin ./install.sh"
    exit 1
fi

# Install binary
echo "Installing picoflow binary to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"
cp picoflow "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/picoflow"

# Install systemd service (if running as root)
if [[ $EUID -eq 0 ]] && [[ -f "picoflow.service" ]]; then
    echo "Installing systemd service to $SERVICE_DIR..."
    cp picoflow.service "$SERVICE_DIR/"
    systemctl daemon-reload
    echo ""
    echo "To enable and start PicoFlow service:"
    echo "  sudo systemctl enable picoflow"
    echo "  sudo systemctl start picoflow"
fi

echo ""
echo "Installation complete!"
echo ""
echo "Verify installation:"
echo "  picoflow --version"
echo ""
echo "Get started:"
echo "  picoflow --help"
INSTALL_SCRIPT

    chmod +x "$package_dir/install.sh"
}

# Create systemd service file
create_systemd_service() {
    local package_dir=$1

    cat > "$package_dir/picoflow.service" <<'SERVICE_FILE'
[Unit]
Description=PicoFlow Workflow Orchestrator
After=network.target

[Service]
Type=simple
User=picoflow
Group=picoflow
ExecStart=/usr/local/bin/picoflow daemon --config /etc/picoflow/config.yaml
Restart=on-failure
RestartSec=10

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/picoflow /var/log/picoflow

# Resource limits
MemoryMax=128M
TasksMax=100

[Install]
WantedBy=multi-user.target
SERVICE_FILE
}

# Create example configuration
create_example_config() {
    local package_dir=$1

    cat > "$package_dir/config.example.yaml" <<'CONFIG_FILE'
# PicoFlow Configuration Example
# Copy to /etc/picoflow/config.yaml and customize

# Database configuration
database:
  path: /var/lib/picoflow/picoflow.db

# Logging configuration
logging:
  level: info
  format: json
  path: /var/log/picoflow

# Metrics configuration
metrics:
  enabled: true
  port: 9090
  path: /metrics

# Workflow configuration
workflow:
  max_parallel: 10
  retry_default: 3
  timeout_default: 300

# Scheduler configuration
scheduler:
  enabled: true
  check_interval: 60
CONFIG_FILE
}

# Create README for package
create_package_readme() {
    local package_dir=$1
    local platform_name=$2

    cat > "$package_dir/README.md" <<README
# PicoFlow $VERSION - $platform_name

Lightweight DAG workflow orchestrator for edge devices.

## Quick Start

### Installation

**System-wide (requires root):**
\`\`\`bash
sudo ./install.sh
\`\`\`

**User directory:**
\`\`\`bash
INSTALL_DIR=~/.local/bin ./install.sh
\`\`\`

### Verify Installation

\`\`\`bash
picoflow --version
\`\`\`

### Create Your First Workflow

1. Create a workflow file \`my-workflow.yaml\`:
\`\`\`yaml
name: hello-world
description: My first PicoFlow workflow

tasks:
  - name: hello
    type: shell
    config:
      command: echo "Hello from PicoFlow!"
\`\`\`

2. Run the workflow:
\`\`\`bash
picoflow run my-workflow.yaml
\`\`\`

## Documentation

- [Getting Started Guide](https://github.com/zoza1982/picoflow/blob/main/docs/getting-started.md)
- [Configuration Reference](https://github.com/zoza1982/picoflow/blob/main/docs/configuration.md)
- [Workflow Examples](https://github.com/zoza1982/picoflow/tree/main/examples)
- [Full Documentation](https://github.com/zoza1982/picoflow/blob/main/README.md)

## Systemd Service (Optional)

To run PicoFlow as a system service:

\`\`\`bash
# Install and enable service
sudo ./install.sh
sudo systemctl enable picoflow
sudo systemctl start picoflow

# Check status
sudo systemctl status picoflow

# View logs
sudo journalctl -u picoflow -f
\`\`\`

## Support

- GitHub: https://github.com/zoza1982/picoflow
- Issues: https://github.com/zoza1982/picoflow/issues
- License: MIT
README
}

# Package a single platform
package_platform() {
    local platform_info=$1
    local platform_name="${platform_info%%:*}"
    local target="${platform_info#*:}"

    log_info "Packaging $platform_name..."

    # Create package directory
    local package_name="picoflow-$VERSION-$platform_name-linux"
    local package_dir="$PACKAGES_DIR/$package_name"
    mkdir -p "$package_dir"

    # Copy binary
    local binary="$BINARIES_DIR/picoflow-$VERSION-$platform_name"
    cp "$binary" "$package_dir/picoflow"
    chmod +x "$package_dir/picoflow"

    # Create installation script
    create_install_script "$package_dir"

    # Create systemd service
    create_systemd_service "$package_dir"

    # Create example configuration
    create_example_config "$package_dir"

    # Create package README
    create_package_readme "$package_dir" "$platform_name"

    # Copy documentation
    if [[ -f "README.md" ]]; then
        cp "README.md" "$package_dir/README-full.md"
    fi

    if [[ -f "LICENSE" ]]; then
        cp "LICENSE" "$package_dir/"
    fi

    # Create archive
    local archive_name="$package_name.tar.gz"
    log_info "Creating archive: $archive_name"

    (cd "$PACKAGES_DIR" && tar -czf "$archive_name" "$package_name")

    # Generate SHA256 checksum
    if command -v sha256sum &> /dev/null; then
        (cd "$PACKAGES_DIR" && sha256sum "$archive_name" > "$archive_name.sha256")
    else
        (cd "$PACKAGES_DIR" && shasum -a 256 "$archive_name" > "$archive_name.sha256")
    fi

    # Get archive size
    local archive_size
    if [[ "$OSTYPE" == "darwin"* ]]; then
        archive_size=$(stat -f%z "$PACKAGES_DIR/$archive_name")
    else
        archive_size=$(stat -c%s "$PACKAGES_DIR/$archive_name")
    fi

    local size_kb=$(echo "scale=1; $archive_size / 1024" | bc)

    log_success "Package created: $archive_name (${size_kb}KB)"

    # Clean up temporary directory
    rm -rf "$package_dir"
}

# Generate release notes
generate_release_notes() {
    local notes_file="$PACKAGES_DIR/RELEASE_NOTES.md"

    log_info "Generating release notes..."

    cat > "$notes_file" <<NOTES
# PicoFlow $VERSION Release Notes

## Installation

Download the appropriate package for your platform:

NOTES

    for platform_info in "${PLATFORMS[@]}"; do
        local platform_name="${platform_info%%:*}"
        local package_name="picoflow-$VERSION-$platform_name-linux.tar.gz"

        if [[ -f "$PACKAGES_DIR/$package_name" ]]; then
            local sha256
            if [[ -f "$PACKAGES_DIR/$package_name.sha256" ]]; then
                sha256=$(cat "$PACKAGES_DIR/$package_name.sha256" | awk '{print $1}')
            else
                sha256="N/A"
            fi

            cat >> "$notes_file" <<NOTES

### $platform_name

\`\`\`bash
# Download
wget https://github.com/zoza1982/picoflow/releases/download/$VERSION/$package_name

# Verify checksum
echo "$sha256  $package_name" | sha256sum -c -

# Extract and install
tar -xzf $package_name
cd ${package_name%.tar.gz}
sudo ./install.sh
\`\`\`

NOTES
        fi
    done

    cat >> "$notes_file" <<NOTES

## What's New

See [CHANGELOG.md](https://github.com/zoza1982/picoflow/blob/main/CHANGELOG.md) for detailed changes.

## Documentation

- [Getting Started Guide](https://github.com/zoza1982/picoflow/blob/main/docs/getting-started.md)
- [Configuration Reference](https://github.com/zoza1982/picoflow/blob/main/docs/configuration.md)
- [Workflow Examples](https://github.com/zoza1982/picoflow/tree/main/examples)

## Support

- GitHub: https://github.com/zoza1982/picoflow
- Issues: https://github.com/zoza1982/picoflow/issues
NOTES

    log_success "Release notes generated: $notes_file"
}

# Print summary
print_summary() {
    echo ""
    echo "========================================"
    echo "Packaging Summary"
    echo "========================================"
    echo "Version: $VERSION"
    echo "Output Directory: $PACKAGES_DIR"
    echo ""
    echo "Packages:"

    for platform_info in "${PLATFORMS[@]}"; do
        local platform_name="${platform_info%%:*}"
        local package_name="picoflow-$VERSION-$platform_name-linux.tar.gz"

        if [[ -f "$PACKAGES_DIR/$package_name" ]]; then
            local archive_size
            if [[ "$OSTYPE" == "darwin"* ]]; then
                archive_size=$(stat -f%z "$PACKAGES_DIR/$package_name")
            else
                archive_size=$(stat -c%s "$PACKAGES_DIR/$package_name")
            fi

            local size_kb=$(echo "scale=1; $archive_size / 1024" | bc)
            echo "  $platform_name: $package_name (${size_kb}KB)"
        fi
    done

    echo ""
    echo "Files in $PACKAGES_DIR:"
    ls -lh "$PACKAGES_DIR" | tail -n +2 | awk '{print "  " $9 " (" $5 ")"}'

    echo ""
    echo "Next steps:"
    echo "  1. Test packages on target platforms"
    echo "  2. Create GitHub release: gh release create $VERSION"
    echo "  3. Upload packages: gh release upload $VERSION $PACKAGES_DIR/*.tar.gz $PACKAGES_DIR/*.sha256"
}

# Main execution
main() {
    echo "========================================"
    echo "PicoFlow Release Packaging"
    echo "========================================"
    echo ""

    get_version
    check_prerequisites
    echo ""

    prepare_packaging
    echo ""

    for platform_info in "${PLATFORMS[@]}"; do
        package_platform "$platform_info"
    done
    echo ""

    generate_release_notes
    echo ""

    print_summary
    log_success "Packaging completed successfully!"
}

# Run main function
main
