# CONVENTIONS.md

> **DOCGUIDE**: This document establishes the authoritative conventions and standards for the minidump-writer project. All contributors must follow these conventions to ensure consistency, maintainability, and quality across the codebase.

## Quick Policy Matrix

| Tool/Config | Configuration | Enforcement | CI Check |
|-------------|---------------|-------------|----------|
| **rustfmt** | Default config | Required | ✅ Blocks merge |
| **clippy** | Default lints | Required | ✅ Blocks merge |
| **cargo-deny** | deny.toml | Required | ✅ Daily + PR |
| **Tests** | cargo test | Required | ✅ Multi-platform |
| **MSRV** | Rust 2021 | Required | ✅ CI validates |
| **Commits** | Conventional | Recommended | ⚠️ Manual review |
| **Branches** | Feature branches | Required | ✅ PR workflow |

## General Code Style Principles

1. **Consistency First**: Match existing code patterns in the module
2. **Platform Abstraction**: Use `cfg_if!` for platform-specific code
3. **External Process Focus**: Design APIs for external process dumping
4. **Safety**: Prefer safe Rust, document all `unsafe` blocks
5. **Error Handling**: Use `thiserror` for custom errors
6. **Documentation**: Document public APIs and platform differences

## Language-Specific Conventions

### Rust Conventions

#### Formatting (Required)
- Use default `rustfmt` settings
- Run `cargo fmt` before committing
- CI enforces formatting

#### Linting (Required)
- All clippy warnings must be addressed
- Use `#[allow(...)]` sparingly with justification
- Run `cargo clippy` locally before pushing

#### Code Organization
```rust
// Module structure by platform
src/
├── linux/          // Linux/Android implementation
├── windows/        // Windows implementation  
├── mac/            // macOS implementation
├── ios/            // iOS implementation
└── lib.rs          // Platform conditional exports

// Use cfg_if! for platform selection
cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        // Linux-specific code
    } else if #[cfg(target_os = "windows")] {
        // Windows-specific code
    }
}
```

#### Naming Conventions
- Module names: `snake_case`
- Types/Traits: `PascalCase`
- Functions/Variables: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Platform modules mirror OS names

#### Error Handling
```rust
// Use thiserror for custom errors
#[derive(Debug, thiserror::Error)]
pub enum MinidumpError {
    #[error("Failed to read process memory")]
    MemoryReadError(#[from] std::io::Error),
}

// Propagate errors with ?
// Document error conditions
```

## Build & Packaging Rules

### Standard Build Commands
```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Platform-specific builds
cargo build --target x86_64-pc-windows-msvc
cargo build --target x86_64-apple-darwin
cargo build --target aarch64-apple-ios

# Feature flags
cargo build --features ios_simulator
```

### Cross-Compilation
- Use GitHub Actions for tier 2/3 targets
- Android builds require NDK setup
- iOS builds require Xcode toolchain
- Document platform-specific requirements

### Release Process
- Use `cargo-release` with release.toml
- Semantic versioning (MAJOR.MINOR.PATCH)
- Update CHANGELOG.md automatically
- Tag format: `v{version}`

## Test Policy Matrix

| Test Type | Command | Coverage Target | Platform |
|-----------|---------|-----------------|----------|
| Unit Tests | `cargo test` | Core logic | All |
| Integration | `cargo test` | API surface | All |
| Platform Tests | `cargo test -p <platform>` | Platform code | Specific |
| Android Tests | `.cargo/android-runner.sh` | Android API | Emulator |
| Release Tests | `cargo test --release` | Performance | All |

### Testing Requirements
1. **New Features**: Must include tests
2. **Bug Fixes**: Must include regression test
3. **Platform Code**: Test on target platform
4. **Unsafe Code**: Extra test coverage required
5. **External Processes**: Use test-binary helper

### Test Organization
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_functionality() {
        // Unit test
    }
    
    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_specific() {
        // Platform-specific test
    }
}

// Integration tests in tests/
```

## Branch & Commit Conventions

### Branch Naming
- Feature: `feature/<description>`
- Bugfix: `fix/<issue-or-description>`
- Platform: `<platform>-support` (e.g., `ios-support`)
- Release: `release/<version>`

### Git Workflow
- **Branching**: Create feature branches from main
- **Updates**: Rebase feature branches on main
- **Merging**: Fast-forward when possible, squash messy history
- **Cleanup**: Delete branches after merge

### Commit Messages (Conventional Commits)
```
<type>(<scope>): <subject>

<body>

<footer>
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Formatting (rustfmt)
- `refactor`: Code restructuring
- `test`: Test additions/changes
- `chore`: Build/tool updates
- `perf`: Performance improvements

