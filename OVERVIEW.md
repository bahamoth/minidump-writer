# minidump-writer – Overview

> Rust rewrite of Breakpad's minidump_writer

<!--DOCGUIDE
type: overview
audience: humans+llm
purpose: Entry point; short context; where to go next.
authority:
  - Not build-authoritative; see build docs.
  - Not requirements-authoritative; see PRD.md.
llm_instructions:
  - Use Repo Map to locate code accurately.
  - Check project type before suggesting commands.
  - Reference linked docs for detailed information.
-->

> **ID Quick Ref:** `G#` Goal · `R-#` Requirement (PRD) · `T-#` Task (TASKS) · `D-YYYY-MM-DD-##` Decision (DECISIONS).  
> Tasks *may* reference Requirements via optional `ReqIDs` column; blank = exploratory/infra.  
> Full scheme in [PRD.md §IDs & Traceability](./PRD.md#id-scheme).

---

## What is minidump-writer?

minidump-writer is a Rust implementation of Breakpad's minidump_writer client. It creates minidump files for analyzing crashes, primarily designed for dumping external processes (though it can also dump local processes). The library provides platform-specific implementations for Linux, Windows, and macOS, with iOS support currently under development.

Key features:
- Cross-platform crash dump generation (Linux, Windows, macOS)
- External process dumping via platform-specific APIs (ptrace, Windows Debug API, Mach)
- Integration with crash-context crate for detailed crash information
- Modular section-based minidump writing
- Memory-efficient streaming writes

---

## Repo Map

| Path | Role | Lang | Notes |
|------|------|------|-------|
| `src/` | Core library code | Rust | Platform abstraction and shared components |
| `src/lib.rs` | Main entry point | Rust | Uses cfg_if! for platform selection |
| `src/linux/` | Platform: Linux/Android | Rust | ptrace-based implementation |
| `src/linux/sections/` | Linux minidump sections | Rust | Thread, memory, system info writers |
| `src/windows/` | Platform: Windows | Rust | Windows Debug API implementation |
| `src/mac/` | Platform: macOS | Rust | Mach kernel interface implementation |
| `src/mac/streams/` | macOS minidump streams | Rust | Platform-specific section writers |
| `src/ios/` | Platform: iOS (planned) | Rust | iOS-specific implementation (TBD) |
| `src/bin/` | Binary targets | Rust | Executable tools |
| `tests/` | Test suite | Rust | Unit and integration tests |
| `examples/` | Usage examples | Rust | Sample implementations |
| `build.rs` | Build script | Rust | Windows dbghelp.dll linking |
| `CLAUDE.md` | AI assistant guide | Markdown | Development instructions |
| `PRD.md` | Product requirements | Markdown | iOS support requirements |
| `TASKS.md` | Development tasks | Markdown | iOS implementation roadmap |

---

## Quick Start

### Prerequisites
- Rust toolchain (1.70+)
- Platform-specific dependencies:
  - Linux: libc, ptrace support
  - Windows: Windows SDK (dbghelp.dll)
  - macOS: Xcode Command Line Tools
  - iOS: Xcode, iOS SDK (planned)

### Building
```bash
# Build the project
cargo build

# Run tests
cargo test

# Run with optimizations
cargo build --release

# Check code without building
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Basic Usage
```rust
// Example: Dump external process
use minidump_writer::{minidump_writer::MinidumpWriter, crash_context::CrashContext};

// Linux example
#[cfg(target_os = "linux")]
let mut writer = MinidumpWriter::new(process_id, blame_thread_id);
writer.dump(&mut crash_context, &mut output_file)?;
```

---

## Development Workflow

1. **Setup**: Clone repo, ensure Rust toolchain is installed
2. **Development**: 
   - Use `cargo check` for quick validation
   - Run `cargo test` frequently
   - Use `cargo clippy` before commits
3. **Testing**: 
   - Unit tests: `cargo test`
   - Platform tests: `cargo test --target <platform>`
   - Integration tests spawn external processes
4. **Documentation**: Update TASKS.md for progress tracking

---

## Architecture Notes

- **Platform Abstraction**: The main `lib.rs` uses conditional compilation to expose platform-specific implementations transparently
- **External Process Focus**: Designed primarily for dumping crashes in external processes rather than self-dumping
- **Section-Based Writing**: Minidumps are composed of sections (threads, memory regions, system info) written independently
- **Memory Safety**: Uses memory-mapped files and bounded writers to prevent corruption
- **Crash Context Integration**: Leverages the `crash-context` crate for signal-safe crash information collection

---

## Linked Documentation

- [README.md](./README.md) - Project introduction and setup
- [CLAUDE.md](./CLAUDE.md) - AI assistant instructions
- [PRD.md](./PRD.md) - Product requirements (iOS support)
- [TASKS.md](./TASKS.md) - Development tasks
- [CHANGELOG.md](./CHANGELOG.md) - Version history

---

_Last updated: 2025-07-17_