#!/usr/bin/env bash
#
# docker-build.sh - Build PicoFlow using Docker for consistent cross-compilation
#
# This script uses Docker to build PicoFlow in a consistent environment
# with all necessary cross-compilation toolchains pre-installed.
#
# Usage:
#   ./scripts/docker-build.sh [TARGET]
#
# Arguments:
#   TARGET - Platform to build (all, arm32, arm64, x86_64). Default: all
#
# Examples:
#   ./scripts/docker-build.sh           # Build for all platforms
#   ./scripts/docker-build.sh arm32     # Build for ARM 32-bit only
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

TARGET="${1:-all}"
IMAGE_NAME="picoflow-builder"

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

# Check if Docker is installed
check_docker() {
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        echo "Please install Docker: https://docs.docker.com/get-docker/"
        exit 1
    fi

    if ! docker info &> /dev/null; then
        log_error "Docker daemon is not running"
        echo "Please start Docker and try again"
        exit 1
    fi

    log_success "Docker is available"
}

# Build Docker image
build_image() {
    log_info "Building Docker image: $IMAGE_NAME"

    if docker build -f Dockerfile.build -t "$IMAGE_NAME" .; then
        log_success "Docker image built successfully"
    else
        log_error "Failed to build Docker image"
        exit 1
    fi
}

# Check if image exists
check_image() {
    if docker images "$IMAGE_NAME" | grep -q "$IMAGE_NAME"; then
        log_info "Using existing Docker image: $IMAGE_NAME"
        return 0
    else
        log_warning "Docker image not found, building..."
        build_image
    fi
}

# Run build in Docker
run_build() {
    log_info "Building PicoFlow in Docker container (target: $TARGET)..."

    # Get absolute path to current directory
    WORKSPACE_DIR="$(pwd)"

    # Run Docker container with volume mount
    if docker run --rm \
        -v "$WORKSPACE_DIR:/workspace" \
        -e CARGO_HOME=/workspace/target/.cargo \
        "$IMAGE_NAME" \
        "$TARGET"; then
        log_success "Build completed successfully"

        echo ""
        echo "========================================"
        echo "Build Results"
        echo "========================================"
        if [ -d "target/release-binaries" ]; then
            ls -lh target/release-binaries/
        fi
    else
        log_error "Build failed"
        exit 1
    fi
}

# Main execution
main() {
    echo "========================================"
    echo "PicoFlow Docker Build"
    echo "========================================"
    echo "Target: $TARGET"
    echo ""

    check_docker
    check_image
    echo ""

    run_build

    echo ""
    log_success "Docker build completed!"
    echo ""
    echo "Binaries available in: target/release-binaries/"
    echo ""
    echo "To rebuild the Docker image:"
    echo "  docker build -f Dockerfile.build -t $IMAGE_NAME ."
}

# Run main function
main