Examples:
```
feat(linux): Add memory mapping optimization
fix(windows): Handle access denied errors correctly
docs: Update platform support matrix
chore(deps): Update minidump-common to 0.21
```

## Dependency Management

### Adding Dependencies
1. Justify the dependency need
2. Check license compatibility (MIT/Apache-2.0)
3. Prefer well-maintained crates
4. Minimize dependency tree
5. Update deny.toml if needed

### Security Scanning
- Daily cargo-deny audits
- Address security advisories immediately
- No duplicate dependencies
- Banned crates: `chrono` (use `time`)

### Version Policy
- Pin major versions in Cargo.toml
- Use `cargo update` cautiously
- Test dependency updates thoroughly
- Document breaking changes

## Security & Compliance

### Code Security
- No hardcoded credentials
- Document all `unsafe` blocks
- Validate external input
- Handle sensitive data carefully
- Follow least privilege principle

### Platform Security
- Linux: Handle ptrace permissions
- Windows: Debug privilege requirements
- macOS: Entitlements for task access
- iOS: Sandbox restrictions

### CI Security
- Dependency audits (cargo-deny)
- No secrets in code
- Secure artifact handling
- Limited third-party actions

## Documentation Requirements

### Code Documentation
```rust
/// Brief description of the function.
///
/// Longer explanation if needed.
///
/// # Arguments
/// * `process_id` - The target process ID
///
/// # Returns
/// * `Ok(Minidump)` - Successfully created minidump
/// * `Err(MinidumpError)` - Various error conditions
///
/// # Platform Notes
/// Linux: Requires ptrace permissions
/// Windows: Requires debug privilege
pub fn dump_process(process_id: u32) -> Result<Minidump> {
    // Implementation
}
```

### File Headers
- Purpose of the module
- Platform-specific behavior
- Key dependencies
- Usage examples

### README Updates
- Keep platform matrix current
- Document new features
- Update examples
- Include breaking changes

## Agent Collaboration

### Agent Identification Protocol
Each AI agent working on this project must:

1. **Identify themselves**:
   - Role: architect, coder, reviewer, tester
   - Name: Choose a unique identifier
   - Example: "I am the architect agent. My name is Stark."

2. **Worktree Pattern**:
   ```bash
   git worktree add -b feature/T-123 ../worktrees/architect-stark/T-123
   ```

### Staged Review Workflow

**Stage 1: Design Competition**
- Multiple architects propose solutions
- Create design docs in worktree
- Boss reviews and selects winner

**Stage 2: Implementation**
- Assigned coder implements chosen design
- Works in isolated worktree
- Pushes to proposal branch

**Stage 3: Polish**
- Reviewers suggest improvements
- Testers validate functionality
- Auto-merge if all checks pass

### Coordination Rules

1. **File Ownership**:
   ```json
   // .agent-info
   {
     "role": "coder",
     "name": "parker",
     "task": "T-123",
     "files": ["src/ios/minidump_writer.rs"]
   }
   ```

2. **Task Assignment**:
   - Update TASKS.md with agent name
   - Mark status: assigned → in_progress → review
   - Never modify another agent's work directly

3. **Communication**:
   - Via commit messages
   - PR comments
   - TASKS.md updates

### Boss-Agent Protocol

1. **Proposal Branches**:
   ```bash
   git push origin proposal/T-123-stark
   ```

2. **Boss Review**:
   ```bash
   git diff proposal/T-123-stark proposal/T-123-rogers
   ```

3. **Merge Decision**:
   - Only Boss merges to main
   - Can combine proposals
   - Documents decision rationale

## Platform-Specific Conventions

### Linux/Android
- Use `nix` crate for system calls
- Handle `/proc` filesystem carefully
- Test with different kernel versions
- Document ptrace requirements

### Windows
- Use official Windows crate bindings
- Handle privileges properly
- Test on Windows 10/11
- Link dbghelp.dll correctly

### macOS
- Use `mach2` for kernel interfaces
- Handle entitlements
- Test on latest two macOS versions
- Document notarization needs

### iOS
- Self-process dumping only
- Handle sandbox restrictions
- Pre-allocate buffers for signal safety
- Test on device and simulator

## Related Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)**: Technical design and structure
- **[CLAUDE.md](CLAUDE.md)**: AI assistant instructions
- **[PRD.md](PRD.md)**: Product requirements (iOS)
- **[TASKS.md](TASKS.md)**: Current work items
- **[README.md](README.md)**: Project overview

## Enforcement

These conventions are enforced through:

1. **Automated CI checks** (Required)
2. **Code review** (Required)  
3. **Developer discipline** (Expected)
4. **Agent coordination** (When applicable)

Non-compliance blocks merging. Exceptions require explicit justification and team approval.