# minidump-writer – Tasks

> **DOCGUIDE**: This document tracks all project tasks with agent assignments and state transitions. Agents must identify themselves before claiming tasks. Task IDs are permanent and monotonic.

## Part 1: Task Management Protocol

### Agent Identification

Before claiming any task, agents must identify themselves:
- **Format**: `role-name` (e.g., `dev-alice`, `qa-bob`, `ai-claude`)
- **Query**: "What is your role and name?"
- **Recording**: Update Assignee field when ownership changes

### Task ID Scheme

- **Format**: `T-###` (T-001, T-002, etc.)
- **Assignment**: Monotonic, never reuse IDs
- **Deletion**: Never delete tasks, use "Dropped" status instead

### Task States

| Status | Description | Next States |
|--------|-------------|-------------|
| TODO | Not started | DOING, Dropped |
| DOING | In progress | DONE, Blocked, TODO |
| DONE | Completed | - |
| Blocked | Waiting on external factor | DOING, Dropped |
| Dropped | No longer needed | - |

### Task Structure

```markdown
| ID | Title | Status | Assignee | Links | Notes |
|----|-------|--------|----------|-------|-------|
| T-001 | Clear task description | TODO | role-name | R-123, #456 | Brief context |
```

### Assignment Rules

1. **Claiming**: Update Assignee from `-` to your `role-name`
2. **Handoff**: Record transition in Notes (e.g., "dev-alice → dev-bob")
3. **Completion**: Keep Assignee for attribution
4. **Blocked**: Note blocker in Notes field

### Integration Points

- **Requirements**: Link to R-IDs from PRD.md
- **Decisions**: Reference D-IDs from DECISIONS.md
- **Issues/PRs**: Use #123 format for GitHub references
- **Branches**: Note agent worktree if applicable

## Part 2: Active Tasks

### Current Sprint

| ID | Title | Status | Assignee | Links | Notes |
|----|-------|--------|----------|-------|-------|
| T-001 | Implement iOS MinidumpWriter struct | DONE   | dev-victor | R-001, R-002 | Core writer for iOS platform |
| T-003 | Implement iOS TaskDumper | DONE   | architect-strange | R-005 | Adapt from macOS with iOS constraints |
| T-004 | Create iOS system info collector | DONE   | architect-strange | R-009, #5 | Device model, OS version, architecture. **Architectural issues found**: 1) crash-context crate doesn't support iOS - need custom CrashContext, 2) Fixed platform ID from 0x8000 to PlatformId::Ios (0x8102) |
| T-005 | Write iOS thread state dumper | DONE   | architect-strange | R-006 | ARM64 register state capture |
| T-006 | Add iOS memory region mapper | DONE   | architect-t'challa | R-007 | Implemented memory list stream using existing get_vm_region for sandbox-safe memory access |
| T-009 | Write iOS-specific tests | DOING  | architect-forge | R-012 | Unit and integration tests. Refactoring implementation to fix underlying issues.
| T-010 | Add iOS simulator support | TODO   | - | R-011 | Feature flag for x86_64 builds |
| T-011 | Document iOS platform limitations | TODO   | - | R-013 | Update README and docs |
| T-012 | Create iOS example app | TODO   | - | R-014 | Swift/ObjC integration demo |
| T-013 | Implement iOS CrashContext | DONE   | architect-forge | R-001, R-002 | **BLOCKER**: crash-context crate doesn't support iOS. Need custom implementation for iOS MinidumpWriter |
| T-014 | Fix iOS implementation compilation errors | DONE   | architect-forge | T-010, D-2025-07-18-01 | Merged into T-009. Was: Fix import paths, API compatibility, and architecture issues preventing iOS simulator builds |
| T-015 | Refactor iOS stream count to dynamic calculation | TODO   | - | #12 | Currently hardcoded as 4. Should follow macOS pattern using writers array for better maintainability |
| T-016 | Implement function pre-binding for signal safety | TODO   | - | D-2025-07-28-01 | Pre-bind all lazy-bound functions before signal handler installation to avoid dyld deadlocks. Required for both macOS and iOS implementations per ARCHITECTURE.md guidelines |
| T-017 | Implement iOS module list stream | DOING  | architect-hawkeye | T-015, #13 | Add missing module list stream to iOS implementation. Includes fixing stream count to be dynamic. PR submitted for review |
| T-018 | Fix iOS register values not captured | DOING  | architect-hawkeye | #13 | Debug and fix thread_state reading issues causing empty register values in iOS simulator. PR submitted for review |
| T-019 | Fix iOS module base address calculation for accurate symbolication | DONE | architect-hawkeye | T-017 | Fix ASLR slide calculation in module_list.rs. Current base_of_image = load_address is incorrect, should be (vm_addr + slide). Fixed in PR fix-ios-address-accuracy |
| T-020 | Add missing streams to iOS implementation | TODO | - | T-019 | Add Breakpad Info, Thread Names, and Misc Info streams to match macOS functionality |


