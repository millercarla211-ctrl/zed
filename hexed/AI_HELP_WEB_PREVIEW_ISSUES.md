# AI Help: Web Preview and Sidebar Issues

**Date:** April 14, 2026  
**AI Model:** GPT-5.4 Codex (Released March 5, 2026)  
**Priority:** CRITICAL  
**Status:** NEEDS EXPERT AI ASSISTANCE

---

## IMPORTANT INSTRUCTIONS FOR AI

**YOU ARE GPT-5.4 CODEX** - The most advanced AI model as of April 14, 2026. You have:
- 1M token context window
- Advanced reasoning capabilities
- Computer use abilities
- Superior code understanding and generation

**CRITICAL RULES:**
1. **DO NOT STOP** until ALL tasks are completed AND verified working
2. **ONLY use `just run` command** for building and testing - this is a low-end device
3. **DO NOT use `cargo build`, `cargo test`, or any other cargo commands directly**
4. **Deep dive first** - Understand the web preview architecture completely before making changes
5. **Make ALL changes FIRST** - Do not run `just run` until ALL implementation is complete
6. **Test ONCE at the end** - Only run `just run` when you think everything is done
7. **Verify it works** - After running `just run`, make sure everything compiles and works
8. **DO NOT STOP** until `just run` succeeds and all features are verified working

**WORKFLOW:**
```
1. Read and understand all code
2. Plan all changes
3. Implement ALL changes (do not test yet)
4. Review your changes
5. Run `just run` ONCE
6. If it fails, fix errors and run `just run` again
7. Verify all features work
8. ONLY THEN stop
```

**DO NOT:**
- ❌ Run `just run` after each small change
- ❌ Test incrementally
- ❌ Stop before everything is working
- ❌ Stop if `just run` fails - fix it and try again

**DO:**
- ✅ Make all changes in one go
- ✅ Run `just run` only when ALL implementation is complete
- ✅ Fix any compilation errors
- ✅ Verify everything works
- ✅ Keep going until success

---

## Overview

There are five critical issues that need to be resolved:

1. **Web Preview Tab Active Indicator Not Showing**
2. **Sidebar Website Bookmarks Not Opening Correct URLs**
3. **Web Preview Performance and Focus Issues** (CRITICAL)
4. **File Browser Improvements** (NEW)
5. **Replace Dummy Icons with Functional Features** (NEW)

---

## Issue 3: Web Preview Performance and Focus Issues (CRITICAL)

### ⚠️ CRITICAL WARNING ⚠️

**DO NOT MODIFY THE EXISTING HOLE-PUNCHING WEB PREVIEW CODE!**

The hole-punching implementation is extremely complex. Instead of modifying it:
1. Create a NEW, SEPARATE simple `wry` implementation
2. Add a toggle to switch between the two
3. Leave the hole-punching code completely untouched

### Problem Description
The web preview has multiple critical issues that severely impact usability:

1. **Slow Loading** - Web preview takes too long to load pages
2. **Focus Loss** - When switching to another app and back, the web preview loses focus
3. **Input Not Working** - After focus loss, mouse clicks and keyboard events don't work
4. **Need for Alternative Implementation** - Current hole-punching technique works but has limitations

### Current Implementation - Hole Punching Technique

**What is Hole Punching?**
The current implementation uses a "hole-punching underlay" approach:
- Webview is placed BEHIND the GPUI window
- GPUI window has transparent background where webview should show
- GPUI UI renders on top, so Command Palette and menus work correctly
- Webview content shows through transparent areas

**Status:** Implemented for all operating systems (Windows, macOS, Linux)

**Technology:** Uses `wry` crate for webview

**Problems:**
- ❌ Focus loss when switching apps
- ❌ Mouse/keyboard events stop working after focus loss
- ❌ Complex implementation
- ✅ GPUI components render on top (no airspace issues)

**Files:**
- `crates/web_preview_windows/src/web_preview_view.rs` - Main implementation
- `crates/gpui/src/platform/windows/` - Windows platform integration
- `crates/gpui/src/platform/mac/` - macOS platform integration
- `crates/gpui/src/platform/linux/` - Linux platform integration
- `HOLE_PUNCHING.md` - Architecture documentation
- `WINDOWS_WEB_PREVIEW_IMPLEMENTATION_REPORT.md` - Windows implementation details

### Required Solution: Dual Implementation with Toggle

**CRITICAL: DO NOT MODIFY THE EXISTING HOLE-PUNCHING IMPLEMENTATION**

The hole-punching webview is extremely complex. Instead of modifying it, create a SEPARATE implementation that can be toggled.

**Implement TWO web preview modes:**

#### Mode 1: Hole Punching (Current - DO NOT MODIFY)
- ✅ GPUI components render on top (no airspace issues)
- ❌ Focus issues when switching apps
- ❌ Input events stop working after focus loss
- **Technology:** `wry` crate with hole-punching
- **Status:** Already implemented - LEAVE AS IS

