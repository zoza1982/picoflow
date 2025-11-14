#!/usr/bin/env bash
#
# build-release.sh - Build optimized release binaries with version embedding
#
# This script builds release-optimized binaries for all platforms with:
# - Version information embedded in the binary
# - Maximum size optimization
# - Symbol stripping
# - Binary verification
#
# Usage:
#   ./scripts/build-release.sh [VERSION]
#
# Examples:
#   ./scripts/build-release.sh           # Use version from Cargo.toml
#   ./scripts/build-release.sh v1.0.0    # Override version
#

set -eo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VERSION="${1:-}"
TARGETS=(
    "armv7-unknown-linux-gnueabihf"
    "aarch64-unknown-linux-gnu"
    "x86_64-unknown-linux-gnu"
)

# Now enable nounset
set -u

# Helper function to get platform name (Bash 3.2 compatible - no associative arrays)
get_platform_name() {
    case "$1" in
        "armv7-unknown-linux-gnueabihf")
            echo "arm32"
            ;;
        "aarch64-unknown-linux-gnu")
            echo "arm64"
            ;;
        "x86_64-unknown-linux-gnu")
            echo "x86_64"
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

# Output directory
OUTPUT_DIR="target/release-binaries"

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

# Set build environment variables
set_build_env() {
    local git_hash
    git_hash=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

    local build_date
    build_date=$(date -u +"%Y-%m-%d")

    export PICOFLOW_VERSION="$VERSION"
    export PICOFLOW_GIT_HASH="$git_hash"
    export PICOFLOW_BUILD_DATE="$build_date"

    log_info "Build environment:"
    echo "  Version: $PICOFLOW_VERSION"
    echo "  Git Hash: $PICOFLOW_GIT_HASH"
    echo "  Build Date: $PICOFLOW_BUILD_DATE"
}

# Clean previous builds
clean_builds() {
    log_info "Cleaning previous builds..."

    for target in "${TARGETS[@]}"; do
        if [[ -d "target/$target/release" ]]; then
            rm -f "target/$target/release/picoflow"
        fi
    done

    if [[ -d "$OUTPUT_DIR" ]]; then
        rm -rf "$OUTPUT_DIR"
    fi

    mkdir -p "$OUTPUT_DIR"

    log_success "Clean completed"
}

# Build for a specific target
build_target() {
    local target=$1
    local platform_name
    platform_name=$(get_platform_name "$target")

    log_info "Building release binary for $target..."

    # Build with maximum optimization
    if RUSTFLAGS="-C embed-bitcode=yes" cargo build \
        --release \
        --target "$target" \
        --locked; then

        local binary_path="target/$target/release/picoflow"

        if [[ -f "$binary_path" ]]; then
            # Get binary size
            local binary_size
            if [[ "$OSTYPE" == "darwin"* ]]; then
                binary_size=$(stat -f%z "$binary_path")
            else
                binary_size=$(stat -c%s "$binary_path")
            fi

            local size_mb=$(echo "scale=2; $binary_size / 1024 / 1024" | bc)
            log_success "Binary size: ${size_mb}MB"

            # Copy to output directory with platform-specific name
            local output_binary="$OUTPUT_DIR/picoflow-$VERSION-$platform_name"
            cp "$binary_path" "$output_binary"
            chmod +x "$output_binary"

            log_success "Binary copied to: $output_binary"
            return 0
        else
            log_error "Binary not found: $binary_path"
            return 1
        fi
    else
        log_error "Build failed for $target"
        return 1
    fi
}

# Verify binary can be executed
verify_binary() {
    local binary=$1
    local target=$2

    log_info "Verifying binary: $(basename "$binary")"

    # Check if binary exists and is executable
    if [[ ! -x "$binary" ]]; then
        log_error "Binary is not executable"
        return 1
    fi

    # For x86_64 on compatible platforms, try to run --version
    if [[ "$target" == "x86_64-unknown-linux-gnu" ]] && [[ "$OSTYPE" == "linux-gnu"* ]]; then
        if "$binary" --version &>/dev/null; then
            log_success "Binary verification passed"
            return 0
        else
            log_warning "Binary exists but could not run --version"
            return 1
        fi
    elif [[ "$target" == "x86_64-unknown-linux-gnu" ]] && [[ "$OSTYPE" == "darwin"* ]]; then
        log_warning "Cannot verify Linux binary on macOS (use Docker or QEMU for testing)"
    else
        log_warning "Cross-compiled binary (cannot verify on host platform)"
    fi

    return 0
}

