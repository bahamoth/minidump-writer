#!/bin/bash
# Build script for iOS Simulator

set -e

# Detect architecture
ARCH=$(uname -m)

if [ "$ARCH" = "arm64" ]; then
    # Apple Silicon Mac
    TARGET="aarch64-apple-ios-sim"
else
    # Intel Mac
    TARGET="x86_64-apple-ios-sim"
fi

echo "Building for iOS Simulator target: $TARGET"

# Build with iOS features enabled
cargo build --target $TARGET

echo "Build complete!"
echo "Binary location: target/$TARGET/debug/minidump-test-cli"
echo ""
echo "To run in simulator:"
echo "xcrun simctl spawn booted $PWD/target/$TARGET/debug/minidump-test-cli dump"