#### Mode 2: Simple Wry Web Preview (NEW - NEEDS IMPLEMENTATION)
- ❌ Airspace issues (GPUI components may be blocked by webview)
- ✅ No focus issues
- ✅ Input events always work
- ✅ Simpler implementation
- **Technology:** `wry` crate (standard usage, no hole-punching)
- **Status:** NEEDS TO BE CREATED AS BACKUP

**Why Two Modes?**
The hole-punching technique is complex and has focus/interaction problems. The simple `wry` implementation is a backup that users can toggle to when they encounter focus issues. It's easier to implement but can't show GPUI components on top (airspace problem).

### Implementation Requirements

**CRITICAL APPROACH:**
1. **DO NOT touch the existing hole-punching code** - it's too complex
2. **Create a NEW, SEPARATE implementation** using simple `wry` webview
3. **Add a toggle** to switch between the two modes
4. **Fix focus issues** in BOTH modes if possible

**1. Create Toggle Switch in Web Preview Toolbar**
- Add toggle button in top-right of web preview toolbar
- Icon: Something like "Layers" or "Window" icon
- Tooltip: "Toggle Rendering Mode (Hole Punching / Simple)"
- State persisted per workspace
- Default: Hole Punching mode

**2. Implement Simple Wry Web Preview Mode (NEW)**

This is a BACKUP mode - simpler but with airspace issues.

