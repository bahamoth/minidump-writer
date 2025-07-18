# CLAUDE.md

Claude Code guidance for minidump-writer.

## 🎯 Task Execution Workflow

### 1. Starting a Task
- **CHECK**: Read [TASKS.md](./TASKS.md) for available work
- **ACTION**: Claim task with your agent identifier (e.g., dev-zatanna)
- **CREATE**: Personal checklist file:
  ```bash
  mkdir -p .agent-checklists
  touch .agent-checklists/{your-role}-{your-name}-{task-id}.md
  ```
- **USE**: TodoWrite for session-based tracking

### 2. Before Implementation
- **CHECK**: Read [ARCHITECTURE.md](./ARCHITECTURE.md) for system design patterns
- **CHECK**: Read [PRD.md](./PRD.md) if implementing new iOS features
- **CHECK**: Read relevant section in [DECISIONS.md](./DECISIONS.md) for technical choices
- **ACTION**: Plan your approach based on existing patterns

### 3. During Implementation
- **FOLLOW**: Existing code patterns in the module
- **REFERENCE**: Platform-specific examples in src/{platform}/
- **UPDATE**: Your checklist file as you progress

### 4. Before Commit ⚠️
- **CHECK**: Read [CONVENTIONS.md#linting-required](./CONVENTIONS.md#linting-required)
- **CHECK**: Read [CONVENTIONS.md#testing-requirements](./CONVENTIONS.md#testing-requirements)
- **EXECUTE**: All required checks:
  ```bash
  cargo fmt
  cargo clippy  # Fix ALL warnings
  cargo test
  ```
- **ONLY THEN**: Create your commit

### 5. Before Push ⚠️
- **RE-READ**: [CONVENTIONS.md lines 35-38](./CONVENTIONS.md#linting-required)
- **EXECUTE**: Final validation:
  ```bash
  cargo clippy  # Must pass without warnings
  ```

### 6. Before Creating PR
- **CHECK**: [CONVENTIONS.md#commit-conventions](./CONVENTIONS.md#commit-conventions)
- **ENSURE**: Commit message follows the format
- **UPDATE**: TASKS.md with completion status

## 📋 Standard Task Checklist Template

Create this in `.agent-checklists/{your-role}-{your-name}-{task-id}.md`:

```markdown
# Task: {task-id} - {task-description}
Agent: {your-role}-{your-name}
Date: {YYYY-MM-DD}

## Pre-Implementation
- [ ] Read ARCHITECTURE.md relevant sections
- [ ] Read PRD.md for requirements (if applicable)
- [ ] Review existing similar implementations
- [ ] Plan approach

## Implementation
- [ ] Write core functionality
- [ ] Add unit tests
- [ ] Update documentation

## Pre-Commit Validation
- [ ] Run `cargo fmt`
- [ ] Run `cargo clippy` - fix ALL warnings
- [ ] Run `cargo test` - ensure all pass
- [ ] Self-review changes

## Pre-Push Validation
- [ ] Run `cargo clippy` final check
- [ ] Verify no new warnings introduced

## Finalization
- [ ] Update TASKS.md status
- [ ] Create PR with proper description
- [ ] Link PR to task in TASKS.md
```

## 🚨 Critical Rules

1. **At each checkpoint, you MUST read the referenced section**
   - Do NOT rely on memory or previous readings
   - Context can be lost in long sessions

2. **Never skip validation steps**
   - Even if you "think" the code is fine
   - Automation prevents human errors

3. **Checklist files are part of your commit**
   - Shows your work process
   - Helps other agents understand progress

## 📚 Documentation Quick Reference

| Document | When to Read | Purpose |
|----------|--------------|---------|
| [OVERVIEW.md](./OVERVIEW.md) | First time on project | Understand project goals |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Before implementing | System design patterns |
| [CONVENTIONS.md](./CONVENTIONS.md) | Before commit/push | Coding standards & rules |
| [DECISIONS.md](./DECISIONS.md) | When making choices | Understand past decisions |
| [PRD.md](./PRD.md) | iOS features only | iOS-specific requirements |
| [TASKS.md](./TASKS.md) | Start/end of work | Task management |

## 🤝 Multi-Agent Coordination

### Checklist Management
- **Persistent**: `.agent-checklists/` directory (commit with your code)
- **Session-only**: TodoWrite tool (not shared between agents)
- **Shared state**: TASKS.md (authoritative task status)

### Avoiding Conflicts
1. Always check TASKS.md before claiming work
2. Update your task status immediately when starting
3. Commit your checklist file to show progress
4. Communicate through PR comments

### Agent Identification
- Choose a unique name when starting
- Use format: `{role}-{name}` where:
  - `role`: Your function (dev, reviewer, tester, architect)
  - `name`: Your chosen unique identifier
  - Examples: `dev-zatanna`, `reviewer-stark`, `tester-parker`
- Consistently use this identifier in all interactions