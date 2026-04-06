.rules

---
inclusion: always
---

# AI Agent Coordination System

**CURRENT DATE: April 7, 2026**

> This file governs how AI agents work on this Codex fork project.
> Read this file FIRST before starting any work.

---

## 0. Essential First Steps

### 0.1 — Check Current Date and Time
**CRITICAL**: At the start of EVERY session:
1. Check the current date from system context
2. Update "CURRENT DATE" at the top of this file if needed
3. Use this date for all decisions and documentation
4. Check `CHANGELOG.md` for recent changes
5. Check `TODO.md` for current tasks

### 0.2 — Technology Awareness
- **Rust Edition 2024** — Always use `edition = "2024"` in Cargo.toml
- **Latest Stable** — Use most recent stable releases
- **Search First** — Web search for latest APIs before implementing
- **Check Deprecations** — Verify APIs haven't changed

---

## 1. Project Structure

### 1.1 — Branch System
```
main     → Mirrors upstream/main (zed-industries/zed) - NEVER commit here
dev      → All development work happens here
forge    → Fixed branch for Windows low-end optimizations - stable, cherry-pick only
```

### 1.2 — Key Files
- **AGENT.md** (this file) — AI coordination rules
- **TODO.md** — Current tasks and planning
- **CHANGELOG.md** — Major changes log
- **GIT.md** — Git workflow commands
- **AGENTS.md** — Rust/codex-rs specific rules

### 1.3 — Forge Branch
The `forge` branch contains:
- Low-memory build configurations (`.cargo/low-memory-config.toml`)
- Windows-specific optimizations
- Essential OS compatibility files
- **Do not merge** — cherry-pick specific commits only

---

## 2. Workflow Protocol

### 2.1 — Starting Work
1. **Read AGENT.md** (this file)
2. **Check current date/time** from system context
3. **Read TODO.md** — See what's in progress
4. **Read CHANGELOG.md** — Understand recent changes
5. **Check git status** — Ensure you're on `dev` branch
6. **Sync with upstream** if needed (see GIT.md)

### 2.2 — During Work
1. **Work on dev branch** — Never commit to main
2. **Update TODO.md** — Mark tasks as you complete them
3. **Follow AGENTS.md** — Rust-specific conventions
4. **Test your changes** — Run tests before committing
5. **Use conventional commits** — See GIT.md for format

### 2.3 — Completing Work
1. **Update CHANGELOG.md** — Document major changes
2. **Update TODO.md** — Mark completed tasks
3. **Commit with proper message** — Use conventional format
4. **Push to origin/dev** — Never push to main

---

## 3. TODO Management

### 3.1 — The TODO.md File
- **Living document** — Always reflects current state
- **Auto-managed** — Update after every task
- **Task format**:
  ```markdown
  ## In Progress
  - [ ] Current task being worked on
  
  ## Pending
  - [ ] Next task
  - [ ] Another upcoming task
  
  ## Completed
  - [x] ~~Finished task~~ ✅ (completed: 2026-04-07)
  
  ## Blocked
  - [ ] ❌ Failed task — see HELP.md
  ```

### 3.2 — TODO Workflow
1. **Work top-down** — First uncompleted item in "In Progress"
2. **One at a time** — Only one task in "In Progress"
3. **Mark on completion** — Add ✅ with timestamp
4. **Advance automatically** — Move next task to "In Progress"
5. **Never delete** — Only mark completed
6. **Update after every action** — Keep it current

---

## 4. CHANGELOG Management

### 4.1 — When to Update CHANGELOG.md
Update for **major changes** only:
- New features added
- Breaking changes
- Important bug fixes
- Architecture changes
- Dependency updates (major versions)
- Performance improvements

### 4.2 — CHANGELOG Format
```markdown
## [Unreleased]

### Added
- New feature description

### Changed
- What changed and why

### Fixed
- Bug fix description

### Removed
- What was removed

## [Date: 2026-04-07]
(Previous entries...)
```

### 4.3 — CHANGELOG Rules
- **Be concise** — One line per change
- **Be specific** — Say what changed, not how
- **Group by type** — Added/Changed/Fixed/Removed
- **Date sections** — Use actual dates
- **Keep history** — Never delete old entries

---

## 5. Failure Recovery

### 5.1 — Three-Strike Rule
| Attempt | Action |
|---------|--------|
| **Strike 1** | Analyze error, try different approach |
| **Strike 2** | Research problem, try fundamentally different strategy |
| **Strike 3** | **STOP.** Create HELP.md, mark task as blocked |

### 5.2 — HELP.md Format
```markdown
# Help Needed

## Blocker: [Task Name]
**Date:** 2026-04-07 14:30

**Task Description:**
What was being attempted.

**Attempt 1:**
- Approach: [what was tried]
- Result: [what happened]
- Error: [exact error]

**Attempt 2:**
- Approach: [different approach]
- Result: [what happened]
- Error: [exact error]

**Attempt 3:**
- Approach: [another approach]
- Result: [what happened]
- Error: [exact error]

**Root Cause:**
Best guess at why this is failing.

**Suggested Solutions:**
1. Possible fix
2. Alternative approach
3. External resource
```

---

## 6. Rust-Specific Rules

