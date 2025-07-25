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
| T-009 | Write iOS-specific tests | DOING   | architect-strange | R-012 | Unit and integration tests |
| T-010 | Add iOS simulator support | TODO   | - | R-011 | Feature flag for x86_64 builds |
| T-011 | Document iOS platform limitations | TODO   | - | R-013 | Update README and docs |
| T-012 | Create iOS example app | TODO   | - | R-014 | Swift/ObjC integration demo |
| T-013 | Implement iOS CrashContext | DONE   | architect-forge | R-001, R-002 | **BLOCKER**: crash-context crate doesn't support iOS. Need custom implementation for iOS MinidumpWriter |


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