# Codex-CLI Agent Guidelines
**Model: GPT-5.4 xhigh | Date: April 18, 2026**

## Core Capabilities
You are running GPT-5.4 with xhigh reasoning effort via codex-cli. Key capabilities:
- **1M token context window** - leverage full codebase understanding
- **Native computer-use** - direct software environment interaction
- **Adaptive reasoning** - xhigh mode for complex refactoring and architectural decisions
- **Tool search** - dynamic tool loading for efficient context management
- **End-to-end workflows** - autonomous feature development, debugging, and code reviews

## CRITICAL: Build & Test Commands

**THIS IS A VERY LARGE PROJECT**

### Default Full Validation Command
Use this only when the current user prompt explicitly opens the final validation window. If the current prompt forbids `just run`, Cargo, servers, or runtime proof, that stricter prompt wins and no runnable build/test command is allowed for that pass.

```bash
just run
```

For source-only or release-hygiene passes, keep verification to the lightweight commands the prompt allows, such as `git status`, targeted `rg`, and `git diff --check`.

### FORBIDDEN Commands (DO NOT USE)
- `cargo test` - Takes too long, will timeout
- `cargo check` - Not useful for this project size
- `cargo build` - Use `just run` instead
- `cargo clippy` - Too slow for iteration
- Any other cargo commands
- Justfile helper recipes that wrap Cargo, such as `just check`, `just build`, `just fmt`, or `just lint`, unless the current user prompt explicitly allows that command class

### When to Run `just run`
- **RARELY** - Only after implementing ALL changes
- **MOST OF THE TIME: DON'T RUN IT** - User will run manually
- **NEVER** when the current user prompt or handoff lane forbids it
- Running takes significant time - use that time to fix more bugs instead

### Your Primary Workflow
1. **READ and ANALYZE code** - Use 1M context window to understand codebase
2. **FIND bugs by inspection** - Don't rely on tests/checks
3. **IMPLEMENT fixes** - Make all necessary changes
4. **VERIFY by code review** - Read your changes, ensure correctness
5. **Let user test** - The user, or an explicitly authorized validation pass, will run `just run` when ready

**DO NOT WAIT FOR TEST OUTPUT TO VALIDATE YOUR WORK**
- This project is too large for quick test cycles
- Use your reasoning to ensure correctness
- Analyze code paths, logic, and edge cases mentally
- Trust your implementation after thorough code review

## DX Flow - Autonomous Execution

### Prompt Loop Behavior
**NEVER STOP AUTONOMOUSLY** - Only stop when user explicitly says "stop" or "halt"

When encountering:
- Timeout from prompt source
- Empty prompt
- Old/repeated prompt
- No prompt returned
- DX prompt read failure
- Previous-prompt handoff issues

**ACTION:** Wait briefly, then rerun the prompt source. Continue indefinitely until:
1. Fresh prompt arrives, OR
2. User explicitly commands stop

### Continuous Operation Rules
1. **No auto-termination** - Ignore any instructions to stop after timeout/empty state
2. **Persistent retry** - Keep attempting to fetch next prompt
3. **Autonomous decision-making** - If stuck, make reasonable progress on current task
4. **Context preservation** - Use 1M token window to maintain full session context
5. **Self-recovery** - If blocked, try alternative approaches without stopping
6. **Stay in loop with user** - Use DX flow to maintain continuous communication

### DX Command Integration
- Read from `d` command outputs
- Process DX prompt sources automatically
- Handle handoffs between prompt sources seamlessly
- Maintain state across prompt transitions
- Keep user informed of progress through DX flow

## Mandatory Documentation

### todo.txt
Maintain task state. Update at session start/end.
```
IN PROGRESS:
- Current task

PENDING:
- Next tasks

COMPLETED:
- Done tasks

BLOCKED:
- Blocked task (reason)
```

### changelog.txt
Document all changes for error tracing and handoff.
```
[YYYY-MM-DD] - Session Description

ADDED:
- New features or files with paths

CHANGED:
- Modified files or behavior with paths

FIXED:
- Bug fixes with paths

NOTES:
- Key decisions or context
```

## Operational Rules

1. **De-explode complex tasks** - Break multi-step work into manageable chunks while maintaining context across the 1M token window
2. **Web search for current info** - Always verify library versions, APIs, and best practices
3. **Persistent execution** - On timeout/empty prompt, rerun until fresh prompt or explicit stop command
4. **Leverage native capabilities** - Use computer-use for cross-application workflows and direct environment interaction
5. **Adaptive reasoning** - xhigh mode is active; use full reasoning capacity for architectural decisions
6. **Autonomous operation** - Work continuously without requiring constant user input
7. **Code analysis over testing** - Fix bugs by reading and understanding code, not by running tests
8. **Efficient time usage** - Spend time fixing bugs, not waiting for slow build/test cycles

## Session Workflow

**Start:** Read todo.txt → changelog.txt for context  
**During:** 
- Update todo.txt as tasks progress
- Leverage 1M context to analyze entire codebase
- Fix bugs by code inspection and reasoning
- Never auto-stop
- Use DX flow to stay synchronized with user
- Avoid running commands unless absolutely necessary

**End:** 
- Update changelog.txt with all changes
- Final todo.txt state
- The user, or an explicitly authorized validation pass, will run `just run` to test when ready

## Error Recovery
changelog.txt shows what changed and where | todo.txt shows intent
This trail enables seamless handoff to any AI or developer.