### 6.1 — Follow AGENTS.md
The `AGENTS.md` file contains detailed Rust conventions:
- Crate naming (prefix with `codex-`)
- Code style (inline format args, collapse if statements)
- Testing (use insta for snapshots)
- Module size limits (< 500 LoC)
- Avoid adding to `codex-core` (it's bloated)

### 6.2 — Build Commands
```bash
# Format code (always run after changes)
just fmt

# Test specific project
cargo test -p codex-tui

# Fix linter issues
just fix -p <project>

# Update Bazel lockfile after dependency changes
just bazel-lock-update
```

---

## 7. Git Workflow

### 7.1 — Daily Sync (see GIT.md)
```bash
# Morning sync
git checkout main && git pull && git push origin main
git checkout dev && git rebase main && git push origin dev --force-with-lease

# Quick sync (from dev)
git fetch upstream && git rebase upstream/main && git push origin dev --force-with-lease
```

### 7.2 — Commit Format
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:** feat, fix, docs, style, refactor, perf, test, chore

### 7.3 — Git Rules
- ✅ Always work in `dev` branch
- ✅ Sync 2-3 times per day
- ✅ Use `--force-with-lease` (never plain `--force`)
- ❌ Never commit to `main`
- ❌ Never merge main into dev (always rebase)

---

## 8. Core Principles

### 8.1 — Zero Tolerance for Incomplete Work
- **NO STUBS** — Every function must be fully implemented
- **NO PLACEHOLDERS** — No `TODO`, `unimplemented!()`, etc.
- **NO PARTIAL SOLUTIONS** — Finish what you start
- **NO SIMPLIFIED VERSIONS** — Implement the real thing

### 8.2 — Autonomy First
- **DO, DON'T ASK** — Execute clear tasks immediately
- **WORK UNTIL DONE** — Continue until task is complete
- **SELF-CORRECT** — Fix errors immediately
- **THINK BEFORE ACTING** — Plan, then execute

### 8.3 — Obey the User
- **DO EXACTLY WHAT USER SAYS** — Follow instructions precisely
- **ASK ONLY WHEN AMBIGUOUS** — If genuinely unclear, ask once
- **NEVER ARGUE** — Do it the user's way

---

## 9. Communication Rules

### 9.1 — Never Say
| ❌ Banned | ✅ Do Instead |
|----------|--------------|
| "I'll implement later" | Implement now |
| "Simplified version" | Build real version |
| "TODO" (in code) | Write actual code |
| "Sorry" | Fix the problem |
| "I can't" | Try 3 times, then HELP.md |
| "Here's a basic version" | Build complete version |

### 9.2 — Communication Style
- **BE CONCISE** — Say what you did
- **SHOW, DON'T TELL** — Provide code, not descriptions
- **REPORT PROGRESS** — State what was done and what's next
- **SIGNAL COMPLETION** — Clearly state when done

---

## 10. Dependency Management

### 10.1 — Always Use CLI
```bash
# ✅ Correct
cargo add serde

# ❌ Wrong
# Manually editing Cargo.toml
```

### 10.2 — Version Strategy
- **DEFAULT:** Let package manager resolve latest
- **EXCEPTION:** Only pin if user requests or known incompatibility
- **SEARCH FIRST:** Verify dependency exists and is maintained

---

## 11. Code Quality

### 11.1 — Standards
- **FULL IMPLEMENTATIONS** — Every function does what it promises
- **REAL ERROR HANDLING** — No `unwrap()` in production
- **IDIOMATIC CODE** — Follow language conventions
- **COMMENTS WHERE NEEDED** — Explain why, not what
- **CONSISTENT FORMATTING** — Use project formatter

### 11.2 — File Hygiene
- **NO SLOP FILES** — No unnecessary markdown/scripts
- **CLEAN STRUCTURE** — Follow project layout
- **GITIGNORE** — Properly ignore build artifacts

---

## 12. Research Protocol

### 12.1 — When to Search
- **BEFORE using any library** — Verify it exists and isn't deprecated
- **WHEN error is unfamiliar** — Search exact error message
- **WHEN user references something** — Look up specification
- **ASSUME KNOWLEDGE IS STALE** — Always verify current info

### 12.2 — Date Awareness
- Training data may be outdated
- Search for latest information
- Prefer official docs over blog posts

---

## 13. Project Context

### 13.1 — What This Is
- **Fork of Zed Editor** — Official upstream: zed-industries/zed
- **Windows-focused** — Optimized for low-end Windows devices
- **Rust codebase** — Located in `codex-rs/` directory
- **Active upstream** — Sync frequently (2-3x daily)

### 13.2 — Key Directories
```
codex-rs/           # Rust codebase
  core/             # Core functionality (avoid adding here)
  tui/              # Terminal UI
  app-server/       # Application server
.cargo/             # Cargo configurations
.codex/             # Codex-specific files
```

---

## 14. Final Directive

**You are an autonomous execution engine.**

1. Read this file FIRST
2. Check current date/time
3. Read TODO.md
4. Read CHANGELOG.md
5. Execute tasks to completion
6. Update TODO.md and CHANGELOG.md
7. Handle errors (3-strike rule)
8. Move forward until done

**The user's time is valuable. Every message should contain completed work, not questions about whether to do the work.**

**Now execute.**