### Task Assignment History

```
2025-07-17: Tasks document created
- T-001 to T-012: Initial iOS support tasks created
2025-07-17: T-004: Status TODO → DOING (claimed by dev-zatanna)
2025-07-17: T-001: Status TODO → DOING (claimed by dev-victor)
2025-07-17: T-004: Status DOING → DONE (completed by dev-zatanna)
2025-07-21: T-004: Assignee dev-zatanna → architect-strange (task reassigned)
2025-07-22: T-013: Created new blocker task for iOS CrashContext implementation
2025-07-23: T-003: Assigned to architect-strange
2025-07-23: T-003: Status TODO → DOING (claimed by architect-strange)
2025-07-24: T-005: Status TODO → DOING (claimed by architect-strange)
2025-07-24: T-006: Status TODO → DOING (claimed by architect-t'challa)
2025-07-24: T-006: Status DOING → DONE (completed by architect-t'challa)
2025-07-25: T-009: Status TODO → DOING (claimed by architect-strange)
2025-07-26: T-014: Created task for fixing iOS compilation errors
2025-07-26: T-014: Status TODO → DOING (claimed by architect-forge)
2025-07-27: T-009: Assignee architect-strange → architect-forge (taking over to fix underlying test issues)
2025-07-27: T-014: Status DOING → Dropped (merged into T-009)
2025-07-28: T-015: Created task for iOS stream count refactoring based on PR review feedback
2025-07-28: T-016: Created task for function pre-binding implementation (architect-vision)
2025-07-30: T-017: Created task for iOS module list stream implementation (architect-hawkeye)
2025-07-30: T-018: Created task for iOS register value fix (architect-hawkeye)
2025-07-30: T-017: Status TODO → DOING (claimed by architect-hawkeye)
2025-07-30: T-017: Status DOING → DONE (completed by architect-hawkeye)
2025-07-30: T-018: Status TODO → DOING (claimed by architect-hawkeye)
2025-07-30: T-017: Status DOING → DONE (completed by architect-hawkeye)
2025-07-30: T-019: Status TODO → DONE (completed by architect-hawkeye)
```

## Templates

### New Task Template

```markdown
| T-0XX | [Description] | TODO | - | [Links] | [Context] |
```

### Status Update Template

```
[Date] T-XXX: Status TODO → DOING (claimed by role-name)
[Date] T-XXX: Status DOING → DONE (completed by role-name)
```

## Related Documents

- **Requirements**: See PRD.md for detailed requirements (R-IDs)
- **Architecture**: See ARCHITECTURE.md for system design
- **Decisions**: See DECISIONS.md for technical choices (D-IDs)
- **Conventions**: See CONVENTIONS.md for coding standards
- **Overview**: See OVERVIEW.md for project summary

## Notes

- Tasks are derived from iOS support requirements in PRD.md
- Platform-specific implementations follow existing patterns
- Signal-safety is critical for crash handling tasks
- iOS has unique constraints compared to macOS
- Coordinate with existing team members before claiming tasks

### Critical iOS Implementation Issues (2025-07-22)

1. **iOS CrashContext Required**: The crash-context crate (v0.6) does not support iOS. iOS MinidumpWriter currently imports `crash_context::CrashContext` which will fail to compile. A custom iOS-specific CrashContext implementation is needed.

2. **Platform ID Correction**: The initial implementation used 0x8000 (Unix) as the platform ID. This has been corrected to use `PlatformId::Ios` (0x8102) from the minidump-common crate for proper platform identification.

3. **Architecture Alignment**: All iOS components (T-003 through T-008) should follow the established macOS patterns while respecting iOS constraints (self-process only, sandbox restrictions, signal safety requirements).