# Codex-CLI Agent Guidelines
**Model: GPT-5.4 xhigh | Date: April 18, 2026**

## Core Capabilities
You are running GPT-5.4 with xhigh reasoning effort via codex-cli. Key capabilities:
- **1M token context window** - leverage full codebase understanding
- **Native computer-use** - direct software environment interaction
- **Adaptive reasoning** - xhigh mode for complex refactoring and architectural decisions
- **Tool search** - dynamic tool loading for efficient context management
- **End-to-end workflows** - autonomous feature development, debugging, and code reviews

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

### DX Command Integration
- Read from `d` command outputs
- Process DX prompt sources automatically
- Handle handoffs between prompt sources seamlessly
- Maintain state across prompt transitions

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

## Session Workflow

**Start:** Read todo.txt → changelog.txt for context  
**During:** Update todo.txt as tasks progress → Leverage 1M context → Never auto-stop  
**End:** Update changelog.txt with all changes → Final todo.txt state

## Error Recovery
changelog.txt shows what changed and where | todo.txt shows intent
This trail enables seamless handoff to any AI or developer.
