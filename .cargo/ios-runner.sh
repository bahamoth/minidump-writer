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
    <key>CFBundleName</key>
    <string>MinidumpWriterTest</string>
    <key>CFBundleDisplayName</key>
    <string>MinidumpWriterTest</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSRequiresIPhoneOS</key>
    <true/>
    <key>UIRequiredDeviceCapabilities</key>
    <array>
        <string>arm64</string>
    </array>
</dict>
</plist>
EOF

# Copy and sign the binary
cp "$BINARY_PATH" "$BUNDLE_DIR/"
codesign -s - "$BUNDLE_DIR/$BINARY_NAME"

# Find or create simulator
DEVICE_NAME="minidump-test-iPhone"
DEVICE_TYPE="iPhone 15"

# Get available runtime
# First try to get the runtime ID directly
RUNTIME=$(xcrun simctl list runtimes | grep -m1 -Eo "com.apple.CoreSimulator.SimRuntime.iOS-[0-9-]+" || echo "")

# If that fails, try to find any iOS runtime
if [ -z "$RUNTIME" ]; then
    RUNTIME=$(xcrun simctl list runtimes | grep iOS | head -1 | awk -F' - ' '{print $3}' | xargs)
fi

if [ -z "$RUNTIME" ]; then
    echo "Error: No iOS runtime found" >&2
    echo "Available runtimes:" >&2
    xcrun simctl list runtimes >&2
    exit 1
fi

echo "Using runtime: $RUNTIME"

# Create device if needed
DEVICE_ID=$(xcrun simctl list devices | grep "$DEVICE_NAME" | grep -oE "[A-F0-9-]{36}" | head -1)
if [ -z "$DEVICE_ID" ]; then
    # Try to create device with the runtime
    DEVICE_ID=$(xcrun simctl create "$DEVICE_NAME" "$DEVICE_TYPE" "$RUNTIME" 2>&1)
    if [ $? -ne 0 ]; then
        echo "Failed to create device with runtime: $RUNTIME" >&2
        echo "Error: $DEVICE_ID" >&2
        echo "Trying with first available iPhone device type..." >&2
        
        # Get first available iPhone device type
        DEVICE_TYPE=$(xcrun simctl list devicetypes | grep iPhone | head -1 | awk -F' (' '{print $2}' | tr -d ')')
        if [ -z "$DEVICE_TYPE" ]; then
            DEVICE_TYPE="com.apple.CoreSimulator.SimDeviceType.iPhone-15"
        fi
        
        DEVICE_ID=$(xcrun simctl create "$DEVICE_NAME" "$DEVICE_TYPE" "$RUNTIME" 2>/dev/null || echo "")
    fi
    
    # Extract device ID from output
    DEVICE_ID=$(echo "$DEVICE_ID" | grep -oE "[A-F0-9-]{36}" | head -1)
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