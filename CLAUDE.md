# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

minidump-writer is a Rust rewrite of Breakpad's minidump_writer client. It creates minidump files for analyzing crashes, primarily for external processes (though it can also dump local processes). The project supports Linux, Windows, and macOS platforms with varying levels of implementation maturity.

## Build Commands

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Build for a specific target
cargo build --target <TARGET_TRIPLE>
```

## Testing Commands

```bash
# Run all tests
cargo test

# Run tests for a specific package/module
cargo test -p <PACKAGE_NAME>

# Run a specific test
cargo test <TEST_NAME>

# Run tests without capturing output (useful for debugging)
cargo test -- --nocapture

# Run tests in release mode
cargo test --release
```

## Linting and Code Quality

```bash
# Run clippy for lint checking
cargo clippy

# Run clippy and automatically fix issues
cargo clippy --fix

# Format code
cargo fmt

# Check formatting without making changes
cargo fmt -- --check
```

## Architecture Overview

The codebase is organized by platform with shared components:

### Platform-Specific Implementations

- **Linux/Android** (`src/linux/`): Uses ptrace for process inspection. Key components:
  - `minidump_writer.rs`: Main writer implementation
  - `ptrace_dumper.rs`: Process dumping via ptrace
  - `crash_context.rs`: Platform-specific crash context handling
  - `sections/`: Various minidump section writers

- **Windows** (`src/windows/`): Uses Windows debugging APIs. Key components:
  - `minidump_writer.rs`: Windows-specific writer
  - `ffi.rs`: Windows API bindings
  
- **macOS** (`src/mac/`): Uses Mach kernel interfaces. Key components:
  - `minidump_writer.rs`: macOS-specific writer
  - `task_dumper.rs`: Mach task dumping
  - `mach.rs`: Mach-specific functionality

- **iOS** (`src/ios/`): Adapted from macOS with iOS constraints. Key components:
  - `minidump_writer.rs`: iOS-specific writer (self-process only)
  - `task_dumper.rs`: iOS task dumping with security restrictions
  - `crash_handler.rs`: Signal-safe crash handling with pre-allocated buffers
  - `system_info.rs`: iOS system information gathering

### Shared Components

- `mem_writer.rs`: Memory buffer writing utilities
- `minidump_format.rs`: Minidump format definitions
- `dir_section.rs`: Directory section handling
- `serializers.rs`: Data serialization helpers

### Key Design Patterns

1. **Platform Abstraction**: The main `lib.rs` uses `cfg_if!` to conditionally compile platform-specific modules and re-export their public APIs.

2. **External Process Focus**: The primary use case is dumping external processes, which is more reliable than self-dumping during crashes.

3. **Crash Context Integration**: Integrates with the `crash-context` crate to provide detailed crash information.

4. **Section-Based Writing**: Minidumps are written as sections (threads, memory, system info, etc.) with each platform implementing section writers.

## Development Notes

- The project uses `failspot` for fault injection testing
- Platform support varies - check the README's client status table
- Tests often spawn external processes to test crash dumping
- The Windows build requires linking against `dbghelp.dll` (handled in `build.rs`)
- Integration with `minidump-processor` for validating generated minidumps

## iOS Support Development

For iOS platform support implementation, refer to:

### `PRD.md` - Product Requirements Document
- Detailed technical requirements and constraints
- Signal-safety implementation guidelines
- Platform-specific adaptations needed
- Integration patterns with existing macOS code
- Testing and validation strategies

### `TASKS.md` - Implementation Task List
- High-level functional tasks for iOS support
- Major components to be implemented
- Critical success factors

The PRD serves as the authoritative guide for adding iOS support while maintaining consistency with the existing codebase.

## iOS Build and Testing

### Build Commands for iOS

```bash
# Build for iOS device (ARM64)
cargo build --target aarch64-apple-ios

# Build for iOS simulator (x86_64)
cargo build --target x86_64-apple-ios --features ios_simulator

# Build in release mode
cargo build --target aarch64-apple-ios --release

# Run clippy for iOS
cargo clippy --target aarch64-apple-ios
```

### iOS Testing

Due to iOS platform restrictions, testing requires special approaches:

1. **Unit tests**: Run on host with conditional compilation
2. **Simulator tests**: Use iOS simulator with cargo-dinghy
3. **Device tests**: Require code signing and provisioning profiles
4. **Integration tests**: Use Xcode project with Swift/ObjC bridge

## Task Management

**IMPORTANT**: When working on this project, always update `TASKS.md` to reflect:
- Completed tasks (mark with `[x]`)
- New tasks discovered during implementation
- Priority changes or blockers
- Progress indicators (✅ complete, 🚧 in progress, 📋 planned)

This helps maintain project visibility and ensures no work is lost between sessions.