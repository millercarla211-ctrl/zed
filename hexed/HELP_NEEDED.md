# Critical Issue Requiring Fix

**Date:** April 13, 2026  
**Project:** Zed Editor Fork (Codex)  
**Platform:** Windows

---

## Issue 1: Screen Dock Border Not Showing on Top and Bottom

### Problem Description
The centered screen dock in the title bar only shows left and right borders. The top and bottom borders are completely invisible, even though border styling is applied.

### Visual Evidence
See screenshot: The "TEST DOCK" text is visible with left and right borders, but no top or bottom borders.

### What Has Been Tried (All Failed)

1. **Padding-based border trick** - Used `.p(px(1.))` + `.bg(border_color)` with inner content
   - Result: Only left/right borders visible

2. **Proper border methods** - Changed to `.border_1()` and `.border_color()`
   - Result: Still only left/right borders visible

3. **Adjusted border radius** - Tried various combinations (5px/4px, 3px/2px, 6px/2px)
   - Result: No change

4. **Removed vertical padding** - Removed `.py(px(4.))` from inner content
   - Result: No change

5. **Increased border thickness** - Changed to `.border_2()` for visibility
   - Result: Still only left/right borders visible

6. **Added vertical margin** - Added `.my_2()` to the dock element
   - Result: No change

7. **Increased dock height** - Changed from 32px to 40px (close to 44px title bar height)
   - Result: Still no top/bottom borders

8. **Added padding to parent container** - Added `.py_1()` to the absolute positioned parent
   - Result: No change

### Current Code Location

**File:** `crates/title_bar/src/title_bar.rs`

**Dock rendering function:** `render_screen_dock()` (line ~426)

**Current minimal test implementation:**
```rust
fn render_screen_dock(...) -> AnyElement {
    let border_color = cx.theme().colors().text_muted;

    div()
        .id("screen-dock-test")
        .flex_none()
        .w(px(200.))
        .h(px(40.))
        .my_2()
        .rounded(px(6.))
        .border_2()
        .border_color(border_color)
        .bg(cx.theme().colors().elevated_surface_background)
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size_full()
                .child("TEST DOCK"),
        )
        .into_any_element()
}
```

**Parent container structure** (line ~322):
```rust
.child(
    div()
        .absolute()
        .left_0()
        .right_0()
        .top_0()
        .bottom_0()
        .flex()
        .items_center()
        .justify_center()
        .child(center_dock),
)
```

**Title bar container** (`crates/platform_title_bar/src/platform_title_bar.rs`, line ~197):
```rust
let title_bar = h_flex()
    .window_control_area(WindowControlArea::Drag)
    .w_full()
    .h(height)  // height = 44px on Windows
    // ... more config ...
    .overflow_x_hidden()  // Only horizontal overflow hidden
```

### Suspected Root Causes

1. **Vertical centering with `.items_center()`** - The parent container uses `.items_center()` which might be clipping the top/bottom borders during vertical alignment

2. **Absolute positioning with full stretch** - The parent uses `.left_0().right_0().top_0().bottom_0()` which stretches it to fill the entire title bar, then tries to center the child

3. **Fixed title bar height** - The title bar has a fixed height of 44px on Windows, and something in the rendering pipeline might be clipping content that extends beyond the vertical center line

4. **GPUI rendering issue** - There might be a bug in GPUI's border rendering when elements are vertically centered in absolutely positioned containers

### What Needs to Be Done

**Option A: Fix the parent container**
- Remove `.items_center()` from the absolute positioned parent
- Manually calculate vertical position to center the dock
- Ensure the dock has enough vertical space for borders

**Option B: Change the layout approach**
- Don't use absolute positioning for the dock
- Use a different centering method (transform, margin auto, etc.)
- Ensure the dock is not clipped by parent containers

**Option C: Investigate GPUI**
- Check if this is a known GPUI bug with borders on centered elements
- Look for similar issues in other parts of the codebase
- Consider using a different border rendering approach (box-shadow, outline, etc.)

### Expected Result
The dock should show a complete border on all four sides (top, bottom, left, right) with the specified border radius.

