---
title: minidump-writer – Decisions
version: v2025-07-17
status: Active
owner: architect-claude
updated: 2025-07-17
---

<!--DOCGUIDE
type: decisions
audience: humans+llm
purpose: Capture WHY architectural and technical decisions were made
authority:
  - Append-only log (never delete entries)
  - Authoritative source for Decision IDs (D-YYYY-MM-DD-##)
  - Links to Requirements (R-###) and Tasks (T-###)
llm_instructions:
  - ALWAYS ask agents: "What is your role and name?" before recording decisions
  - Record all participating agents in decisions
  - Never modify past decisions, only supersede with new entries
  - Create new D-ID when resolving architect disputes
  - Reference specific proposals and rationale
  - Include agent worktree references (e.g., architect-stark/T-123)
-->

# Part 1: Decision Recording Protocol

## Agent Identification
Every decision MUST record participating agents using the role-name format:
- **Format**: `role-name` (e.g., architect-stark, coder-parker, boss-human)
- **Getting ID**: Ask "What is your role and name?" at session start
- **Roles**: architect, coder, reviewer, boss, analyst, etc.

## Decision ID Scheme
- **Format**: `D-YYYY-MM-DD-##` where ## increments daily
- **Example**: D-2025-07-17-01 (first decision on July 17, 2025)
- **Permanence**: IDs are permanent; never reuse

## Decision Template
```markdown
## D-YYYY-MM-DD-## - [Decision Title]

**Status**: Proposed | Approved | Deprecated | Superseded
**Date**: YYYY-MM-DD
**Participants**:
  - Proposers: [agent-role-name, ...]
  - Decider: [agent-role-name]
  - Implementer: [agent-role-name]

**Context**: 
[Problem statement requiring decision]

**Options Considered**:
1. **Option A** (proposed by agent-name)
   - [Description]
   - Pros: [List]
   - Cons: [List]

2. **Option B** (proposed by agent-name)
   - [Description]
   - Pros: [List]
   - Cons: [List]

**Decision**: 
[Chosen option and rationale by deciding agent]

**Rationale**:
[Why this option was selected over others]

**Consequences**:
- [Positive impacts]
- [Negative impacts or trade-offs]
- [Follow-up tasks needed]

**Relates-To**: R-###, T-###
**Supersedes**: D-YYYY-MM-DD-## (if applicable)
```

## Recording Guidelines
1. **Append-Only**: Never delete or modify existing decisions
2. **Ordering**: Add new decisions at the top of Part 2
3. **Superseding**: Mark old decisions as "Superseded" with reference
4. **Cross-References**: Always link to relevant R-IDs and T-IDs
5. **Conflicts**: When architects disagree, record all viewpoints

---

# Part 2: Decision Log

## Index
| ID | Date | Title | Status | Decider | Relates-To |
|----|------|-------|--------|---------|------------|
| D-2025-07-18-01 | 2025-07-18 | Apple Common Module Restructuring | Approved | architect-stark | T-001, PR#1 |
| D-2025-07-17-01 | 2025-07-17 | iOS Architecture Pattern Selection | Approved | architect-claude | R-100, R-101, R-102 |

---

## D-2025-07-18-01 - Apple Common Module Restructuring

**Status**: Approved  
**Date**: 2025-07-18  
**Participants**:
  - Proposers: architect-stark, boss-human
  - Decider: architect-stark
  - Implementer: architect-stark

**Context**: 
Initial iOS implementation (PR#1) revealed significant code duplication between macOS and iOS. The macOS MinidumpWriter already supports self-process dumping, making a separate iOS implementation unnecessary. Need to restructure to share common code while maintaining backward compatibility.

**Options Considered**:
1. **Keep Separate iOS Implementation** (initial approach)
   - Maintain src/ios/ with duplicate code
   - Pros: Clear separation, no risk to existing macOS code
   - Cons: Code duplication, maintenance burden, memory leak already found

2. **Direct Reuse of macOS Code** (proposed by architect-stark)
   - Use macOS MinidumpWriter directly for iOS
   - Pros: No duplication, proven code
   - Cons: iOS-specific constraints mixed with macOS code

3. **Apple Common Module Pattern** (proposed by boss-human)
   - Create apple/common for shared code, apple/mac and apple/ios for platform-specific
   - Maintain backward compatibility through re-exports
   - Pros: Clean architecture, code reuse, maintainable, Rust idiomatic
   - Cons: Requires careful restructuring

**Decision**: 
Selected Option 3: Apple Common Module Pattern. This provides the best balance of code reuse, maintainability, and platform-specific customization.

**Rationale**:
- Eliminates code duplication discovered in PR#1 review
- Follows Rust idiomatic patterns for platform abstraction
- Maintains 100% backward compatibility through re-exports
- Allows iOS-specific adaptations without polluting macOS code
- Fixes memory leak (vm_deallocate) in shared code benefits both platforms

**Consequences**:
- Positive: Clean architecture, reduced maintenance, shared bug fixes
- Positive: Better code organization following Rust conventions
- Negative: Initial complexity in restructuring
- Follow-up: iOS-specific TaskDumper still needed for platform constraints

**Implementation Details** (commit 90e4c70b):
- Created src/apple/ module structure:
  - apple/common/ - shared implementations
  - apple/mac/ - macOS-specific code  
  - apple/ios/ - iOS-specific code (deleted after migration)
- Maintained backward compatibility:
  - src/mac.rs re-exports apple::mac
  - src/lib.rs provides original public API
- Fixed memory leak in TaskDumper::read_thread_info
- All tests pass without modification

**Relates-To**: T-001, PR#1
**Supersedes**: None

---

## D-2025-07-17-01 - iOS Architecture Pattern Selection

**Status**: Approved
**Date**: 2025-07-17
**Participants**:
  - Proposers: architect-claude, architect-stark
  - Decider: architect-claude
  - Implementer: coder-parker

**Context**: 
The iOS implementation needs to adapt the existing macOS architecture while respecting iOS platform constraints including sandboxing, signal safety requirements, and the inability to inspect external processes.

**Options Considered**:
1. **Direct Port from macOS** (proposed by architect-stark)
   - Copy macOS implementation with minimal changes
   - Pros: Fast implementation, code reuse, proven patterns
   - Cons: Won't work due to iOS restrictions, external process inspection impossible

2. **Self-Process Only with Signal Safety** (proposed by architect-claude)
   - Focus on self-process dumping with signal-safe implementation
   - Use pre-allocated buffers and async-signal-safe functions
   - Pros: Works within iOS constraints, crash-safe, follows platform guidelines
   - Cons: Limited to self-process, more complex implementation

3. **Hybrid with Runtime Detection** (proposed by architect-stark)
   - Detect capabilities at runtime and adapt behavior
   - Pros: Flexible, could support jailbroken devices
   - Cons: Complex, App Store rejection risk, maintenance burden

**Decision**: 
Selected Option 2: Self-Process Only with Signal Safety. This approach respects iOS platform constraints while providing reliable crash reporting capabilities.

**Rationale**:
- iOS sandboxing makes external process inspection impossible
- Signal safety is critical for crash handling reliability
- Aligns with App Store guidelines and security model
- Similar approach used by successful iOS crash reporters

**Consequences**:
- Positive: Reliable crash capture, App Store compliant, maintainable
- Negative: Limited to self-process dumps, no external process support
- Follow-up: Implement pre-allocated buffer system (T-201), signal-safe writers (T-202)

**Relates-To**: R-100, R-101, R-102
**Supersedes**: None

---

## Related Documents
- **PRD.md**: Product requirements (WHAT to build)
- **ARCHITECTURE.md**: Technical design (HOW it's built)
- **TASKS.md**: Implementation tracking (execution)
- **CONVENTIONS.md**: Coding standards and patterns