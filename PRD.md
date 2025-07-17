---
title: iOS Minidump Writer – PRD
version: v2025-07-17
status: Draft
owner: bahamoth
updated: 2025-07-17
---

<!--DOCGUIDE
type: prd
audience: humans+llm
purpose: WHAT & WHY (goals, requirements, scope); not implementation.
authority:
  - Authoritative source for Requirement IDs (R-#).
  - Implementation details live in ARCHITECTURE.md & code.
llm_instructions:
  - Use this file to understand user needs & scope.
  - TASKS may optionally map to Requirements; blanks allowed.
  - Confirm current status before large edits.
-->

# 1. Problem / Opportunity
iOS developers cannot collect comprehensive crash data from production applications. When apps crash in the field, developers receive limited information through Apple's crash reporting, making it difficult to diagnose and fix issues. This gap prevents developers from understanding crash context, analyzing memory state, and correlating crashes across platforms.

# 2. Goals & Success Indicators
Use measurable targets. Tag for roadmap hints ([MVP], [Future]).

| GoalID | Description | Metric | Target | Tag |
|--------|-------------|--------|--------|-----|
| G1 | Capture iOS application crashes | Crash capture rate | ≥95% | [MVP] |
| G2 | Generate industry-standard crash reports | Valid minidump files | 100% | [MVP] |
| G3 | Maintain consistency with other platform implementations | API/behavior differences | Minimal | [MVP] |
| G4 | Operate within iOS constraints | App Store rejections | 0 | [MVP] |
| G5 | Minimize app impact | Performance degradation | <1% | [MVP] |

---

# 3. IDs & Traceability  {#id-scheme}
Stable tokens link requirements, tasks, and design decisions.

| Prefix | Entity | Defined In | Pattern | Notes |
|--------|--------|------------|---------|-------|
| **G**  | Goal | PRD.md | `G[0-9]+` | High-level outcome; can span many R-IDs. |
| **R-** | Requirement | PRD.md | `R-[0-9]+` | Permanent; never reuse. WHAT/WHY. |
| **T-** | Task | TASKS.md | `T-[0-9]+` | Execution unit; may map 0..n R-. |
| **D-** | Decision | DECISIONS.md | `D-YYYY-MM-DD-##` | Append-only; can supersede. |
| **Q-** | Question | §Open Questions | `Q-[0-9]+` | Resolve → may spawn R-/T-. |

**Linking rules**
- Requirements may list tracker refs in a `Links` column (GitHub, Jira, URL…).
- TASKS.md has *optional* `ReqIDs` column (comma-separated R-IDs).
- DECISIONS.md records `Relates-To: R-...` when constraining a requirement.
- IDs uppercase; never recycled; deprecate instead.

---

# 4. Target Users / Personas
- **iOS App Developer**: Needs detailed crash information to fix production issues
- **SDK Developer**: Embeds crash reporting in third-party libraries

# 5. Use Cases / Scenarios
1. **Crash Data Collection**: App crashes; crash-context captures info; minidump-writer generates crash report file
2. **Local Storage**: Generated minidump saved to app's sandbox for later retrieval
3. **SDK Integration**: Third-party library embeds minidump-writer for crash collection

# 6. Functional Requirements (User-Story Style)
Each row = user need; implementation lives in Tasks & code.

| ReqID | As a… | I need… | So that… | Links | Notes |
|-------|-------|---------|----------|-------|-------|
| R-1 | iOS developer | Crash data when app terminates unexpectedly | I can fix the root cause | G1 | Requires crash-context |
| R-2 | iOS developer | Thread state at time of crash | I can trace execution flow | G2 |  |
| R-3 | iOS developer | Memory context around crash | I can analyze data corruption | G2 |  |
| R-4 | iOS developer | Device and OS information | I can reproduce issues | G2 |  |
| R-5 | iOS developer | Loaded modules and versions | I can identify problematic libraries | G2 |  |
| R-6 | iOS developer | Easy SDK integration | I can add crash reporting quickly | G3 | Swift/ObjC support |
| R-7 | iOS developer | Crash reports saved locally | Data persists through app restart | G1 |  |
| R-8 | iOS developer | App Store compliant solution | My app passes review | G4 |  |
| R-9 | iOS developer | Configurable data collection | I control what's captured | G1 | Privacy compliance |
| R-10 | iOS developer | Low performance overhead | User experience isn't affected | G5 |  |

# 7. Non-Functional Requirements
- **Reliability**: Must capture ≥95% of crashes
- **Performance**: <1% CPU/memory overhead during normal operation
- **Storage**: Crash reports ≤10MB
- **Compatibility**: iOS 12.0+ support required
- **Compliance**: Must not use Apple's private APIs
- **Dependencies**: Requires crash-context crate for crash information

# 8. Scope / Out of Scope
Clarify boundaries to prevent drift.

**In Scope:**
- iOS crash data processing from crash-context
- Minidump format generation
- Local crash report storage
- iOS 12.0+ support
- Self-process dumping only

**Out of Scope:**
- Crash report transmission/upload
- Symbolication services
- Crash analysis/grouping
- Signal handler implementation
- Crash detection mechanisms
- External process dumping
- Other apps' crash collection
- watchOS/tvOS platforms

# 9. Open Questions / Assumptions
Track unknowns that may spawn R-/T-/D-.

| QID | Topic | Owner | Due | Status | Notes |
|-----|-------|-------|-----|--------|-------|
| Q-1 | iOS sandbox storage location? | Engineering | 2025-07-20 | Open | Where to save minidumps within app sandbox |
| Q-2 | iOS Simulator support level? | Engineering | 2025-07-22 | Open | What works/doesn't work on simulator |
| Q-3 | Mac Catalyst support? | Product | 2025-07-25 | Open | Unified macOS/iOS apps |
| Q-4 | crash-context iOS compatibility? | Engineering | 2025-07-23 | Open | Verify iOS support in crash-context |

**Assumptions:**
- iOS 12.0+ provides necessary APIs for implementation
- App sandbox allows minidump file creation and storage
- Simulator behavior differs from physical devices
- crash-context crate will provide complete crash information on iOS

# 10. Links
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Technical design details
- [TASKS.md](./TASKS.md) - Implementation task tracking
- [Google Breakpad Minidump Format](https://chromium.googlesource.com/breakpad/breakpad/+/master/docs/minidump_format.md)
- [crash-context crate](https://crates.io/crates/crash-context) - Crash information provider

# 11. Revision History
| Version | Date | Author | Notes |
|---------|------|--------|-------|
| v2025-07-17 | 2025-07-17 | bahamoth | Initial iOS support PRD with crash-context integration |