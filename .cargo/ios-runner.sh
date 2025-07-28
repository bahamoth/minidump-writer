#!/bin/sh -e

# iOS Simulator test runner for minidump-writer
# Runs test binaries in iOS Simulator environment

BINARY_PATH="$1"
BINARY_NAME=$(basename "$BINARY_PATH")
shift

# Configuration
BUNDLE_ID="com.rust-minidump.test"
APP_NAME="test.app"
BUNDLE_DIR="/tmp/minidump-ios-test/$APP_NAME"

# Clean up previous runs
rm -rf /tmp/minidump-ios-test
mkdir -p "$BUNDLE_DIR"

# Create minimal Info.plist
cat > "$BUNDLE_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$BINARY_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>$BUNDLE_ID</string>
    <key>LSRequiresIPhoneOS</key>
    <true/>
</dict>
</plist>
EOF

# Copy and sign the binary
cp "$BINARY_PATH" "$BUNDLE_DIR/"
codesign -s - "$BUNDLE_DIR/$BINARY_NAME"

# Find or create simulator
DEVICE_NAME="minidump-test-iPhone"
DEVICE_TYPE="iPhone 15"

# Get available runtime (prefer latest iOS)
RUNTIME=$(xcrun simctl list runtimes iOS | grep -Eo "iOS.*" | sort -V | tail -1 | xargs)
if [ -z "$RUNTIME" ]; then
    echo "Error: No iOS runtime found" >&2
    exit 1
fi

# Create device if needed
DEVICE_ID=$(xcrun simctl list devices | grep "$DEVICE_NAME" | grep -oE "[A-F0-9-]{36}" | head -1)
if [ -z "$DEVICE_ID" ]; then
    DEVICE_ID=$(xcrun simctl create "$DEVICE_NAME" "$DEVICE_TYPE" "$RUNTIME")
fi

# Boot device if needed
xcrun simctl boot "$DEVICE_ID" 2>/dev/null || true

# Wait for boot
xcrun simctl bootstatus "$DEVICE_ID" -b >/dev/null 2>&1

# Uninstall previous version
xcrun simctl uninstall "$DEVICE_ID" "$BUNDLE_ID" 2>/dev/null || true

# Install and run
xcrun simctl install "$DEVICE_ID" "$BUNDLE_DIR"

# Run the test and capture exit code
# Use spawn instead of launch to get proper exit code
set +e
xcrun simctl spawn "$DEVICE_ID" "$BUNDLE_ID/$BINARY_NAME" "$@"
EXIT_CODE=$?
set -e

# Cleanup
xcrun simctl uninstall "$DEVICE_ID" "$BUNDLE_ID" 2>/dev/null || true
xcrun simctl shutdown "$DEVICE_ID" 2>/dev/null || true
rm -rf /tmp/minidump-ios-test

exit $EXIT_CODE