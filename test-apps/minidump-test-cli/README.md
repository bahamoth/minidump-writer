# Minidump Test CLI

A simple command-line tool for testing minidump generation across platforms.

## Purpose

This tool is designed for internal testing of the minidump-writer library. It is intentionally excluded from the main workspace to avoid CI/CD complications.

## Building

### iOS Simulator (ARM64)
```bash
cargo build --target aarch64-apple-ios-sim
```

### iOS Simulator (x86_64)
```bash
cargo build --target x86_64-apple-ios-sim
```

### macOS
```bash
cargo build --target aarch64-apple-darwin  # Apple Silicon
cargo build --target x86_64-apple-darwin   # Intel
```

### Linux
```bash
cargo build --target x86_64-unknown-linux-gnu
```

## Usage

### Generate a minidump without crashing
```bash
minidump-test-cli dump
```

### Create multiple threads then dump
```bash
minidump-test-cli threads 10
```

### Trigger a crash (not recommended on iOS)
```bash
minidump-test-cli crash segfault
minidump-test-cli crash abort
minidump-test-cli crash illegal
```

### Options
- `--output <path>`: Specify output file path
- `--debug`: Enable debug output

## Default Output Locations

- **iOS/macOS**: `~/Documents/minidump_YYYYMMDD_HHMMSS.dmp`
- **Other platforms**: `./minidump_YYYYMMDD_HHMMSS.dmp`

## Running on iOS Simulator

### Quick Build
```bash
# Use the build script (auto-detects architecture)
./build-ios-sim.sh
```

### Manual Build
```bash
# Apple Silicon Mac
RUSTFLAGS="-C target-sdk-version=12.0" cargo build --target aarch64-apple-ios-sim

# Intel Mac
RUSTFLAGS="-C target-sdk-version=12.0" cargo build --target x86_64-apple-ios-sim
```

### Running in Simulator
```bash
# Make sure a simulator is running
xcrun simctl list devices | grep Booted

# Run the test CLI
xcrun simctl spawn booted $PWD/target/aarch64-apple-ios-sim/debug/minidump-test-cli dump

# Test with crash (generates minidump via signal handler)
xcrun simctl spawn booted $PWD/target/aarch64-apple-ios-sim/debug/minidump-test-cli crash segfault --debug

# Check output
ls ~/Documents/minidump_*.dmp
```

### iOS Crash Handler
The iOS version includes a signal handler that:
- Catches SIGSEGV, SIGABRT, SIGILL, and SIGBUS
- Generates a minidump with crash context
- Saves to ~/Documents/minidump_TIMESTAMP.dmp
- Re-raises the signal for default handling

## Notes

- iOS crash handler is implemented using signal handlers (SIGSEGV, SIGABRT, etc.)
- Signal handlers have limitations - they cannot use heap allocation or most system calls
- The iOS implementation uses a temporary crash context until crash-context crate adds iOS support
- This tool is for testing purposes only and should not be distributed