---

## Issue 2: Screen Switching Creates Blank Panes Instead of Full-Width Screens

### ⚠️ CANNOT BE TESTED - Screen dock border must be fixed first

**Status:** A potential fix was applied to `ensure_screen_pane()` in `crates/workspace/src/workspace.rs`, but it cannot be tested because the screen dock buttons are not usable without visible borders.

**Attempted Fix (April 13, 2026):**
Changed `ensure_screen_pane()` to return the center pane for ALL screen types instead of creating splits.

```rust
fn ensure_screen_pane(
    &mut self,
    kind: WorkspaceScreenKind,
    window: &mut Window,
    cx: &mut Context<Self>,
) -> Entity<Pane> {
    if let Some(existing) = self.pane_for_screen_kind(kind, cx) {
        return existing;
    }

    // ALL screen kinds (Editor, Browser, Terminal) use the same center pane
    // The pane's visible_item_indices() will filter tabs by screen kind
    self.last_active_center_pane
        .clone()
        .and_then(|pane| pane.upgrade())
        .unwrap_or_else(|| self.active_pane.clone())
}
```

**Why This Cannot Be Verified:**
The screen dock buttons (Editor, Browser, Terminal) are not clickable/usable because the dock has no visible top and bottom borders, making it impossible to see or interact with the buttons properly.

**What Needs to Happen:**
1. **FIRST:** Fix the screen dock border issue (Issue 1) so the buttons are visible and usable
2. **THEN:** Test if the screen switching fix works correctly

### Original Problem Description

When switching from Code screen to Terminal or Browser screen, the system creates a small blank pane on the side instead of showing a full-width screen with the appropriate tabs.

**Expected Behavior:**
- **Code Screen (Default):** Full-width pane with only code editor tabs
- **Terminal Screen:** Full-width pane with only terminal tabs (should look like code screen)
- **Browser Screen:** Full-width pane with only browser tabs (should look like code screen)

**Actual Behavior:**
- **Code Screen:** ✅ Works correctly
- **Terminal Screen:** ❌ Creates a small blank pane on the left side
- **Browser Screen:** ❌ Creates a small blank pane on the right side

**Root Cause:**
The old `ensure_screen_pane()` was calling `split_pane()` for Terminal (left split) and Browser (right split), creating narrow side panes instead of reusing the full-width center pane.

**Potential Solution Applied:**
Changed the function to return the center pane for all screen types, letting the existing `visible_item_indices()` filtering show only matching tabs.

**Verification Status:** ❌ CANNOT TEST - Screen dock border must be fixed first

---

---

## Additional Context

### Project Structure
- **Rust codebase** using GPUI framework
- **Windows platform** (primary target)
- **Zed Editor fork** with custom features

### Key Files
- `crates/title_bar/src/title_bar.rs` - Title bar and screen dock
- `crates/workspace/src/workspace.rs` - Workspace and screen management
- `crates/workspace/src/pane.rs` - Pane and tab management
- `crates/platform_title_bar/src/platform_title_bar.rs` - Platform title bar rendering

### Build Commands
```bash
just fmt      # Format code
just run      # Build and run
cargo test    # Run tests
```

### Documentation
- `AGENTS.md` - AI agent rules and conventions
- `SCREENS.md` - Screen category system documentation (partially incorrect)
- `TODO.md` - Current tasks
- `CHANGELOG.md` - Change history

---

## Request to Next AI

**CRITICAL PRIORITY:** Fix the screen dock border issue FIRST. Nothing else can be tested or verified until the dock buttons are visible and usable.

### Issue 1: Screen Dock Border (TOP PRIORITY)
Make the screen dock border visible on all four sides. The top and bottom borders must show. This has been attempted 8+ times without success, suggesting a deeper GPUI rendering issue with vertically centered elements in absolutely positioned containers.

### Issue 2: Screen Switching (CANNOT TEST YET)
A potential fix has been applied to make Terminal and Browser screens use the full-width center pane, but it cannot be tested until Issue 1 is resolved.

**The screen dock border MUST be fixed before anything else can be verified.**

Thank you!
