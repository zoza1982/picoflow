#!/usr/bin/env bash
#
# build-all.sh - Build PicoFlow for all target platforms
#
# This script builds PicoFlow for:
# - ARM 32-bit (Raspberry Pi Zero 2 W, Pi 3/4 in 32-bit mode)
# - ARM 64-bit (Raspberry Pi 4/5, modern SBCs)
# - x86_64 (Standard Linux servers, dev machines)
#
# Usage:
#   ./scripts/build-all.sh [OPTIONS]
#
# Options:
#   --debug      Build debug binaries instead of release
#   --verbose    Show verbose build output
#   --help       Show this help message
#

set -eo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Build configuration
BUILD_TYPE="release"
VERBOSE=""
USE_CROSS=false
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
            echo "ARM 32-bit (Pi Zero 2 W, Pi 3/4)"
            ;;
        "aarch64-unknown-linux-gnu")
            echo "ARM 64-bit (Pi 4/5, modern SBCs)"
            ;;
        "x86_64-unknown-linux-gnu")
            echo "x86_64 (Linux servers)"
            ;;
        *)
            echo "Unknown"
            ;;
    esac
}

# Binary size limits (in bytes)
MAX_BINARY_SIZE=$((10 * 1024 * 1024))  # 10MB

# Parse command-line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --debug)
                BUILD_TYPE="debug"
                shift
                ;;
            --verbose)
                VERBOSE="--verbose"
                shift
                ;;
            --help)
                sed -n '2,16p' "$0" | sed 's/^# //'
                exit 0
                ;;
            *)
                echo -e "${RED}Error: Unknown option: $1${NC}"
                echo "Run with --help for usage information"
                exit 1
                ;;
        esac
    done
}

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

# Check if required toolchains are installed
check_toolchains() {
    log_info "Checking Rust toolchains..."

    local missing_targets=()

    for target in "${TARGETS[@]}"; do
        if ! rustup target list --installed | grep -q "$target"; then
            missing_targets+=("$target")
        fi
    done

    if [[ ${#missing_targets[@]} -gt 0 ]]; then
        log_warning "Missing toolchains detected. Installing..."
        for target in "${missing_targets[@]}"; do
            log_info "Installing target: $target"
            rustup target add "$target"
        done
    else
        log_success "All required toolchains are installed"
    fi
}

# Check if cross-compilation linkers are available
check_linkers() {
    log_info "Checking cross-compilation linkers..."

    local missing_linkers=()

    # Check ARM 32-bit linker
    if ! command -v arm-linux-gnueabihf-gcc &> /dev/null; then
        missing_linkers+=("arm-linux-gnueabihf-gcc")
    fi

    # Check ARM 64-bit linker
    if ! command -v aarch64-linux-gnu-gcc &> /dev/null; then
        missing_linkers+=("aarch64-linux-gnu-gcc")
    fi

    if [[ ${#missing_linkers[@]} -gt 0 ]]; then
        log_warning "Missing cross-compilation linkers:"
        for linker in "${missing_linkers[@]}"; do
            echo "  - $linker"
        done
        echo ""

        # Check if cross is available (check PATH and ~/.cargo/bin)
        if command -v cross &> /dev/null || [[ -x "$HOME/.cargo/bin/cross" ]]; then
            log_info "Using 'cross' for Docker-based cross-compilation"
            USE_CROSS=true
            # Ensure ~/.cargo/bin is in PATH
            export PATH="$HOME/.cargo/bin:$PATH"
            return 0
        else
            log_error "Neither native linkers nor 'cross' are available"
            echo ""
            log_info "Option 1 - Install native linkers:"
            echo "  Ubuntu/Debian: sudo apt-get install gcc-arm-linux-gnueabihf gcc-aarch64-linux-gnu"
            echo "  macOS: brew install arm-linux-gnueabihf-binutils aarch64-linux-gnu-binutils"
            echo ""
            log_info "Option 2 - Install cross (Docker-based):"
            echo "  cargo install cross --git https://github.com/cross-rs/cross"
            echo ""
            log_info "Option 3 - Use Docker build script:"
            echo "  ./scripts/docker-build.sh"
            return 1
        fi
    else
        log_success "All required linkers are available"
    fi
}

# Build for a specific target
build_target() {
    local target=$1
    local platform_name
    platform_name=$(get_platform_name "$target")

    log_info "Building for $platform_name ($target)..."

    local build_flags=""
    if [[ "$BUILD_TYPE" == "release" ]]; then
        build_flags="--release"
    fi

    # Build the binary (use cross if enabled, otherwise cargo)
    local build_cmd="cargo"
    if [[ "$USE_CROSS" == true ]]; then
        build_cmd="cross"
        # Get the active toolchain for cross
        local toolchain
        toolchain=$(rustup show active-toolchain | awk '{print $1}')
        export CROSS_CUSTOM_TOOLCHAIN=1
    fi

    if $build_cmd build $build_flags --target "$target" $VERBOSE; then
        log_success "Build completed for $platform_name"

        # Get binary path
        local binary_path="target/$target/$BUILD_TYPE/picoflow"

        if [[ -f "$binary_path" ]]; then
            # Check binary size
            local binary_size
            if [[ "$OSTYPE" == "darwin"* ]]; then
                binary_size=$(stat -f%z "$binary_path")
            else
                binary_size=$(stat -c%s "$binary_path")
            fi

            local size_mb=$(echo "scale=2; $binary_size / 1024 / 1024" | bc)

            if [[ $binary_size -gt $MAX_BINARY_SIZE ]]; then
                log_warning "Binary size: ${size_mb}MB (exceeds 10MB target)"
            else
                log_success "Binary size: ${size_mb}MB"
            fi

            # Strip binary if not already stripped (for debug builds)
            if [[ "$BUILD_TYPE" == "debug" ]]; then
                log_info "Stripping debug binary..."
                strip "$binary_path" 2>/dev/null || true
            fi
        fi

        return 0
    else
        log_error "Build failed for $platform_name"
        return 1
    fi
}

# Build for all targets
build_all() {
    log_info "Building PicoFlow for all platforms ($BUILD_TYPE mode)..."
    echo ""

    local failed_builds=()

    for target in "${TARGETS[@]}"; do
        if ! build_target "$target"; then
            failed_builds+=("$target")
        fi
        echo ""
    done

    # Summary
    echo "========================================"
    echo "Build Summary"
    echo "========================================"

    if [[ ${#failed_builds[@]} -eq 0 ]]; then
        log_success "All builds completed successfully!"
        echo ""
        echo "Binaries location:"
        for target in "${TARGETS[@]}"; do
            echo "  $target: target/$target/$BUILD_TYPE/picoflow"
        done
        return 0
    else
        log_error "Some builds failed:"
        for target in "${failed_builds[@]}"; do
            echo "  - $target"
        done
        return 1
    fi
}

# Main execution
main() {
    parse_args "$@"

    echo "========================================"
    echo "PicoFlow Cross-Compilation Build"
    echo "========================================"
    echo "Build type: $BUILD_TYPE"
    echo "Targets: ${#TARGETS[@]}"
    echo ""

    # Check prerequisites
    check_toolchains || exit 1

    # Check linkers (warning only, may use cross-rs)
    check_linkers || log_warning "Continuing without native linkers (may use cross-rs)"

    echo ""

    # Build all targets
    build_all
}

# Run main function
main "$@"