# Generate build manifest
generate_manifest() {
    local manifest_file="$OUTPUT_DIR/manifest.json"

    log_info "Generating build manifest..."

    cat > "$manifest_file" <<EOF
{
  "version": "$VERSION",
  "build_date": "$PICOFLOW_BUILD_DATE",
  "git_hash": "$PICOFLOW_GIT_HASH",
  "binaries": [
EOF

    local first=true
    for target in "${TARGETS[@]}"; do
        local platform_name
    platform_name=$(get_platform_name "$target")
        local binary="$OUTPUT_DIR/picoflow-$VERSION-$platform_name"

        if [[ -f "$binary" ]]; then
            local binary_size
            if [[ "$OSTYPE" == "darwin"* ]]; then
                binary_size=$(stat -f%z "$binary")
            else
                binary_size=$(stat -c%s "$binary")
            fi

            local sha256sum
            if command -v sha256sum &> /dev/null; then
                sha256sum=$(sha256sum "$binary" | awk '{print $1}')
            else
                sha256sum=$(shasum -a 256 "$binary" | awk '{print $1}')
            fi

            if [[ "$first" == false ]]; then
                echo "," >> "$manifest_file"
            fi
            first=false

            cat >> "$manifest_file" <<EOF
    {
      "platform": "$platform_name",
      "target": "$target",
      "filename": "$(basename "$binary")",
      "size_bytes": $binary_size,
      "sha256": "$sha256sum"
    }
EOF
        fi
    done

    cat >> "$manifest_file" <<EOF

  ]
}
EOF

    log_success "Manifest generated: $manifest_file"
}

# Build summary
print_summary() {
    echo ""
    echo "========================================"
    echo "Release Build Summary"
    echo "========================================"
    echo "Version: $VERSION"
    echo "Output Directory: $OUTPUT_DIR"
    echo ""
    echo "Binaries:"

    for target in "${TARGETS[@]}"; do
        local platform_name
    platform_name=$(get_platform_name "$target")
        local binary="$OUTPUT_DIR/picoflow-$VERSION-$platform_name"

        if [[ -f "$binary" ]]; then
            local binary_size
            if [[ "$OSTYPE" == "darwin"* ]]; then
                binary_size=$(stat -f%z "$binary")
            else
                binary_size=$(stat -c%s "$binary")
            fi

            local size_mb=$(echo "scale=2; $binary_size / 1024 / 1024" | bc)
            echo "  $platform_name: ${size_mb}MB"
        else
            echo "  $platform_name: FAILED"
        fi
    done

    echo ""
    echo "Next steps:"
    echo "  1. Test binaries on target platforms"
    echo "  2. Run: ./scripts/package-release.sh $VERSION"
    echo "  3. Create GitHub release and upload artifacts"
}

# Main execution
main() {
    echo "========================================"
    echo "PicoFlow Release Build"
    echo "========================================"
    echo ""

    get_version
    set_build_env
    echo ""

    clean_builds
    echo ""

    local failed_builds=()

    for target in "${TARGETS[@]}"; do
        if ! build_target "$target"; then
            failed_builds+=("$target")
        fi
        echo ""
    done

    # Verify binaries
    for target in "${TARGETS[@]}"; do
        local platform_name
    platform_name=$(get_platform_name "$target")
        local binary="$OUTPUT_DIR/picoflow-$VERSION-$platform_name"
        if [[ -f "$binary" ]]; then
            verify_binary "$binary" "$target"
        fi
    done
    echo ""

    # Generate manifest
    generate_manifest
    echo ""

    # Print summary
    if [[ ${#failed_builds[@]} -eq 0 ]]; then
        print_summary
        log_success "Release build completed successfully!"
        exit 0
    else
        log_error "Some builds failed:"
        for target in "${failed_builds[@]}"; do
            echo "  - $target"
        done
        exit 1
    fi
}

# Run main function
main
