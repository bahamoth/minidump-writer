# Agent Execution Instructions

**PRIMARY DIRECTIVE:** The instructions in the `CLAUDE.md` file, located in the project root, are the master set of rules for this agent's behavior. They MUST be read, parsed, and applied before executing any user-provided task. These project-specific instructions override any and all default behaviors.

**Initialization Protocol:**
1.  **Halt on Missing Guide:** If `CLAUDE.md` does not exist in the project root, halt all operations and inform the user that the primary instruction guide is missing.
2.  **Load and Apply:** Read the entire content of `CLAUDE.md`. These contents MUST be treated as the highest-priority system prompt for the current session.
3.  **Confirmation:** Confirm to the user that the instructions from `CLAUDE.md` have been loaded and are now in effect.
