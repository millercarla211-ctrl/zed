# Changelog

All notable changes to this Codex fork will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased]

### April 12, 2026 - Title Bar Screen Dock
**Moved screen navigation and creation into a centered dock**

- Added a centered rounded screen dock in the title bar and moved the active path and branch display into it
- Added default screen switching buttons for Editor, Browser, and Terminal, plus dock entries for extra active-pane screens
- Made the dock add menu context-aware so each screen type creates its own matching item instead of mixing file, browser, and terminal creation
- Changed the web preview tab-bar add button to create only a new web preview
- Routed terminal creation from the dock and terminal screen add button through the center-terminal action instead of the bottom dock
- Made browser and terminal screen mode reuse the existing center pane while showing only same-type tabs instead of leaking editor tabs into those screens

### April 12, 2026 - Fixed Space Carousel Navigation and Removed Gap Calculation
**Fixed arrow navigation direction and simplified click handling**

- Swapped arrow navigation logic: LEFT arrow now scrolls right (shows later spaces), RIGHT arrow scrolls left (shows earlier spaces)
- Removed gap calculation logic completely - dots now have direct click handlers only
- Simplified mouse_up handler - no more gap detection, just clear drag state
- Each dot has its own reliable click handler for activation

### April 12, 2026 - Fixed Space Carousel Drag and Click
**Improved carousel interaction and fixed drag range**

