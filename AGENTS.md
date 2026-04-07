.rules

---
inclusion: always
---

# AI Agent Coordination System

**CURRENT DATE: February 7, 2026**
**AI MODEL: GPT-5.4 (Released March 5, 2026)**

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
- **GPT-5.4 Model** — Using OpenAI's latest model with 1M token context window
- **Rust Edition 2024** — Always use `edition = "2024"` in Cargo.toml
- **Latest Stable** — Use most recent stable releases
- **Search First** — Web search for latest APIs before implementing
- **Check Deprecations** — Verify APIs haven't changed

### 0.3 — GPT-5.4 Capabilities (March 2026)
- **1M Token Context** — Can handle entire large codebases in context
- **Computer Use** — Native screenshot reading and UI automation (75% success rate on OSWorld)
- **Advanced Reasoning** — Configurable reasoning effort for complex tasks
- **Tool Search** — Improved tool discovery and orchestration
- **Agentic Workflows** — Better at planning, executing, and verifying multi-step tasks

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

**IMPORTANT: Use `just run` for building and running the project.**

The project uses `justfile` for build automation. DO NOT use manual `cargo build` commands.

```bash
# Run the project (builds automatically)
just run

# Format code (always run after changes)
just fmt

# Test specific project
cargo test -p web_preview

# Fix linter issues
just fix -p <project>

# Update Bazel lockfile after dependency changes (if using Bazel)
just bazel-lock-update
```

**Build Command Rules:**
- ✅ Use `just run` to build and run
- ✅ Use `just fmt` to format code
- ✅ Use `cargo test` for testing
- ❌ DO NOT use `cargo build` directly
- ❌ DO NOT use `cargo run` directly
- ❌ DO NOT use custom build scripts

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
crates/             # All Rust crates
  web_preview/      # NEW: Embedded web browser (wry-based)
  workspace/        # Workspace and pane management
  zed/              # Main application entry
