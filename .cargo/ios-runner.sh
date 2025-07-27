#!/bin/sh -e

# iOS Simulator test runner for minidump-writer
# Directly runs test binaries without app bundle

BINARY_PATH="$1"
shift

# Find or create simulator
DEVICE_NAME="minidump-test-iPhone"

# Check if device already exists
DEVICE_ID=$(xcrun simctl list devices | grep "$DEVICE_NAME" | grep -oE "[A-F0-9-]{36}" | head -1)

if [ -z "$DEVICE_ID" ]; then
    # Get first available iOS runtime
    RUNTIME=$(xcrun simctl list runtimes | grep iOS | head -1 | awk -F' - ' '{print $3}' | xargs)
    if [ -z "$RUNTIME" ]; then
        echo "Error: No iOS runtime found" >&2
        exit 1
    fi
    
    echo "Creating device with runtime: $RUNTIME"
    
    # Create device with generic iPhone type
    DEVICE_ID=$(xcrun simctl create "$DEVICE_NAME" "iPhone" "$RUNTIME" 2>&1 || echo "")
    
    # Extract device ID if creation succeeded
    if echo "$DEVICE_ID" | grep -q "Invalid device type"; then
        # Try with specific device type - extract the identifier in parentheses
        DEVICE_TYPE=$(xcrun simctl list devicetypes | grep "iPhone SE" | head -1 | sed 's/.*(\(.*\))/\1/')
        if [ -z "$DEVICE_TYPE" ]; then
            # Try any iPhone device type
            DEVICE_TYPE=$(xcrun simctl list devicetypes | grep iPhone | head -1 | sed 's/.*(\(.*\))/\1/')
        fi
        
        if [ -n "$DEVICE_TYPE" ]; then
            DEVICE_ID=$(xcrun simctl create "$DEVICE_NAME" "$DEVICE_TYPE" "$RUNTIME")
        else
            echo "Error: Could not find suitable device type" >&2
            echo "Available device types:" >&2
            xcrun simctl list devicetypes >&2
            exit 1
        fi
    fi
    
    DEVICE_ID=$(echo "$DEVICE_ID" | grep -oE "[A-F0-9-]{36}" | head -1)
fi

echo "Using device: $DEVICE_ID"

# Boot device if needed
xcrun simctl boot "$DEVICE_ID" 2>/dev/null || true

# Wait for boot
echo "Waiting for device to boot..."
xcrun simctl bootstatus "$DEVICE_ID" -b

# Run the test binary directly
echo "Running tests..."
xcrun simctl spawn "$DEVICE_ID" "$BINARY_PATH" "$@"
EXIT_CODE=$?

# Shutdown device
xcrun simctl shutdown "$DEVICE_ID" 2>/dev/null || true

exit $EXIT_CODE