- Restored left/right navigation arrows (they were correct, shouldn't have been removed)
- Fixed drag sensitivity: changed from 30px to 20px per dot for smoother scrolling
- Drag now properly scrolls through all 12 spaces (was limited before)
- Clicking anywhere in carousel activates nearest visible space dot
- Click detection works even in gaps between dots for better UX
- Small drags (<5px) are treated as clicks to activate spaces
- Carousel shows 7 dots at a time with proper skip/take logic

### April 12, 2026 - Draggable Space Carousel
**Added drag-to-scroll functionality to space carousel**

- Removed all right padding from top bar (no pr_1, completely flush)
- Made space carousel draggable with mouse - drag left/right to scroll through spaces
- Added pointing hand cursor to carousel to indicate draggability
- Drag sensitivity: 30px of mouse movement = 1 space dot scroll
- Drag state tracked with carousel_drag_start field
- Works alongside existing arrow button navigation

### April 12, 2026 - Sidebar Space Carousel Navigation Fix
**Fixed space carousel scrolling behavior**

- Fixed navigation arrows to properly scroll the carousel one position at a time
- Arrows now directly manipulate scroll position instead of recalculating based on active space
- Active space automatically scrolls into view when clicked
- Reduced top header right padding from pr_1p5 to pr_1 for tighter spacing
- Navigation now works smoothly: left arrow scrolls left, right arrow scrolls right

### April 12, 2026 - Sidebar Space Dots UI Improvements
**Enhanced space carousel with better UX**

- Added horizontal padding (px_2) to toolbar for better spacing
- Navigation arrows now always visible but disabled when can't navigate (no pop-in/pop-out)
- Fixed space dot navigation logic to properly scroll through all 12 spaces
- Active space automatically centers in visible range when activated
- Removed drag-and-drop (will be added later with proper GPUI implementation)
- Right-click menu temporarily disabled (will be added with proper context menu API)

### April 12, 2026 - Sidebar Space Dots Implementation
**Implemented dummy space carousel system with navigation**

- Added 12 dummy spaces by default to test horizontal scrolling
- Implemented space dots carousel at sidebar bottom with left/right navigation
- Split toolbar icons: left side (Focus Search, Previous/Next Space), right side (Refresh, Create New Space)
- Fixed space dot colors: active uses primary text color, inactive uses bordered style
- Removed icon background from "Current Space" heading
- Fixed navigation arrows to properly show/hide and scroll through spaces
- Disabled sync_space_state to avoid MultiWorkspace conflicts with dummy system
- Fixed all compiler warnings with proper allow attributes

### Upstream Sync - April 11, 2026
**Merged upstream zed-industries/zed main (commit b15969086e)**

Integrated 453 files from upstream with minimal conflicts. Our custom features (Liquid Glass, Web Preview, GPUI enhancements) remain isolated and unaffected.

**Major Upstream Changes:**
- **Agent/AI System Refactoring:** New thread branching system, worktree archive/picker, improved agent panel UI, better tool permission handling
- **Language Model Architecture:** New `language_model_core` and `language_models_cloud` crates, provider refactoring (Anthropic, OpenAI, Google), completion API improvements
- **Fuzzy Search:** New `fuzzy_nucleo` crate with better file/path matching
- **Git Integration:** Enhanced repository handling, better blame support, improved worktree management
- **Editor Enhancements:** Block comment tests, better semantic tokens, display map improvements
- **Sidebar Refactoring:** Major UI/UX improvements (6746 lines in tests, 2054 in main code)
- **Removed Features:** `crates/storybook/`, `crates/story/`, notification panel from collab_ui

**Conflicts Resolved:**
- `Cargo.lock` - Accepted upstream version (auto-regenerates with our dependencies)
- `crates/agent_ui/src/conversation_view.rs` - Accepted upstream's improved `play_notification_sound` implementation

**Custom Features Preserved:**
- ✅ Liquid Glass (separate crate, no conflicts)
- ✅ Web Preview (Windows complete, Linux/macOS in progress)
- ✅ GPUI platform enhancements (Windows/Linux/macOS)
- ✅ All documentation files

See `MERGE_CONFLICT_ANALYSIS.md` for complete merge details and risk assessment.

### Added
- Added a current cross-platform web preview status report that records the completed Windows implementation and the remaining macOS/Linux host work.
- Added a root-level Windows web preview architecture report that documents the frozen rendering/input model and the "do not touch casually" policy for the working Windows path.
- Added separate `web_preview_windows`, `web_preview_macos`, and `web_preview_linux` backend crates so platform work can continue without routing through the frozen Windows implementation.
- Added a new native `Liquid Glass` workspace item entry point beside Web Preview, backed by a GPUI-rendered GPU primitive instead of the old standalone windowed demo.
- Added a root-level Liquid Glass status report that documents the current native GPUI integration, current limitations, and the remaining renderer-level live-backdrop work.

### Changed
- Moved Web Preview navigation and action controls into the main pane tab bar, removed the in-page toolbar, and made the active web tab switch between the page title and URL editor on click.
- Began isolating non-Windows web preview work so macOS and Linux support can be developed without modifying the working Windows path.
- Tidied the root directory by moving auxiliary web preview notes and logs into the `docs` tree, while keeping the canonical Windows implementation report in the repository root.
- Updated `AGENTS.md` so future work treats the Windows web preview as frozen and develops macOS/Linux support in separate platform-specific paths.
- Turned `web_preview` into a thin facade crate that dispatches to platform-specific backends by `#[cfg]`.
- Started the Linux backend with an isolated X11-first native child-webview path and an explicit Wayland split instead of mixing Linux work into the Windows implementation.
- Made `web_preview_macos` and `web_preview_linux` target-local backend crates so they stop compiling copied Windows-only modules on the wrong OS.
- Hardened the macOS backend around its native `WKWebView` path with a platform-local browser event notifier and first-click passthrough behavior.
- Reordered the macOS native `WKWebView` under GPUI's AppKit view inside the same `NSWindow` so the backend no longer relies purely on default child-view ordering.
- Wired the Linux backend to initialize and pump GTK alongside GPUI, which is required for `wry`/WebKitGTK child webviews on X11.
- Registered the macOS and Linux web preview body as a real GPUI passthrough region so those backends now use GPUI's native platform hit-test yielding instead of only drawing the browser under the editor.
- Split the Linux native mount path into explicit X11 and Wayland branches so future Wayland host work can advance without destabilizing the X11 backend.
- Added GPUI Wayland exported parent-handle plumbing and started a dedicated Linux Wayland GTK/WebKit host path instead of aborting immediately on Wayland sessions.
- Moved the macOS backend off pure in-window child-view ordering and onto a dedicated backend-local host-window layer that reparents the native `WKWebView` under GPUI while keeping macOS focus handoff inside the macOS backend.
- Hardened the non-Windows preview lifecycle so macOS and Linux hide their native preview hosts when a tab or workspace deactivates instead of leaving underlay browser surfaces alive behind the editor.
- Fixed non-Windows host-window bounds syncing so macOS and Linux Wayland no longer rely only on unchanged local preview bounds when the parent editor window moves.
- Moved Linux X11 off the remaining legacy child-webview mount path and onto a backend-local managed host model so both Linux backends now use dedicated native host windows instead of mixing host strategies.
- Split Linux focus handoff from macOS so Linux no longer routes `focus_parent()` back into its backend host window when GPUI needs to reclaim focus.
- Fixed Linux host-window placement to derive global host-window coordinates from GPUI inner window bounds instead of using local preview coordinates directly.
- Wired macOS and Linux native host windows to GPUI's window-bounds observer so host placement keeps updating during editor window moves and resizes even without a preview-content-triggered rerender.
- Wired macOS and Linux native host windows to GPUI window-activation observers and item activity tracking so separate host windows hide and restore with the editor window instead of relying only on tab deactivation.
- Switched Linux X11 and Wayland preview hosts off unmanaged GTK popup windows and onto undecorated managed GTK toplevel hosts so non-Windows backends stop relying on popup-window-manager quirks for underlay composition.
- Attached the Linux X11 preview host to the GPUI parent window through an explicit X11 transient relationship and requested RGBA-capable visuals for both Linux host windows so the underlay host surfaces align more closely with the editor window.
- Changed the macOS host-window visibility restore path to explicitly reorder the backend host below the GPUI parent window instead of relying on a generic front-order call.
- Made the Linux X11 and Wayland hosts explicitly clear their own backgrounds with transparent Cairo painting so the non-Windows underlay path no longer depends on toolkit default background behavior.
- Synced the macOS host window's level and collection behavior from the GPUI parent window so host-window ordering stays aligned with the editor's native AppKit state.
- Made the Linux transparent host-drawing path authoritative by stopping default GTK background drawing after the transparent clear pass and keeping both host windows and fixed containers non-focusable.
- Moved the macOS screenshot path off the old Windows-only gate and onto backend-local host-window snapshot capture, so the macOS backend no longer falls back to an unsupported screenshot error for screenshot, selected-area capture, and inspect-element image attachments.
- Reused the screenshot attachment downscaling path on Linux too, so Linux screenshot and inspect-element image attachments now stay aligned with the Windows/macOS agent image size limits.
- Linux now explicitly hands focus back from the native webview to GPUI controls when the preview toolbar/editor takes focus, instead of leaving that transition implicit in the host window state.
- Linux X11 and Wayland preview hosts no longer mark their GTK toplevels as permanently non-focusable, so the native page can acquire keyboard focus from user interaction without stealing focus on map.
- macOS and Linux native preview hosts now explicitly return focus to GPUI before hiding on tab/window deactivation, instead of relying on implicit focus changes during native host teardown.
- macOS now uses a dedicated backend host `NSWindow` subclass that can become key/main, so native page focus can participate more reliably in the AppKit window chain instead of depending on plain borderless-window defaults.
- macOS and Linux now track URL-editor focus explicitly and only refocus the native page when the preview becomes active without the GPUI URL editor already owning focus, instead of relying on incidental activation ordering.
- Tightened the macOS/Linux native-page refocus rule again so the page is only refocused when the preview item itself owns GPUI focus, which prevents native-page focus grabs while other GPUI overlays or controls are active.
- macOS and Linux native preview creation now retries on transient parent-handle readiness failures instead of latching those timing conditions as permanent mount errors.
- Fixed the non-Windows render path so macOS/Linux no longer force native preview hosts visible on every repaint, which previously risked overriding the explicit hide/deactivation lifecycle.
- Fixed the non-Windows passthrough-hole lifecycle so macOS/Linux only register preview mouse passthrough while the native host is actually active, instead of leaving a stale input hole behind when the host is hidden.
- Tightened the macOS/Linux transient mount retry path so one temporary native-parent readiness failure schedules only a single retry instead of allowing repeated renders to queue duplicate remount attempts.
- Moved macOS and Linux native preview visibility back onto explicit activation state instead of repaint-time host toggles, added backend-local visible-state tracking, and made non-Windows host show/hide idempotent.
- Made the macOS host reassert its native parent-window ordering and collection state during bounds updates, and made the Linux Wayland host explicitly lower itself on show just like the X11 host.
- Fixed a Linux backend regression where the transient native-preview retry path still referenced `cx` through an `_cx` parameter name.
- Reworked Linux host layout so X11 continues to use preview-sized host windows while Wayland now uses a parent-sized host window with the embedded webview positioned inside it, which is a better match for compositor-safe underlay placement.
- Added Linux local browser extension discovery for Chromium- and Firefox-family browsers, including common Flatpak profile locations, so the Linux backend no longer leaves extension detection as a Windows-only feature.
- Expanded Linux Chromium-family extension discovery to scan real multi-profile browser roots instead of only a single `Default` profile directory.
- Linux X11 and Wayland host screenshot capture now crops to the actual embedded webview rectangle, so Linux screenshots, selected-area captures, and inspect-element attachments operate on the preview surface instead of the full host window.
- Stopped the Wayland host path from pretending it can control global host-window placement with `move_`, so the Wayland backend now relies on exported-parent attachment plus parent-sized host resizing and internal webview positioning instead.
- Fixed Linux host initialization so X11 and Wayland hosts now start with the correct embedded webview bounds from the first mount instead of temporarily seeding host state from the wrong window-bounds shape.
- Linux X11 and Wayland native-host creation now waits for a real preview layout before mounting, instead of mapping a dummy fallback-sized host before the preview rect exists.
- Linux X11 and Wayland preview hosts now track their native parent attachment as explicit backend state, retarget in place when possible, and force a remount when the host kind or native parent relationship changes underneath the preview.
- Fixed a Linux backend regression where the preview canvas referenced `native_mount_requested` without capturing that Linux mount state into the closure.
- Linux Wayland now tears down and refreshes the native preview host when the exported parent handle temporarily disappears, instead of keeping stale host state attached to an invalid parent relationship.
- macOS preview hosts now track their GPUI AppKit parent/window identity as explicit backend state, retarget that host relationship when it changes, and remount cleanly if the native parent chain becomes temporarily unavailable.
- macOS native preview mounting now waits for a real preview layout before creating the host window/webview, instead of mapping a fallback-sized host first and correcting it later.
- macOS host retargeting now reapplies native host bounds and visibility when the AppKit parent chain changes, so the preview cannot keep a stale screen-space frame after an in-place parent retarget.
- Linux X11 host retargeting now preserves underlay stacking and reapplies native bounds/visibility after parent-window changes, so X11 preview hosts cannot keep stale placement or drift above the editor after retarget or resize churn.
- Linux Wayland hosts now track their exported parent handle as explicit backend state and re-lower after layout churn, visibility restores, and parent-handle retargeting, so the Wayland underlay host stays attached and stacked correctly through compositor-side parent changes.
- Restored Windows to the original `crates/web_preview` runtime wiring from `windows-webpreview`, while leaving macOS/Linux on separate backend crates, so the smooth proven Windows path is no longer routed through the split facade.
- Began merging the old `crates/liquid_glass` standalone demo into Zed proper by moving Liquid Glass assets to the root asset pipeline, replacing imgui controls with native GPUI controls, and wiring the renderer through GPUI's shared primitive/back-end system.
- Restored the Liquid Glass demo to the correct single-element model: a static preview image underlay with one glass shader pass above it, instead of treating the glass as a dragged miniature image panel.
- Changed the floating Liquid Glass overlay to use a transparent source surface plus procedural shader highlights, so the moving glass element is no longer a sampled copy of the selected preview image.
- Changed the Liquid Glass overlay renderer to capture the already-rendered editor frame in WGPU, DirectX, and Metal before each glass segment, so the moving glass now uses the real backdrop instead of a dummy transparent fallback.

### Fixed
- Restored the whole-editor Liquid Glass overlay to the original tint/alpha/glow shader stack while keeping the live editor backdrop as its source, so the floating lens no longer uses the drifted gray-white look.
- Cleared stale Windows web preview passthrough capture state on capture loss and host deactivation/hide without force-resetting the normal webview keyboard-focus path, so long-lived sessions stop latching dead input while normal interactions keep working.
- Web preview toolbar action icons now stay muted at rest and only switch to the primary accent during hover and press states.
- Web preview URL editing no longer forces focus away after navigation and avoids overwriting in-progress input while the page reports URL updates.
- Web preview screenshots now copy the captured image to the clipboard and insert image plus URL attachments into the AI composer.
- Dragging a Web Preview tab into the agent panel now inserts the current page URL as a chat attachment.
- Web preview element selection now sends selected-element context into the AI composer with DOM details and a captured element snapshot on supported platforms.
- Removed the persistent top-bar notice above the web preview and made the extensions action available even before the first extension scan.
- GPUI workspace toasts now render from the top-right instead of the bottom edge, and web preview actions use the normal toast path again.
- Web preview screenshot and element-selector actions now guard against action-path panics and shrink oversized AI image attachments before insertion.
- Windows web preview now mounts into a separate underlay host window, and the GPUI preview body stops painting an opaque editor background over the browser region.
- Windows no longer forces `GPUI_DISABLE_DIRECT_COMPOSITION=1` at startup, which is required for alpha-based hole-punch composition work.
- Workspace items can now opt into a transparent workspace background, and web preview uses that path so the underlay browser surface is no longer blocked by the workspace root fill.
- Windows web preview wheel input now routes through the focused/root native webview window instead of the deepest child hit target, which restores mouse-wheel scrolling in the inline preview.
- GPUI now yields cursor ownership while the pointer is over the web preview passthrough body so the native webview cursor no longer flickers against GPUI over video regions.
- Windows web preview mouse-move relay now also sends native `WM_SETCURSOR`, so browser hover and cursor updates can work through the GPUI overlay path.
- Windows web preview body no longer registers GPUI mouse listeners, so the native underlay hole can keep ownership of hover and wheel input instead of fighting the relay path.
- Removed copied `windows_visual_webview` code from the macOS and Linux backend crates so future non-Windows work no longer drags Windows-only files and dependencies with it.
- Windows web preview now forwards hover and wheel through the stable root webview HWND from the Windows message pump, instead of chasing transient Chromium child windows that caused laggy hover and dead wheel input.
- Windows composition-hosted web preview keyboard now uses an isolated WebView2 DevTools input bridge for page typing, while leaving the working hover, wheel, click, and z-index paths untouched.
- Completed the missing Windows HLSL and macOS Metal backend hooks for the existing `LiquidGlass` primitive so the integrated effect no longer depends on the old standalone wgpu/imgui app path.

---

For upstream Zed changes, see the [official releases page](https://github.com/zed-industries/zed/releases).
