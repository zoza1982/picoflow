#!/bin/bash
set -euo pipefail

TARGET="${1:-all}"

build_target() {
    local target=$1
    local platform=$2

    echo "========================================"
    echo "Building for $platform ($target)"
    echo "========================================"

    # Set target-specific environment variables for cross-compilation
    case "$target" in
        armv7-unknown-linux-gnueabihf)
            export CC_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-gcc
            export CXX_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-g++
            export AR_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-ar
            export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc
            ;;
        aarch64-unknown-linux-gnu)
            export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
            export CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++
            export AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar
            export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
            ;;
    esac

    cargo build --release --target "$target" --locked

    # Get binary info
    BINARY="target/$target/release/picoflow"
    if [ -f "$BINARY" ]; then
        SIZE=$(stat -c%s "$BINARY")
        SIZE_MB=$(echo "scale=2; $SIZE / 1024 / 1024" | bc)
        echo "Binary size: ${SIZE_MB}MB"

        # Copy to output directory with platform name
        mkdir -p /workspace/target/release-binaries
        cp "$BINARY" "/workspace/target/release-binaries/picoflow-${platform}"
        chmod +x "/workspace/target/release-binaries/picoflow-${platform}"
        echo "Binary copied to: target/release-binaries/picoflow-${platform}"
    else
        echo "Error: Binary not found at $BINARY"
        return 1
    fi

    echo ""
}

case "$TARGET" in
    all)
        build_target "armv7-unknown-linux-gnueabihf" "arm32"
        build_target "aarch64-unknown-linux-gnu" "arm64"
        build_target "x86_64-unknown-linux-gnu" "x86_64"
        ;;
    arm32)
        build_target "armv7-unknown-linux-gnueabihf" "arm32"
        ;;
    arm64)
        build_target "aarch64-unknown-linux-gnu" "arm64"
        ;;
    x86_64)
        build_target "x86_64-unknown-linux-gnu" "x86_64"
        ;;
    *)
        echo "Unknown target: $TARGET"
        echo "Usage: docker run picoflow-builder [all|arm32|arm64|x86_64]"
        exit 1
        ;;
esac

echo "========================================"
echo "Build complete!"
echo "========================================"
echo "Binaries available in: target/release-binaries/"
ls -lh /workspace/target/release-binaries/ || true
