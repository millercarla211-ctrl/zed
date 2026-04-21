---
inclusion: auto
---

# Git Branch Workflow

## Branch Structure

### main
- **Purpose**: Always stays in sync with Zed upstream (zed-industries/zed)
- **Tracks**: `upstream/main`
- **Never**: Contains custom features or modifications
- **Updates**: Pull from upstream regularly to stay current

### forge
- **Purpose**: Our local development branch with custom features
- **Contains**: All custom modifications, features, and experiments
- **Base**: Regularly merges from `main` to stay current with upstream

### Deprecated Branches
- `deprecated-1` (was: dev)
- `deprecated-2` (was: forge - old version)
- `deprecated-3` (was: windows-webpreview)
- `deprecated-4` (was: main - old version before workflow change)

## Workflow

### 1. Keeping main in sync with upstream

```bash
git checkout main
git pull upstream main
git push origin main
```

**Frequency**: Daily or before starting new work

### 2. Merging upstream changes into forge

```bash
git checkout forge
git merge main --no-ff -m "Merge main (upstream) into forge: sync with Zed upstream"
```

**When conflicts occur**:
- **ALWAYS keep BOTH changes** (local from forge + remote from main)
- Merge intelligently:
  - Combine all imports
  - Keep all struct fields from both branches
  - Keep all methods from both branches
  - Keep all enum variants from both branches
- Document EVERY change in `changelog.txt` at the root:
  - File path
  - What was in local (HEAD/forge)
  - What was in remote (main)
  - How you merged them

### 3. After resolving conflicts

```bash
# Stage all resolved files
git add <file1> <file2> ...

# Commit the merge
git commit -m "Merge main (upstream) into forge: resolved all conflicts keeping both changes

- List of files resolved
- Summary of what was kept from each branch
- Reference to changelog.txt for details"
```

## Merge Conflict Resolution Strategy

### Golden Rule: Keep BOTH Changes

Never discard features from either branch. Always merge them together.

### Examples

#### Imports
```rust
// LOCAL (forge)
use crate::{NewLiquidGlass, NewWebPreview, ...};

// REMOTE (main)
use crate::{NewTerminal, ...};

// MERGED (keep both)
use crate::{NewLiquidGlass, NewWebPreview, NewTerminal, ...};
```

#### Struct Fields
```rust
// LOCAL (forge)
struct Sidebar {
    space_labels: HashMap<WorkspaceId, SharedString>,
    carousel_drag_start: Option<(f32, usize)>,
}

// REMOTE (main)
struct Sidebar {
    history_visible: bool,
}

// MERGED (keep both)
struct Sidebar {
    space_labels: HashMap<WorkspaceId, SharedString>,
    carousel_drag_start: Option<(f32, usize)>,
    history_visible: bool,
}
```

#### Methods
```rust
// LOCAL (forge)
fn render_space_carousel(&self) -> impl IntoElement { ... }

// REMOTE (main)
fn toggle_history(&mut self) { ... }

// MERGED (keep both methods)
fn render_space_carousel(&self) -> impl IntoElement { ... }
fn toggle_history(&mut self) { ... }
```

## Documentation Requirements

### changelog.txt Format

```markdown
[YYYY-MM-DD] - Merge Conflict Resolution: main into forge

================================================================================
FILE: path/to/file.rs
================================================================================

CONFLICT N (Lines X-Y): Description
-------------------------------------------
LOCAL (HEAD/forge):
<code or description>

REMOTE (main):
<code or description>

RESOLUTION:
<detailed explanation of how both were merged>
```

## Build and Test

### CRITICAL: Only Use `just run`

```bash
# CORRECT
just run

# WRONG - DO NOT USE
cargo test    # Takes too long, will timeout
cargo check   # Not useful for this project size
cargo build   # Use just run instead
cargo clippy  # Too slow
```

### When to Run

- **RARELY** - Only after implementing ALL changes
- **MOST OF THE TIME: DON'T RUN IT** - User will run manually
- Running takes significant time - use that time to fix more bugs instead

### Verification Strategy

1. **READ and ANALYZE code** - Use full context to understand codebase
2. **FIND bugs by inspection** - Don't rely on tests/checks
3. **IMPLEMENT fixes** - Make all necessary changes
4. **VERIFY by code review** - Read your changes, ensure correctness
5. **Let user test** - User will run `just run` when ready

## Remote Repositories

### upstream
- **URL**: https://github.com/zed-industries/zed
- **Purpose**: Official Zed repository
- **Branch**: main

### origin
- **URL**: https://github.com/millercarla211-ctrl/zed
- **Purpose**: Our fork
- **Branches**: main, forge, deprecated-*

## Common Commands

### Check current branch and status
```bash
git status
git log --oneline -5
```

### View branches
```bash
git branch -a
```

### View remotes
```bash
git remote -v
```

### Sync workflow (complete)
```bash
# 1. Update main from upstream
git checkout main
git pull upstream main
git push origin main

# 2. Merge main into forge
git checkout forge
git merge main --no-ff

# 3. Resolve conflicts (keep both changes)
# ... resolve each file ...
git add <resolved-files>

# 4. Update changelog.txt with all resolutions
# ... document changes ...
git add changelog.txt

# 5. Commit merge
git commit -m "Merge main into forge: ..."

# 6. Push forge
git push origin forge
```

## Important Notes

1. **Never force push** to main or forge without explicit user permission
2. **Always document** merge resolutions in changelog.txt
3. **Keep both changes** - never discard features from either branch
4. **Test by inspection** - don't rely on slow build/test cycles
5. **main tracks upstream** - it should always be a clean mirror of zed-industries/zed
6. **forge is our dev branch** - all custom work happens here

## GitHub Account

- **Current User**: millercarla211-ctrl
- **Previous User**: manfromexistence (logged out)

## Project Context

This is a fork of the Zed editor with custom features:
- Sidebar space carousel with drag functionality
- Web preview integration (NewWebPreview, NewLiquidGlass)
- Screen dock integration
- Liquid glass visual effects
- Custom workspace management

All custom features are maintained in the `forge` branch while `main` stays synchronized with upstream Zed.
