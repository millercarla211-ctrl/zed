# Project TODO

> Auto-managed by AI. Updated after every completed or failed task.
> Last updated: 2026-04-11

## In Progress
- [ ] Finish integrating Liquid Glass as a native GPUI-rendered workspace item with platform renderer support and native controls

## Pending
- [ ] Runtime-validate the completed macOS, Linux X11, and Linux Wayland web preview backends on real platform sessions when those environments are available
- [x] Sync with upstream Zed (first sync after setup)
- [x] Test low-memory build configuration from forge branch
- [x] Document any Windows-specific issues encountered

## Completed
- [x] ~~Move Web Preview navigation and actions into the main pane tab bar, remove the in-page toolbar, and make the active web tab switch between page title and URL editor~~ ✅ (completed: 2026-04-11)
- [x] ~~Restore the whole-editor Liquid Glass overlay to the original panel shader stack while keeping the live editor backdrop as its source~~ ✅ (completed: 2026-04-11)
- [x] ~~Clear stale Windows web preview passthrough capture state on capture loss and host deactivation/hide without breaking the normal webview focus path~~ (completed: 2026-04-09)
- [x] ~~Add a true live-backdrop GPUI renderer path for Liquid Glass so the effect can refract real editor content instead of using the old transparent fallback~~ (completed: 2026-04-10)
- [x] ~~Restore Windows web preview to the original monolithic `crates/web_preview` runtime wiring while keeping macOS/Linux on split backend crates~~ (completed: 2026-04-09)
- [x] ~~Make Linux Wayland hosts track exported-parent attachment as explicit backend state and reapply underlay stacking after layout, visibility, and parent-handle retargets~~ (completed: 2026-04-09)
- [x] ~~Make Linux X11 host retargeting reapply underlay stacking, bounds, and visibility so parent-window changes cannot leave stale placement or stale z-order~~ (completed: 2026-04-09)
- [x] ~~Make macOS host retargeting reapply native bounds and visibility so parent-chain changes cannot leave the preview in a stale screen position~~ (completed: 2026-04-09)
- [x] ~~Make the macOS native preview host retarget or remount when the AppKit parent/view chain changes~~ (completed: 2026-04-09)
- [x] ~~Make macOS wait for a real preview layout before mounting the native host window and webview~~ (completed: 2026-04-09)
- [x] ~~Make Linux Wayland tear down and refresh the native preview host when the exported parent handle temporarily disappears~~ (completed: 2026-04-09)
- [x] ~~Teach Linux web preview hosts to retarget or remount when their native X11 parent ID or Wayland exported parent handle changes~~ (completed: 2026-04-09)
- [x] ~~Fix the Linux preview canvas to capture native mount state explicitly instead of referencing an uncaptured field from the outer view~~ (completed: 2026-04-09)
- [x] ~~Make Linux X11 and Wayland wait for a real preview layout before mounting native hosts~~ ✅ (completed: 2026-04-09)
- [x] ~~Stop the Linux Wayland host path from pretending it can control global placement with `move_`; rely on exported-parent attachment plus parent-sized host resizing instead~~ ✅ (completed: 2026-04-09)
- [x] ~~Seed Linux X11 and Wayland host state with the correct embedded webview layout from the first mount~~ ✅ (completed: 2026-04-09)
- [x] ~~Expand Linux Chromium-family extension discovery to real multi-profile browser roots instead of only a single Default profile~~ ✅ (completed: 2026-04-09)
- [x] ~~Make Linux X11 and Wayland screenshots capture the actual embedded preview rectangle instead of the full native host window~~ ✅ (completed: 2026-04-09)
- [x] ~~Split Linux native host layout between X11 preview-sized hosts and Wayland parent-sized hosts so Wayland can position the webview inside a compositor-safe parent window~~ ✅ (completed: 2026-04-09)
- [x] ~~Add Linux local browser extension discovery parity for Chromium, Firefox, and common Flatpak browser profiles~~ ✅ (completed: 2026-04-09)
- [x] ~~Move macOS and Linux native host visibility fully back to activation state instead of repaint-time show/hide toggles~~ ✅ (completed: 2026-04-09)
- [x] ~~Guard macOS and Linux transient mount retries so repeated renders cannot queue duplicate remount attempts~~ ✅ (completed: 2026-04-09)
- [x] ~~Keep macOS and Linux passthrough holes aligned with native host visibility so hidden hosts do not leave stale input holes behind~~ ✅ (completed: 2026-04-09)
- [x] ~~Stop macOS and Linux render passes from forcing native preview hosts visible on every repaint, so activation/deactivation visibility stays authoritative~~ ✅ (completed: 2026-04-09)
- [x] ~~Retry macOS and Linux native preview creation when the parent native handle is temporarily unavailable instead of treating that timing issue as a permanent mount failure~~ ✅ (completed: 2026-04-09)
- [x] ~~Constrain macOS and Linux native-page refocus so it only happens when the preview item itself owns GPUI focus, not merely when the tab is active~~ ✅ (completed: 2026-04-09)
- [x] ~~Add explicit non-Windows page-focus intent so macOS and Linux only refocus the native page when the preview activates without the GPUI URL editor already owning focus~~ ✅ (completed: 2026-04-09)
- [x] ~~Move the macOS preview host onto a dedicated `NSWindow` subclass that can become key/main so native page focus is not relying on plain borderless-window defaults~~ ✅ (completed: 2026-04-09)
- [x] ~~Return focus from macOS and Linux native preview hosts back to GPUI before hiding those hosts on tab or window deactivation~~ ✅ (completed: 2026-04-09)
- [x] ~~Allow Linux X11 and Wayland preview host windows to take keyboard focus from user interaction without stealing focus on map~~ ✅ (completed: 2026-04-09)
- [x] ~~Make Linux web preview explicitly release native focus back to GPUI controls when the toolbar or editor takes focus~~ ✅ (completed: 2026-04-09)
- [x] ~~Reuse the screenshot attachment downscaling path on Linux so native Linux preview captures stay aligned with Windows and macOS agent image limits~~ ✅ (completed: 2026-04-09)
- [x] ~~Add native macOS host-window screenshot capture so the macOS backend no longer falls back to the old unsupported screenshot path~~ ✅ (completed: 2026-04-09)
- [x] ~~Make Linux transparent host drawing authoritative and keep Linux host widgets non-focusable~~ ✅ (completed: 2026-04-09)
- [x] ~~Make Linux host windows paint explicit transparent backgrounds instead of relying on toolkit defaults~~ ✅ (completed: 2026-04-09)
- [x] ~~Sync the macOS host window level and collection behavior from the GPUI parent window~~ ✅ (completed: 2026-04-09)
- [x] ~~Move Linux X11 and Wayland preview hosts off unmanaged GTK popup windows and onto managed undecorated toplevel hosts~~ ✅ (completed: 2026-04-09)
- [x] ~~Attach the Linux X11 preview host to the GPUI parent window via an explicit transient relationship and request RGBA visuals for Linux host windows~~ ✅ (completed: 2026-04-09)
- [x] ~~Keep the macOS backend host window explicitly ordered below the GPUI parent window when restoring visibility~~ ✅ (completed: 2026-04-09)
- [x] ~~Wire macOS and Linux native host windows to GPUI activation observers and item activity tracking so separate host windows hide/restore with the editor window~~ ✅ (completed: 2026-04-09)
- [x] ~~Wire macOS and Linux native host windows to GPUI window-bounds observers so host placement updates during window moves and resizes~~ ✅ (completed: 2026-04-09)
- [x] ~~Fix Linux host-window placement to use GPUI inner window bounds instead of raw local preview coordinates~~ ✅ (completed: 2026-04-09)
- [x] ~~Move Linux X11 onto a backend-local managed host path and stop Linux focus handoff from routing back into backend host windows~~ ✅ (completed: 2026-04-09)
- [x] ~~Fix non-Windows host-window bounds syncing so macOS and Linux Wayland update when the parent editor window moves~~ ✅ (completed: 2026-04-09)
- [x] ~~Move the macOS backend onto a backend-local host-window layer and hide non-Windows preview hosts on tab/workspace deactivation~~ ✅ (completed: 2026-04-09)
- [x] ~~Seed a dedicated Linux Wayland web preview host path using GTK/WebKitGTK plus GPUI-exported xdg-foreign parent handles~~ ✅ (completed: 2026-04-09)
- [x] ~~Register the macOS and Linux web preview body as a GPUI passthrough region so non-Windows backends can use the platform-native hole-punch hit-testing path~~ ✅ (completed: 2026-04-09)
- [x] ~~Move the macOS native `WKWebView` under GPUI's AppKit view so the backend no longer relies purely on default child-view ordering~~ ✅ (completed: 2026-04-09)
- [x] ~~Isolate the macOS and Linux web preview backends from copied Windows-only modules and add the Linux GTK runtime pump required by `wry` on X11~~ ✅ (completed: 2026-04-09)
- [x] ~~Split web preview into platform-specific backend crates while preserving the working Windows implementation~~ ✅ (completed: 2026-04-09)
- [x] ~~Document the frozen Windows web preview architecture and tidy auxiliary root web preview docs~~ ✅ (completed: 2026-04-09)
- [x] ~~Finish the Windows native web preview and freeze its working path~~ ✅ (completed: 2026-04-09)
- [x] ~~Finish web preview agent attachments, selector context, and toolbar interactions~~ ✅ (completed: 2026-04-07)
- [x] ~~Configure git for fork maintenance~~ ✅ (completed: 2026-04-07)
- [x] ~~Create dev branch for development work~~ ✅ (completed: 2026-04-07)
- [x] ~~Create forge branch for Windows optimizations~~ ✅ (completed: 2026-04-07)
- [x] ~~Add low-memory Cargo configuration~~ ✅ (completed: 2026-04-07)
- [x] ~~Create GIT.md workflow guide~~ ✅ (completed: 2026-04-07)
- [x] ~~Set up AI agent coordination system~~ ✅ (completed: 2026-04-07)

## Blocked / Failed
(none)