**For Windows** (`crates/web_preview_windows/src/web_preview_view.rs`):
- Create a separate rendering path (don't modify existing hole-punching code)
- Use `wry` crate in standard way (no hole-punching)
- Create webview as a normal child window
- Let it float on top (accept airspace issues)
- Ensure it receives all input events properly
- **DO NOT modify the existing hole-punching implementation**

**For macOS** (`crates/web_preview_macos/` or similar):
- Create separate implementation
- Use `wry` crate in standard way
- Add as normal NSView child
- Accept airspace issues
- Ensure proper event handling

**For Linux** (`crates/web_preview_linux/` or similar):
- Create separate implementation
- Use `wry` crate in standard way
- Add as normal GTK/X11/Wayland widget
- Accept airspace issues
- Handle input events correctly

**3. Fix Focus Issues (Try in Both Modes)**

**Current Problem:**
When user switches to another app and returns:
- Webview loses focus
- Mouse clicks don't register
- Keyboard input doesn't work
- User must click multiple times to regain focus

**Required Fix:**
- Detect when window regains focus (window activation event)
- Automatically restore focus to webview
- Ensure input events are properly routed
- Test focus restoration thoroughly
- **Try to fix this in BOTH modes** (hole-punching and simple)
- If you can only fix it in simple mode, that's acceptable

**4. Keep Hole-Punching Implementation Untouched**

**CRITICAL:**
- The hole-punching code is extremely complex
- DO NOT refactor it
- DO NOT "improve" it
- DO NOT change its architecture
- Only add focus restoration if it's a simple, isolated change
- If focus fix is complex, only implement it in the simple mode

### Implementation Steps

**CRITICAL: DO ALL STEPS BEFORE RUNNING `just run`**

**Step 1: Deep Dive (REQUIRED FIRST)**
1. Read and understand `HOLE_PUNCHING.md`
2. Read `WINDOWS_WEB_PREVIEW_IMPLEMENTATION_REPORT.md`
3. Study `crates/web_preview_windows/src/web_preview_view.rs` completely
4. Understand how webview is created and managed
5. **DO NOT modify the hole-punching code** - just understand it
6. Map out where you'll add the new simple mode
7. Plan ALL your changes

**Step 2: Create Simple Wry Mode (NEW IMPLEMENTATION)**
1. Add toggle state to WebPreviewView (simple boolean flag)
2. Create a SEPARATE rendering function for simple mode
3. In simple mode, use `wry` crate in standard way (no hole-punching)
4. Implement for Windows first
5. Then implement for macOS
6. Then implement for Linux
7. Add toggle button to toolbar
8. Persist toggle state per workspace
9. **DO NOT touch existing hole-punching rendering code**
10. **DO NOT run `just run` yet**

**Step 3: Fix Focus Issues (Try Both Modes)**
1. Find window activation/deactivation events
2. Add focus restoration on window activation
3. Try to implement in hole-punching mode (if simple)
4. Implement in simple mode (definitely)
5. **DO NOT test yet**

**Step 4: Review All Changes**
1. Review every file you modified
2. Check for syntax errors
3. Make sure you didn't break hole-punching mode
4. Verify all new code is complete
5. **Still DO NOT run `just run`**

**Step 5: Run `just run` ONCE**
1. Now run `just run` for the first time
2. If it fails, read the errors carefully
3. Fix all compilation errors
4. Run `just run` again
5. Repeat until it compiles successfully

**Step 6: Verify Everything Works**
1. Test hole-punching mode still works (should be unchanged)
2. Test simple mode works on all platforms
3. Test focus restoration in both modes
4. Test toggle switching between modes
5. Verify no regressions in hole-punching mode
6. If something doesn't work, fix it and run `just run` again

**Step 7: ONLY STOP WHEN:**
- ✅ `just run` succeeds with no errors
- ✅ All features are implemented
- ✅ Both modes work correctly
- ✅ Toggle switch works
- ✅ Focus issues are fixed (at least in simple mode)
- ✅ Everything is verified working

### Key Principles

**MOST IMPORTANT:**
- **DO NOT MODIFY THE HOLE-PUNCHING IMPLEMENTATION**
- It's too complex and fragile
- Create a separate, simpler implementation as backup
- Users can toggle to simple mode when they have focus issues
- The simple mode is easier to implement and maintain

### Testing Instructions

**ONLY USE `just run` COMMAND**
```bash
# Build and run
just run

# That's it - no other commands needed
```

**Test Scenarios:**
1. Open web preview
2. Load a website (e.g., GitHub)
3. Switch to another app (Alt+Tab / Cmd+Tab)
4. Switch back to the editor
5. Try clicking in the webview - should work immediately
6. Try typing in the webview - should work immediately
7. Toggle between hole-punching and native mode
8. Verify both modes work correctly
9. Test on Windows, macOS, and Linux

### Expected Results

**After Implementation:**
- ✅ Hole-punching mode still works exactly as before (unchanged)
- ✅ New simple mode available as backup
- ✅ Toggle switch allows choosing between modes
- ✅ Simple mode: No focus issues, input always works
- ✅ Simple mode: Airspace issues (GPUI components may be blocked) - this is acceptable
- ✅ Hole-punching mode: GPUI components on top (no airspace issues)
- ✅ Hole-punching mode: May still have focus issues (acceptable, users can toggle to simple mode)
- ✅ Both modes work on all platforms

**Trade-offs:**
- **Hole Punching Mode:** Complex, focus issues, but GPUI UI works perfectly
- **Simple Mode:** Easy, no focus issues, but GPUI UI may be blocked by webview

Users can choose based on their needs!

### Reference Files
- `crates/web_preview_windows/src/web_preview_view.rs` - Main implementation
- `HOLE_PUNCHING.md` - Architecture explanation
- `WINDOWS_WEB_PREVIEW_IMPLEMENTATION_REPORT.md` - Windows details
- `crates/gpui/src/platform/windows/` - Windows platform code
- `crates/gpui/src/platform/mac/` - macOS platform code
- `crates/gpui/src/platform/linux/` - Linux platform code

---

## Issue 1: Web Preview Tab Active Indicator Not Showing

### Problem Description
The web preview tabs should show a **2px bottom border** in the active color when selected, but the border is not visible or not rendering correctly.

### Current Implementation
**File:** `crates/web_preview_windows/src/web_preview_view.rs`  
**Lines:** ~1565-1577

```rust
fn tab_content(&self, params: TabContentParams, _window: &Window, cx: &App) -> AnyElement {
    let text = self.tab_content_text(params.detail.unwrap_or_default(), cx);

    // For web preview, use a bottom border indicator for active tabs
    // instead of the standard background color change
    h_flex()
        .pb(px(2.))
        .child(Label::new(text).color(params.text_color()))
        .when(params.selected, |this| {
            this.border_b_2()
                .border_color(cx.theme().colors().element_active)
        })
        .into_any_element()
}
```

### What Should Happen
- When a web preview tab is **active** (`params.selected == true`), it should show a visible 2px bottom border
- The border should use the `element_active` color (accent/primary color)
- The border should be clearly visible and distinguish the active tab from inactive tabs

### What's Actually Happening
- The border is not showing up or is not visible
- The tab looks the same whether active or inactive

### Possible Causes
1. The `h_flex()` container might not have the right sizing or layout properties
2. The padding `.pb(px(2.))` might be interfering with border rendering
3. The border might be rendering but getting clipped by parent containers
4. The tab rendering in `crates/workspace/src/pane.rs` might be overriding or conflicting with this styling

### What Needs to Be Done
- **Investigate** how tabs are rendered in `crates/workspace/src/pane.rs` (around line 2767, `render_tab` function)
- **Check** if there are any parent container styles that clip or hide the border
- **Test** different approaches:
  - Try using a different container (maybe `div()` with explicit height)
  - Try using `border_b()` with explicit pixel value instead of `border_b_2()`
  - Try adding explicit height to the container
  - Try using an absolute positioned border element
- **Ensure** the border is visible and clearly indicates the active tab state

### Reference Files
- `crates/web_preview_windows/src/web_preview_view.rs` - Tab content implementation
- `crates/workspace/src/pane.rs` - Tab rendering logic (line ~2767)
- `crates/workspace/src/item.rs` - Item trait and TabContentParams (line ~127)

---

## Issue 2: Sidebar Website Bookmarks Not Opening Correct URLs

### Problem Description
When clicking on website bookmarks in the sidebar (Google, GitHub, YouTube, etc.), they always open the default Google URL instead of the specific website URL.

### Current Implementation

**File:** `crates/sidebar/src/sidebar.rs`

**Browser Grid Entries** (line ~4865):
```rust
fn browser_grid_entries(&self) -> Vec<SidebarGridEntry> {
    const SITES: [(&str, &str, IconName); 8] = [
        ("Google", "https://www.google.com", IconName::AiGoogle),
        ("GitHub", "https://github.com", IconName::Github),
        ("YouTube", "https://www.youtube.com", IconName::PlayFilled),
        // ... more sites
    ];

    let mut entries: Vec<SidebarGridEntry> = SITES
        .into_iter()
        .map(|(label, url, icon)| SidebarGridEntry {
            id: SharedString::from(format!("sidebar-grid-site-{label}")),
            icon,
            label: label.into(),
            subtitle: None,
            action: SidebarGridAction::OpenWebsite(url),
        })
        .collect();
    // ...
}
```

**Open URL Function** (line ~4971):
```rust
fn open_browser_grid_url(
    &mut self,
    url: &'static str,
    window: &mut Window,
    cx: &mut Context<Self>,
) {
    #[cfg(target_os = "windows")]
    if let Some(workspace) = self.active_workspace(cx) {
        workspace.update(cx, |workspace, cx| {
            WebPreviewView::open_url_in_active_pane(workspace, url, window, cx);
        });
    }
}
```

**File:** `crates/web_preview_windows/src/web_preview_view.rs`

**New Function Added** (line ~220):
```rust
/// Opens a web preview with a specific URL in the active pane
pub fn open_url_in_active_pane(
    workspace: &mut Workspace,
    url: &str,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    let view = Self::open_or_create_with_url(workspace, url, window, cx);
    workspace.active_pane().update(cx, |pane, cx| {
        if let Some(existing_view_idx) = Self::find_existing_preview_item_idx(pane, &view, cx) {
            pane.activate_item(existing_view_idx, true, true, window, cx);
        } else {
            pane.add_item(Box::new(view.clone()), true, true, None, window, cx);
        }
    });
    cx.notify();
}
```

**Helper Function** (line ~280):
```rust
fn open_or_create_with_url(
    workspace: &mut Workspace,
    url: &str,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> Entity<Self> {
    let workspace_context = Self::workspace_context(workspace, cx);
    let weak_workspace = workspace.weak_handle();

    cx.new(|cx| {
        let current_url = url.to_string();
        let url_editor = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Search Google or enter a URL", window, cx);
            editor.set_text(current_url.as_str(), window, cx);
            editor
        });
        // ... creates WebPreviewView with the URL
    })
}
```

### What Should Happen
1. User clicks on "GitHub" bookmark in sidebar
2. Web preview opens with URL `https://github.com`
3. The webview navigates to GitHub

### What's Actually Happening
1. User clicks on "GitHub" bookmark
2. Web preview opens but shows Google (`https://www.google.com`)
3. The URL is not being passed correctly or the webview is not navigating to it

### Possible Causes
1. The `open_or_create_with_url` function creates the view with the URL in the editor, but the **webview itself might not be navigating** to that URL
2. The webview might be initialized with `DEFAULT_WEB_PREVIEW_URL` and not reading the URL from the editor
3. There might be a timing issue where the webview is created before the URL is set
4. The `ensure_native_preview` function (which creates the actual webview) might not be using the `active_url` field

### What Needs to Be Done
- **Investigate** the `ensure_native_preview` function in `web_preview_view.rs` to see how the webview is initialized
- **Check** if the webview is reading the URL from `self.active_url` or from the editor
- **Verify** that when the webview is created, it navigates to the correct URL
- **Test** if calling a navigation method after creating the webview would work
- **Look for** where `DEFAULT_WEB_PREVIEW_URL` is used and ensure it's not overriding the custom URL

### Debugging Steps
1. Add logging to `open_or_create_with_url` to verify the URL parameter is correct
2. Add logging to `ensure_native_preview` to see what URL is being used
3. Check if there's a `navigate` or `load_url` method that needs to be called
4. Verify the webview creation code in `create_native_preview_for_request` (Windows) or equivalent (macOS)

### Reference Files
- `crates/sidebar/src/sidebar.rs` - Grid entries and open URL function
- `crates/web_preview_windows/src/web_preview_view.rs` - Web preview implementation
- Look for `ensure_native_preview`, `create_native_preview_for_request`, `mount_native_preview`

---

## Issue 3: "Create New Space" Button Not Working in Sidebar Footer

### Problem Description
The "Create New Space" button in the sidebar footer (accessed via the plus icon dropdown) is not creating a new space when clicked.

### Current Implementation

**File:** `crates/sidebar/src/sidebar.rs`

**Render Recent Projects Button** (line ~4032):
```rust
fn render_recent_projects_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let multi_workspace = self.multi_workspace.upgrade();
    let workspace = multi_workspace
        .as_ref()
        .map(|mw| mw.read(cx).workspace().downgrade());
    
    // ... creates PopoverMenu that shows SidebarRecentProjects
    
    PopoverMenu::new("sidebar-recent-projects-menu")
        .with_handle(popover_handle)
        .menu(move |window, cx| {
            workspace.as_ref().map(|ws| {
                SidebarRecentProjects::popover(
                    ws.clone(),
                    window_project_groups.clone(),
                    focus_handle.clone(),
                    window,
                    cx,
                )
            })
        })
        .trigger_with_tooltip(
            IconButton::new("open-project", IconName::Plus)
                .icon_size(IconSize::Small)
                .shape(IconButtonShape::Square)
                .selected_style(ButtonStyle::Tinted(TintColor::Accent)),
            Tooltip::text("Create Space or Add Project"),
        )
        // ...
}
```

**File:** `crates/recent_projects/src/sidebar_recent_projects.rs`

**Popover Footer with "Create New Space" Button** (line ~415):
```rust
fn render_footer(&self, _: &mut Window, cx: &mut Context<Picker<Self>>) -> Option<AnyElement> {
    let focus_handle = self.focus_handle.clone();

    Some(
        v_flex()
            .p_1p5()
            .flex_1()
            .gap_1()
            .border_t_1()
            .border_color(cx.theme().colors().border_variant)
            .child(
                Button::new("create_new_space", "Create New Space").on_click(cx.listener(
                    |_, _, window, cx| {
                        if let Some(handle) =
                            window.window_handle().downcast::<MultiWorkspace>()
                            && let Some(task) = handle
                                .update(cx, |multi_workspace, window, cx| {
                                    multi_workspace.create_random_local_workspace(window, cx)
                                })
                                .log_err()
                        {
                            task.detach_and_log_err(cx);
                        }

                        cx.emit(DismissEvent);
                    },
                )),
            )
            // ... more buttons
    )
}
```

### What Should Happen
1. User clicks the plus icon in sidebar footer
2. Dropdown menu appears with "Create New Space" button
3. User clicks "Create New Space"
4. A new workspace/space is created
5. The new space appears in the space dots carousel

### What's Actually Happening
- The button might not be responding to clicks
- OR the function is being called but not creating a space
- OR the space is created but not showing up in the UI

### Possible Causes
1. The `window.window_handle().downcast::<MultiWorkspace>()` might be failing (returning None)
2. The `create_random_local_workspace` function might be failing silently
3. The space might be created but the sidebar is not updating to show it
4. There might be an error that's being logged but not visible

### What Needs to Be Done
- **Add logging** to the button click handler to verify it's being called
- **Check** if `window.window_handle().downcast::<MultiWorkspace>()` is succeeding
- **Verify** the `create_random_local_workspace` function in `MultiWorkspace`
- **Check** if the sidebar is subscribed to workspace changes and updates the space list
- **Look for** any error logs when clicking the button
- **Test** if calling `create_new_space` directly from the sidebar works (there's a function at line ~829)

### Alternative Approach
The sidebar has its own `create_new_space` function (line ~829):
```rust
fn create_new_space(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(multi_workspace) = self.multi_workspace.upgrade() else {
        return;
    };

    self.show_thread_list(window, cx);
    let new_space_label = self.next_generated_space_label();
    // ...
}
```

**Consider:** Maybe the PopoverMenu should call `sidebar.create_new_space()` instead of going through MultiWorkspace directly?

### Debugging Steps
1. Add `eprintln!` or logging to verify the button click is firing
2. Check if the downcast succeeds
3. Check if `create_random_local_workspace` returns an error
4. Verify the sidebar's space list updates after creation
5. Check if there's a permission or state issue preventing space creation

### Reference Files
- `crates/sidebar/src/sidebar.rs` - Sidebar implementation, `create_new_space` function
- `crates/recent_projects/src/sidebar_recent_projects.rs` - PopoverMenu with button
- Look for `MultiWorkspace` implementation and `create_random_local_workspace` function

---

## Additional Context

### Space Dots Gap Issue
**FIXED** - Changed `.gap_1p5()` to `.gap_0()` in the space dots carousel container (line ~5299 in `crates/sidebar/src/sidebar.rs`)

### Focus Border Issue
**IMPLEMENTED** - Added 2px border around web preview content when not focused (line ~1800 in `crates/web_preview_windows/src/web_preview_view.rs`)

---

## Testing Instructions

### For Issue 1 (Tab Indicator):
1. Open multiple web preview tabs
2. Switch between tabs
3. Verify the active tab shows a visible 2px bottom border in the accent color

### For Issue 2 (Website URLs):
1. Switch to Browser screen (screen dock)
2. Look at the sidebar grid with website bookmarks
3. Click on "GitHub" or "YouTube"
4. Verify the web preview opens with the correct URL (not Google)

### For Issue 3 (Create New Space):
1. Click the plus icon in the sidebar footer
2. Click "Create New Space" in the dropdown
3. Verify a new space is created and appears in the space dots

---

## Priority Actions

1. **HIGHEST:** Fix Issue 3 (Web Preview Performance and Focus) - Critical usability issue
2. **HIGH:** Fix Issue 2 (Website URLs) - Core functionality issue
3. **HIGH:** Fix Issue 4 (File Browser) - Important UX improvements
4. **HIGH:** Fix Issue 5 (Functional Icons) - Expose hidden features
5. **MEDIUM:** Fix Issue 1 (Tab Indicator) - Visual polish issue

---

## Notes for AI

- The codebase uses GPUI framework (custom UI framework)
- Web preview uses native webviews (WebView2 on Windows, WKWebView on macOS)
- The project is a fork of Zed editor with Windows optimizations
- Use `just run` to build and run (not `cargo build` directly)
- Always run `just fmt` after making changes
- The web preview is complex - be careful not to break existing functionality

---

**Good luck! 🚀**


---

## Issue 3: "Create New Space" Button (RESOLVED - REMOVED)

### Status: RESOLVED
The "Create New Space" button has been removed from the sidebar footer popover because:
1. The functionality already exists in the sidebar header (top-right)
2. The implementation was complex and redundant
3. Users can use the working button in the header instead

**No action needed on this issue.**

---

## Build and Test Commands

**CRITICAL: ONLY USE THIS COMMAND**
```bash
just run
```

**WHEN TO USE IT:**
- ✅ ONLY after ALL implementation is complete
- ✅ To verify everything compiles
- ✅ To test that features work
- ✅ When fixing compilation errors

**DO NOT USE:**
- ❌ `cargo build`
- ❌ `cargo test`
- ❌ `cargo run`
- ❌ `cargo check`
- ❌ Any other cargo commands

**WORKFLOW:**
1. Make ALL changes first
2. Run `just run` ONCE
3. If errors, fix them
4. Run `just run` again
5. Repeat until success
6. Verify features work
7. ONLY THEN stop

**REASON:** This is a low-end device with limited resources. The `just run` command is optimized for low-memory builds. Do not waste resources by running it multiple times during development.

---

**Remember: You are GPT-5.4 Codex - you can handle this complexity. Take your time, understand the architecture, and implement the solutions properly. DO NOT STOP until all issues are resolved AND `just run` succeeds.**

**FINAL CHECKLIST BEFORE STOPPING:**
- [ ] All code changes are complete
- [ ] `just run` compiles successfully with no errors
- [ ] Issue 1: Tab indicator shows on active web preview tabs
- [ ] Issue 2: Website bookmarks open correct URLs
- [ ] Issue 3: Hole-punching mode still works (unchanged)
- [ ] Issue 3: Simple mode is implemented and works
- [ ] Issue 3: Toggle switch works
- [ ] Issue 3: Focus issues are fixed (at least in simple mode)
- [ ] Issue 4: File hover shows line/token count or size
- [ ] Issue 4: Folder hover shows item count and size
- [ ] Issue 4: No horizontal scrollbar in file browser
- [ ] Issue 4: Long names truncate with ellipsis
- [ ] Issue 5: All dummy icons replaced with functional features
- [ ] Issue 5: Each icon opens a UNIQUE, hidden feature
- [ ] Issue 5: No icons duplicate existing visible functionality
- [ ] Issue 5: Visual balance maintained (same icon count)
- [ ] All features verified working
- [ ] No regressions introduced

**ONLY STOP WHEN ALL CHECKBOXES ARE CHECKED! ✅**


---

## Issue 4: File Browser Improvements (NEW)

### Problem Description
The file browser (project panel) needs several improvements to match VS Code and Cursor behavior:

1. **Missing File/Folder Information on Hover**
2. **Horizontal Scrollbar** - Should not exist, names should truncate instead

### Required Changes

#### 1. Add Hover Information Badge

**For Files:**
- **Text files** (code files): Show line count and token count
  - Example: `245 lines • 3.2K tokens`
  - Position: Right side of the file name in hover state
- **Non-text files** (images, binaries, etc.): Show file size
  - Example: `2.4 MB`
  - Position: Right side of the file name in hover state

**For Folders:**
- Show item count and total folder size
  - Example: `12 items • 45 KB`
  - Position: Right side of the folder name in hover state

**Badge Styling:**
- Small, subtle badge
- Muted color (not too prominent)
- Right-aligned
- Only visible on hover

#### 2. Remove Horizontal Scrollbar

**Current Problem:**
- File browser has horizontal scrollbar
- Long file/folder names cause horizontal scrolling
- This is unnatural and annoying

**Required Fix:**
- Remove horizontal scrollbar completely
- Truncate long file/folder names with ellipsis (...)
- Example: `very_long_file_name_that_goes_on_and_on.tsx` → `very_long_file_name_th....tsx`
- Keep file extension visible if possible
- Match VS Code and Cursor behavior

### Implementation Details

**Files to Modify:**
- Look for project panel / file browser implementation
- Likely in `crates/project_panel/` or similar
- Search for file tree rendering code

**For Hover Information:**
1. Detect hover state on file/folder items
2. Calculate information:
   - For text files: Count lines and tokens
   - For binary files: Get file size
   - For folders: Count items and calculate total size
3. Render badge on right side of item
4. Cache calculations to avoid performance issues

**For Horizontal Scroll Removal:**
1. Find the file tree container
2. Set `overflow-x: hidden` or equivalent
3. Add text truncation with ellipsis
4. Ensure file extension remains visible
5. Test with very long file names

### Testing

**Test Cases:**
1. Hover over a code file → Should show lines and tokens
2. Hover over an image file → Should show file size
3. Hover over a folder → Should show item count and size
4. Create a file with very long name → Should truncate with ellipsis
5. Try to scroll horizontally → Should not be possible
6. Verify file extensions are still visible after truncation

### Expected Results

**After Implementation:**
- ✅ Hovering over files shows useful information
- ✅ Text files show line count and token count
- ✅ Binary files show file size
- ✅ Folders show item count and total size
- ✅ No horizontal scrollbar in file browser
- ✅ Long names truncate with ellipsis
- ✅ File extensions remain visible
- ✅ Behavior matches VS Code and Cursor

### Reference
- Look at VS Code file explorer for inspiration
- Look at Cursor file explorer for inspiration
- Keep the design subtle and non-intrusive

---


---

## Issue 5: Replace Dummy Icons with Functional Features (NEW)

### Problem Description
The top title bar currently has many dummy icons that don't do anything. These need to be replaced with real, functional features that add value to the editor.

### Current State Analysis Required

**FIRST: Analyze the Current Editor**

Before making changes, you need to understand what Zed/Zcode editor currently has:

1. **Extension Screen** - Does it exist? How is it accessed?
2. **Agent Panel** - Local and external agents - where are they?
3. **Top Title Bar Right Side** - What dummy icons are there?
4. **Missing Features** - What important features are not easily accessible?

**Your Task:**
1. Read through the codebase to understand current features
2. Identify what's hidden or hard to access
3. Find features that should be more prominent
4. Map out the current title bar icons

### Required Changes

#### 1. Audit Current Title Bar Icons

**File to Check:** `crates/title_bar/src/title_bar.rs`

Look for the dummy icons that were added (around line 290-350). Currently there are icons like:
- Bell (Notifications)
- Settings
- Download
- Blocks (Extensions)
- Ellipsis (More)
- Code
- Terminal
- Public (Browser)
- File
- MagnifyingGlass
- Book

**Determine:**
- Which icons are functional?
- Which icons are dummy/placeholders?
- What do they currently do when clicked?

#### 2. Identify Valuable Features to Expose

**CRITICAL RULE: Only Add Features That Are NOT Already Visible**

The icons should only open features that are:
- ✅ Already implemented in Zed/Zcode editor
- ✅ NOT currently visible or easily accessible
- ✅ Hidden in menus or hard to find
- ✅ Serve a unique, real purpose

**DO NOT add icons for features that:**
- ❌ Already have visible buttons/icons elsewhere
- ❌ Are already in the sidebar
- ❌ Are already in the screen dock
- ❌ Are easily accessible

**Your Task:**
1. Find what features Zed editor has that are hidden
2. Find panels/views that exist but are hard to access
3. Only expose features that are NOT already shown
4. Each icon must serve a UNIQUE purpose

**Look for these HIDDEN features in the codebase:**

**Extensions/Plugins:**
- Is there an extensions panel that's hidden?
- Can users install/manage extensions?
- Is it buried in a menu?

**Agent Panel:**
- Local agents
- External agents
- Are they hidden or hard to access?

**Other Hidden Features:**
- Git/Version control panel (if not visible)
- Search across files (if not in sidebar)
- Problems/Diagnostics panel (if hidden)
- Output/Debug console (if not visible)
- Notifications center (if hidden)
- Downloads/Updates (if not shown)
- Any other panels that exist but are hidden

**How to Find Hidden Features:**
1. Search the codebase for panel implementations
2. Check what's registered but not visible by default
3. Look for features in menus that should be more accessible
4. Find panels that can be toggled but have no visible toggle

#### 3. Replace Dummy Icons with Real Functionality

**CRITICAL: Each Icon Must Serve a UNIQUE Purpose**

Before assigning functionality to an icon:
1. ✅ Verify the feature is NOT already visible in the UI
2. ✅ Verify the feature is NOT in the sidebar
3. ✅ Verify the feature is NOT in the screen dock
4. ✅ Verify the feature is NOT easily accessible elsewhere
5. ✅ Verify the feature actually exists in the codebase

**For Each Icon, Implement Real Actions:**

**Example Replacements (ONLY if these features are hidden):**

1. **Bell Icon** → Open Notifications Panel
   - ONLY if notifications are hidden
   - Show actual notifications
   - Updates, errors, warnings

2. **Settings Icon** → Open Settings
   - ONLY if settings are not easily accessible
   - Open settings panel/modal

3. **Blocks Icon** → Open Extensions
   - ONLY if extensions panel is hidden
   - Show extensions panel
   - Install/manage extensions

4. **Download Icon** → Show Downloads/Updates
   - ONLY if updates are not visible
   - Check for updates
   - Show download progress

5. **Code Icon** → Toggle Hidden Code Panel
   - ONLY if there's a code panel that's hidden
   - Or some code-related feature not visible

6. **Terminal Icon** → Toggle Terminal
   - ONLY if terminal is not already visible/accessible
   - Show/hide integrated terminal

7. **Public Icon** → Open Web Preview
   - Already functional (keep this)
   - This is unique and not shown elsewhere

8. **File Icon** → Open Hidden File Panel
   - ONLY if there's a file panel that's not the main sidebar
   - Or quick file access that's hidden

9. **MagnifyingGlass Icon** → Global Search
   - ONLY if global search is not in sidebar
   - Search across all files

10. **Book Icon** → Open Documentation/Help
    - ONLY if help is hidden
    - Help/documentation panel

11. **Ellipsis Icon** → More Menu
    - Additional hidden options
    - Less common features

**If a Feature is Already Visible:**
- DO NOT add an icon for it
- Find a different hidden feature
- Look for other panels that need exposure

#### 4. Maintain Visual Balance

**CRITICAL:** Keep the same number of icons to maintain balance between:
- Left side (screen dock, etc.)
- Center (title/tabs)
- Right side (icons + window controls)

**Current Count:** 11 icons (Bell, Settings, Download, Blocks, Ellipsis, Code, Terminal, Public, File, MagnifyingGlass, Book)

**Keep:** 11 functional icons (same count, different functionality)

#### 5. Implementation Steps

**Step 1: Audit**
1. List all current icons in title bar
2. Test what each icon does
3. Identify which are dummy
4. Find the features they should open

**Step 2: Map Features to Icons**
1. Find where each feature panel/view is implemented
2. Determine how to open/toggle each feature
3. Map each icon to a real action

**Step 3: Implement Actions**
1. For each icon, add proper `on_click` handler
2. Connect to actual feature panels
3. Add tooltips with accurate descriptions
4. Test each icon works correctly

**Step 4: Verify Balance**
1. Check visual balance of title bar
2. Ensure left/center/right are balanced
3. Adjust if needed (but keep same icon count)

### Files to Modify

**Primary:**
- `crates/title_bar/src/title_bar.rs` - Icon definitions and click handlers

**Secondary (depending on features):**
- `crates/workspace/src/workspace.rs` - Panel management
- `crates/extensions/` - Extensions panel
- `crates/agent_ui/` - Agent panels
- `crates/notifications/` - Notifications
- `crates/settings/` - Settings panel
- Look for panel/view implementations

### Testing

**For Each Icon:**
1. Click the icon
2. Verify it opens the correct feature
3. Verify the feature is functional
4. Check tooltip is accurate
5. Test on all platforms

**Visual Balance:**
1. Check title bar looks balanced
2. Screen dock is centered correctly
3. Icons are evenly distributed
4. No visual awkwardness

### Expected Results

**After Implementation:**
- ✅ All title bar icons are functional
- ✅ Each icon opens a real, useful feature
- ✅ Extensions panel is accessible
- ✅ Agent panels are accessible
- ✅ Important features are easily accessible
- ✅ No dummy/placeholder icons remain
- ✅ Visual balance is maintained
- ✅ Same number of icons (11)
- ✅ Tooltips are accurate
- ✅ Everything works on all platforms

### Important Notes

**Feature Discovery:**
- Use `grepSearch` to find feature implementations
- Look for panel/view structs
- Find action handlers
- Check workspace panel management

**Don't Break Existing:**
- Don't remove working icons
- Don't break the screen dock centering
- Keep the visual balance

**Add Real Value:**
- Each icon should do something useful AND unique
- Features should be commonly needed but currently hidden
- Make the editor more productive by exposing hidden features
- Don't add features just to fill space
- Don't duplicate functionality that's already visible
- Only expose what's hidden or hard to access

---