.cargo/             # Cargo configurations
assets/             # Icons, fonts, themes
```

### 13.3 — Web Preview Feature (Added February 2026)

**Location:** `crates/web_preview/`

The web preview feature provides an embedded web browser inside the editor using the `wry` library (WebView2 on Windows).

**Key Files:**
- `crates/web_preview/src/web_preview_view.rs` — Main implementation (2,440 lines)
- `crates/web_preview/src/web_preview.rs` — Module initialization
- `crates/web_preview/Cargo.toml` — Dependencies (wry 0.53)
- `WEB_PREVIEW_IMPLEMENTATION.md` — Complete documentation

**Features Implemented:**
- ✅ Full web browser with navigation (back/forward/reload)
- ✅ URL input bar with bookmark system
- ✅ Browser extensions support (Chrome/Firefox auto-detection)
- ✅ Developer tools integration
- ✅ Screenshot capture (full page and area selection)
- ✅ Zoom controls (10% increments)
- ✅ Session isolation per workspace
- ✅ IPC communication with agent panel
- ✅ Custom tab bar controls (URL bar, navigation buttons)
- ✅ Transparent overlay for input blocking when URL editor focused

**Architecture:**
- Webview stays ALWAYS VISIBLE (no hiding logic)
- Input blocking via transparent overlay when URL editor has focus
- Native webview (WebView2 on Windows)
- Isolated browser profiles per workspace ID
- Profile storage: `~/.local/share/zed/web_preview_profiles/{workspace_id}/`

**Windows-Specific:**
- Disables DirectComposition to support child webviews
- Environment variable: `GPUI_DISABLE_DIRECT_COMPOSITION=1`
- Set in `crates/zed/src/main.rs`

**Integration Points:**
1. `Cargo.toml` — Added web_preview to members and dependencies
2. `crates/zed/Cargo.toml` — Added web_preview dependency
3. `crates/zed/src/main.rs` — Windows fix + init call
4. `crates/workspace/src/workspace.rs` — NewWebPreview action
5. `crates/workspace/src/pane.rs` — Context menu integration
6. `crates/workspace/src/item.rs` — PaneTabBarControls trait

**Usage:**
- Open via context menu: Tab bar `+` button → "New Web Preview"
- Navigate using toolbar: Back/Forward/Reload buttons + URL input
- Bookmark pages with star icon
- Access DevTools, screenshots, and more via toolbar buttons

**Important Notes:**
- DO NOT add webview hiding logic — it stays visible always
- Input control is via transparent overlay, not visibility
- Tab navigation arrows remain for tab switching
- Web navigation arrows are in the preview's own toolbar

**Hole-Punching Architecture (CRITICAL - MUST IMPLEMENT NOW):**

The web preview MUST use a "hole-punching underlay" approach for proper UI layering on ALL platforms.

**The Problem:**
Standard webviews float ON TOP of the app, blocking UI elements like Command Palette, tooltips, and menus.

**The Solution - Underlay Architecture:**
1. **Place webview BEHIND the GPUI window** (not on top)
2. **Make GPUI window background transparent** where webview should show
3. **GPUI UI always renders on top** - Command Palette, menus, tooltips work perfectly
4. **Webview shows through transparent areas** of GPUI window

**How It Works:**
- Webpage content shows through transparent GPUI background
- When you open Command Palette, GPUI draws it normally on top
- GPUI is the top layer, so editor UI has priority over webview
- Webview's internal dropdowns cannot float over GPUI elements (acceptable limitation)

**Implementation Requirements (ALL PLATFORMS - NOT OPTIONAL):**

1. **Windows Platform** (`crates/gpui/src/platform/windows/`) - PRIORITY:
   - Create webview window as PARENT window (not child)
   - Create GPUI window as CHILD of webview window with `WS_EX_TRANSPARENT` extended style
   - Set GPUI window background to transparent where webview should show
   - Use `SetLayeredWindowAttributes` or DWM composition for transparency
   - **STATUS: NOT IMPLEMENTED - MUST DO NOW**

2. **macOS Platform** (`crates/gpui/src/platform/mac/`) - CRITICAL:
   - Create webview as parent NSWindow
   - Create GPUI NSWindow as child with transparent background
   - Use `setOpaque(false)` and `setBackgroundColor(NSColor.clear)`
   - Implement proper window layering with NSWindow level management
   - **STATUS: NOT IMPLEMENTED - MUST DO NOW**
   - **THIS IS NOT "FUTURE WORK" - THIS IS REQUIRED FOR PROFESSIONAL CROSS-PLATFORM SUPPORT**

3. **Linux Platform** (`crates/gpui/src/platform/linux/`) - CRITICAL:
   - Create webview as parent X11/Wayland window
   - Create GPUI window as child with transparent background
   - Use compositor transparency features (X11: ARGB visual, Wayland: wl_surface transparency)
   - Handle both X11 and Wayland properly
   - **STATUS: NOT IMPLEMENTED - MUST DO NOW**
   - **THIS IS NOT "FUTURE WORK" - THIS IS REQUIRED FOR PROFESSIONAL CROSS-PLATFORM SUPPORT**

4. **Web Preview Integration** (`crates/web_preview/`):
   - Coordinate with GPUI to mark webview area as transparent
   - Ensure webview bounds match transparent area
   - Handle window resizing to keep layers synchronized
   - Must work identically on Windows, macOS, and Linux

**Expected Result (ALL PLATFORMS):**
- Webview content visible through GPUI window
- Command Palette, menus, tooltips render perfectly on top
- Zero input lag (OS handles hit-testing naturally)
- Native performance for both webview and editor UI
- Identical behavior on Windows, macOS, and Linux

**CRITICAL NOTES:**
- **ALL THREE PLATFORMS MUST BE IMPLEMENTED** - This is not optional
- macOS and Linux are NOT "future work" - they are core requirements
- Professional software supports all major platforms from day one
- DO NOT implement OS-level hit-testing (WM_NCHITTEST) - that was the wrong approach
- Implement proper underlay architecture as described above
- Test on all three platforms before considering complete

**Implementation Order:**
1. Windows (current platform, implement first)
2. macOS (implement immediately after Windows)
3. Linux (implement immediately after macOS)
4. Test all three platforms thoroughly

**Reference Files:**
- `HOLE_PUNCHING.md` - Full explanation of underlay architecture
- `IMPLEMENT_HOLE_PUNCHING.txt` - Detailed implementation steps (OUTDATED - ignore)

---

## 14. Code Formatting and Linting

### 14.1 — Always Format and Lint After Changes

After making ANY code changes, you MUST run formatting and linting:

```bash
# Format all code
just fmt

# Run linter and auto-fix issues
just fix

# Or fix specific package
just fix -p web_preview
```

### 14.2 — Before Committing

Always run these commands before committing:

```bash
# 1. Format code
just fmt

# 2. Fix linting issues
just fix

# 3. Run tests (if applicable)
cargo test -p <package_name>
```

### 14.3 — Formatting Rules
- **ALWAYS run `just fmt`** after editing Rust files
- **ALWAYS run `just fix`** to auto-fix linter warnings
- **DO NOT commit** unformatted or unlinted code
- **CHECK diagnostics** with getDiagnostics tool after changes

---

## 15. Final Directive

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
