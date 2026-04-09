# Web Preview Status

**Date:** April 9, 2026  
**Repository:** `F:\dx`  
**Active branch:** `dev`  
**Current priority:** runtime-validate the completed macOS, Linux X11, and Linux Wayland backends without changing the completed Windows path

## Executive Summary

The Windows web preview is now the stable reference implementation in this fork. It uses the native Windows browser stack and a GPUI-aware embedding path that preserves the solved z-index behavior, mouse interaction, keyboard input, URL-bar focus handoff, and editor overlays.

That Windows path is now isolated behind a separate backend crate and must be treated as frozen. New work should happen only in macOS and Linux code paths unless a Windows regression is explicitly reported.

The current cross-platform picture is:

- **Windows:** complete and working in daily use
- **macOS:** target-local backend is in place, the preview mounts into a dedicated backend-local host window under GPUI, inherits parent window level/collection behavior, host placement/visibility is now coordinated through both GPUI bounds and activation observers, screenshot capture now runs through backend-local host-window snapshotting instead of an unsupported fallback, the backend treats the AppKit parent/view identity as live host state so it can retarget or remount cleanly when the native parent chain changes, and retargeting now reapplies native bounds/visibility so parent swaps cannot leave the preview at a stale screen-space frame
- **Linux X11:** now mounts into its own backend-local managed GTK toplevel host window under the GPUI passthrough path, attaches that host to the GPUI parent through an explicit X11 transient relationship, derives host placement from GPUI inner window bounds instead of raw local coordinates, treats the X11 host window itself as the preview-sized underlay surface, and now reapplies underlay stacking plus native bounds/visibility when the X11 parent relationship changes
- **Linux Wayland:** now has its own managed GTK/WebKit host path tied to GPUI-exported parent handles, derives host placement from GPUI inner window bounds, requests RGBA-capable visuals, explicitly lowers the host on show like X11, uses a parent-sized host window with the native webview positioned inside it to better match Wayland compositor constraints, no longer assumes it can control global host placement via `move_`, waits for a real preview layout before mounting the native host, returns screenshot captures from the actual embedded preview rectangle instead of the full host surface, treats the exported-parent attachment as live backend state so the host can retarget or remount cleanly when the parent relationship changes, tears down and refreshes cleanly when that exported-parent handle temporarily disappears, and now re-lowers plus reapplies layout/visibility when the exported parent changes so the Wayland underlay host stays attached and stacked correctly through compositor-side parent churn

## Support Matrix

| Platform | Status | Current Host Strategy | Notes |
|---|---|---|---|
| Windows | Complete | `web_preview_windows` + native WebView2 + frozen Windows-specific embedding/input path | Do not modify unless fixing a confirmed Windows regression |
| macOS | Implementation complete | `web_preview_macos` + native `WKWebView` reparents into a backend-local AppKit host window under GPUI | Backend is now target-local, keeps its own browser-event notifier, reparents the webview into its own host window, restores GPUI focus through that host, registers the preview body as a GPUI passthrough region for native AppKit hit-test yielding, mirrors the GPUI parent window's level and collection behavior, captures screenshots through that host window instead of the old Windows-only path, now uses a dedicated host-window subclass so native page focus is not relying on plain borderless-window defaults, tracks URL-editor focus explicitly so native page refocus only happens when the preview reactivates without GPUI already owning the URL field, now tracks the AppKit host target as explicit backend state so it can retarget/remount cleanly when the parent window relationship changes, reapplies native bounds and visibility when that retarget happens, and now waits for a real preview layout before mounting the native host |
| Linux X11 | Implementation complete | `web_preview_linux` + backend-local GTK toplevel host window + WebKitGTK | Backend is isolated, initializes/pumps GTK correctly, registers the preview body as a GPUI passthrough region, no longer depends on the older direct X11 child mount path, now places the host from GPUI inner-window global bounds, explicitly transient-parents that managed host to the GPUI X11 window, keeps the host/window widget tree transparently painted while allowing the GTK host to take keyboard focus from user interaction, treats the X11 host window itself as the preview-sized underlay surface, returns screenshot captures from the embedded preview rectangle, routes screenshot attachments through the same size-normalization path as the other desktop backends, discovers Chromium/Firefox extensions from native Linux profile directories and multi-profile Chromium roots, tracks URL-editor focus explicitly so native page refocus only happens when the preview reactivates without GPUI already owning the URL field, and now reapplies underlay stacking plus native bounds/visibility when the X11 parent relationship changes |
| Linux Wayland | Implementation complete | Dedicated GTK/WebKit Wayland host path attached through GPUI-exported xdg-foreign parent handles | GPUI now exports Wayland parent handles, the Linux backend no longer aborts immediately on Wayland, the backend host now uses a managed undecorated GTK toplevel instead of an unmanaged popup surface, keeps the host/window widget tree transparently painted while allowing the GTK host to take keyboard focus from user interaction, now uses a parent-sized host window with the native webview positioned inside it instead of trying to place the host itself at the preview rect, no longer relies on `move_` for compositor-controlled placement, initializes host state with the correct embedded webview bounds from the first mount, waits for a real preview layout before mounting the native host, returns screenshot captures from the embedded preview rectangle, routes screenshot attachments through the same size-normalization path as the other desktop backends, discovers Chromium/Firefox extensions from native Linux profile directories and multi-profile Chromium roots, now tracks URL-editor focus explicitly so native page refocus only happens when the preview reactivates without GPUI already owning the URL field, retargets or remounts the native host when the exported Wayland parent relationship changes, tears down and refreshes cleanly when that parent handle temporarily disappears, and now re-lowers plus reapplies layout/visibility when the exported parent changes |

## Current Percentages

- **Windows:** `100/100`
- **macOS:** `100/100` implementation, runtime validation pending
- **Linux X11:** `100/100` implementation, runtime validation pending
- **Linux Wayland:** `100/100` implementation, runtime validation pending
- **Overall desktop web preview:** `100/100` implementation, runtime validation pending

## What Is Working Now

### Windows

- Native page rendering inside the editor
- GPUI controls above the preview
- Working mouse click, right click, hover, wheel, and keyboard input
- Working URL-bar focus handoff
- Stable toolbar and page coexistence

The detailed Windows implementation write-up lives in [WINDOWS_WEB_PREVIEW_IMPLEMENTATION_REPORT.md](/F:/dx/WINDOWS_WEB_PREVIEW_IMPLEMENTATION_REPORT.md).

### macOS

- The codebase already has a macOS-native preview creation path in [web_preview_view.rs](/F:/dx/crates/web_preview/src/web_preview_view.rs)
- The current path uses `wry` + `WKWebView` child embedding
- Bounds syncing and toolbar integration already exist
- The macOS backend crate now owns its own browser-event notifier instead of relying on copied Windows backend structure
- First-click delivery is now explicitly enabled on the native `WKWebView` path
- The macOS backend now creates a dedicated backend-local host window and reparents the native `WKWebView` into it instead of relying purely on in-window child-view ordering
- The macOS preview body now registers a real GPUI passthrough region, which is what lets AppKit hit-testing yield back to the underlay browser surface instead of only drawing it
- macOS focus restoration now routes back through that host window so GPUI controls can reclaim first responder state without relying on `WKWebView`'s parent view still being the GPUI view
- The macOS host path now reapplies bounds continuously, which keeps the native host window aligned even when the editor window moves without changing the local preview rect
- The macOS host path is now also wired to GPUI window-bounds observers, so host placement updates on window movement and resize events even if the preview itself does not rerender in that moment
- The macOS host path now tracks GPUI window activation and item activity too, so the separate host window hides and restores with the editor window instead of depending only on tab-level visibility changes
- The macOS host visibility restore path now explicitly reorders the host below the GPUI parent window instead of using a generic front-order call that could drift the host z-order
- The macOS host now also mirrors the GPUI parent window's native level and collection behavior so host ordering stays aligned with the AppKit window stack
- The macOS backend now captures screenshots, selected-area crops, and inspect-element image attachments through backend-local host-window snapshots instead of falling back to an unsupported-platform error
- The macOS backend now explicitly restores GPUI focus before hiding its backend-local host window, which removes another implicit focus transition from the deactivation path
- The macOS backend host now uses a dedicated `NSWindow` subclass that can become key/main, which gives the native page a more correct AppKit focus path than a plain borderless host window
- The macOS backend now tracks the URL editor's focus handle explicitly, so native page refocus on activation is skipped when GPUI already owns the URL field
- The macOS backend now also requires the preview item itself to own GPUI focus before refocusing the native page, which prevents page refocus while other GPUI overlays or controls are active
- The macOS backend now retries native host creation when the GPUI AppKit view is not yet attached to its `NSWindow`, instead of treating that startup timing condition as a permanent mount failure
- The macOS render path no longer forces the native host window visible on every repaint, so the explicit activation/deactivation lifecycle remains authoritative
- The macOS passthrough hole now follows the same active/visible lifecycle as the native host window, instead of remaining registered while the host is hidden
- The macOS transient remount path now keeps only one retry in flight, so repeated renders cannot queue duplicate retries while the native parent handle is still warming up
- The macOS host now tracks native host visibility explicitly, so repeated preview rerenders no longer re-show the host window or `WKWebView` unless activation state actually changed
- The macOS backend now waits for a real preview layout before mounting its native host window and webview, instead of creating a fallback-sized host before the preview rect exists
- The macOS backend now treats the GPUI AppKit view/window identity as explicit host state, so existing hosts can retarget in place or remount cleanly when the native parent chain changes underneath the preview
- The macOS backend now reapplies native host bounds and visibility when that AppKit host retarget happens, so a parent-chain change cannot leave the preview at stale screen coordinates if the local preview rect itself did not change

### Linux

- The codebase previously stopped with a Linux-specific error instead of mounting a native preview
- X11 and Wayland need to be handled as separate backends
- Current work is focused on starting with Linux host wiring without touching Windows
- X11 now has GTK runtime initialization and GTK main-loop pumping, which `wry` requires for WebKitGTK child webviews
- Linux backends now register the preview body as a GPUI passthrough region, so the existing X11/Wayland platform-window passthrough machinery can participate once a native Linux host is mounted
- The Linux backend now branches explicitly between X11 and Wayland at mount time instead of treating all Linux windows as one host path
- Linux X11 now uses a backend-local managed host window instead of the remaining direct child-webview mount path, which brings its host architecture closer to Wayland and keeps Linux backend behavior more consistent
- Linux host windows now derive their absolute placement from GPUI's inner window bounds, which is the coordinate space the preview body is actually rendered in
- Linux X11 and Wayland host windows now use managed undecorated GTK toplevels instead of unmanaged GTK popup windows, which gives the window manager and compositor real host surfaces to track under the editor
- Linux host windows now request RGBA-capable visuals before realization so their host surfaces can participate in transparent underlay composition instead of assuming the default visual is alpha-capable
- Linux host windows now explicitly clear their own backgrounds with transparent Cairo painting instead of relying on toolkit default drawing behavior
- Linux X11 now attaches its backend host window to the GPUI parent window through an explicit transient relationship instead of leaving the host unmanaged relative to the editor
- Linux screenshot, selected-area capture, and inspect-element image attachments now pass through the same AI-image size-normalization path as the other desktop backends
- Linux now explicitly tells the native webview to return focus to GPUI when the toolbar or editor takes focus, instead of leaving that handoff implicit in the host window state
- Linux X11 and Wayland hosts now allow their GTK toplevels to take focus from real user interaction while still avoiding focus-on-map, which is the required balance for native page keyboard input without host-window focus thrash
- Linux host hide/deactivation now explicitly returns focus from the native webview back to GPUI before hiding the GTK host windows, instead of relying on whatever focus side effects the window manager produces
- Linux now tracks the URL editor's focus handle explicitly, so native page refocus on activation is skipped when GPUI already owns the URL field
- Linux now also requires the preview item itself to own GPUI focus before refocusing the native page, which prevents page refocus while other GPUI overlays or controls are active
- The Linux backend now retries native host creation when the Wayland exported parent handle is not ready yet, instead of treating that startup timing condition as a permanent mount failure
- The Linux render path no longer forces the native host windows visible on every repaint, so the explicit activation/deactivation lifecycle remains authoritative
- The Linux passthrough hole now follows the same active/visible lifecycle as the native host windows, instead of remaining registered while the hosts are hidden
- The Linux transient remount path now keeps only one retry in flight, so repeated renders cannot queue duplicate retries while the native parent handle is still warming up
- Linux X11 and Wayland now both track native host visibility explicitly, so repeated preview rerenders no longer re-show the GTK host window or WebKit view unless activation state actually changed
- Linux X11 now keeps a preview-sized host window while Linux Wayland now keeps a parent-sized host window and positions the embedded webview inside it, which is a more correct split between X11 window management and Wayland compositor constraints
- Linux X11 and Wayland host screenshots now crop to the actual embedded preview rectangle, so screenshot capture and inspect-element image attachments no longer inherit full host-window dimensions after the host-layout split
- Linux X11 now reapplies host lowering plus native bounds/visibility after transient-parent retargeting, so parent-window churn cannot leave the X11 underlay host stale, misplaced, or restacked above the editor
- The Linux Wayland host now explicitly lowers itself when shown, matching the X11 host's ordering behavior instead of relying only on `keep_below`
- The Linux Wayland host no longer tries to control global placement with `move_`; it now relies on transient exported-parent attachment plus parent-sized host resizing and internal webview positioning, which is a better match for Wayland compositor rules
- GPUI Wayland now exports xdg-foreign parent handles from its toplevel surfaces
- `web_preview_linux` now has a dedicated Wayland GTK/WebKit host path that attaches to that exported parent instead of failing immediately
- The Wayland GTK host now avoids normal window-focus acquisition so it can behave more like an underlay surface instead of a regular foreground popup surface
- The Wayland host path now forces dynamic bounds resynchronization instead of assuming unchanged local preview bounds mean unchanged host placement
- Both Linux host paths now also listen to GPUI window-bounds changes, so the native host windows can keep tracking the editor even when movement happens without a preview-content-triggered draw pass
- Both Linux host paths now also track GPUI window activation plus item activity, so the separate host windows hide and restore with the editor window instead of relying only on tab deactivation
- Linux now discovers Chromium- and Firefox-family browser extensions from native Linux profile directories, including common Flatpak locations, instead of leaving extension detection as a Windows-only feature
- Linux Chromium-family extension discovery now scans profile roots instead of a single `Default` directory, so Linux browser-extension support follows real multi-profile Chromium layouts too
- Linux host creation now seeds X11 and Wayland host state with the correct embedded webview layout from the first mount, instead of temporarily borrowing host-window bounds as webview bounds until the first resync
- Linux now treats X11 parent IDs and Wayland exported-parent handles as explicit preview-host state, so existing Linux hosts can retarget in place when the native parent stays compatible and force a clean remount when the parent relationship changes in a way the current host can no longer safely represent
- The Linux render path now captures its native-mount state explicitly inside the preview canvas closure instead of implicitly reading an uncaptured field from the outer view state
- Linux Wayland now treats temporary loss of the exported parent handle as a remount condition, so the backend does not keep stale native host state attached to an invalid parent relationship
- Wayland implementation is now complete in code, but it still needs compositor validation and final layering verification on real Wayland sessions before it can be treated as runtime-proven

## Current Technical Reality

### Windows Is Frozen

The working Windows path is complex and fragile enough that it must not be refactored casually. It is the known-good implementation and should remain the baseline while other platforms catch up. The backend split now reinforces that rule at the crate boundary.

### macOS Is Not the Same Problem as Windows

macOS already has a native `WKWebView` path, and that path now lives in its own backend crate. The current goal there is to keep native rendering while moving mounting, sizing, visibility, and focus restoration into a backend-local host-window layer instead of relying on fragile child-view ordering inside the GPUI window.

### Linux Must Be Split

Linux support is not one implementation:

- **X11** now has an isolated backend path that can use child-host embedding as the first practical step, plus the GTK runtime pumping that `wry` expects
- **X11** now has its own backend-local managed host path, which is a better match for the underlay architecture than the older direct child-webview mount
- **Wayland** now has its own parent-sized backend host strategy because the generic child-window path is not the right long-term answer there, and `wry` does not treat child embedding as the same solution on Wayland

Treating Linux as one generic target is the fastest way to end up with a broken implementation.

### Non-Windows Visibility Is Now Lifecycle-Driven

macOS and Linux no longer rely on the preview canvas to keep re-showing native hosts during every repaint. Those backends now track native-host visibility explicitly and only show or hide the native host when activation state changes. Bounds updates still happen during layout, but visibility itself is now activation-driven rather than paint-driven.

## Active Implementation Plan

1. Keep Windows untouched.
2. Keep the completed macOS, Linux X11, and Linux Wayland paths stable in their own backend crates.
3. Runtime-validate the non-Windows backends on real target sessions.
4. Only after non-Windows hosts are stable should broader cross-platform cleanup happen.

## Files That Matter Most

- [web_preview_view.rs](/F:/dx/crates/web_preview/src/web_preview_view.rs)
- [web_preview.rs](/F:/dx/crates/web_preview/src/web_preview.rs)
- [lib.rs](/F:/dx/crates/web_preview_windows/src/lib.rs)
- [web_preview_view.rs](/F:/dx/crates/web_preview_windows/src/web_preview_view.rs)
- [lib.rs](/F:/dx/crates/web_preview_macos/src/lib.rs)
- [macos_host.rs](/F:/dx/crates/web_preview_macos/src/macos_host.rs)
- [web_preview_view.rs](/F:/dx/crates/web_preview_macos/src/web_preview_view.rs)
- [lib.rs](/F:/dx/crates/web_preview_linux/src/lib.rs)
- [x11_host.rs](/F:/dx/crates/web_preview_linux/src/x11_host.rs)
- [wayland_host.rs](/F:/dx/crates/web_preview_linux/src/wayland_host.rs)
- [web_preview_view.rs](/F:/dx/crates/web_preview_linux/src/web_preview_view.rs)
- [window.rs](/F:/dx/crates/gpui_macos/src/window.rs)
- [window.rs](/F:/dx/crates/gpui_linux/src/linux/x11/window.rs)
- [window.rs](/F:/dx/crates/gpui_linux/src/linux/wayland/window.rs)
- [WINDOWS_WEB_PREVIEW_IMPLEMENTATION_REPORT.md](/F:/dx/WINDOWS_WEB_PREVIEW_IMPLEMENTATION_REPORT.md)

## Next Safe Step

The correct next step is to runtime-validate macOS, Linux X11, and Linux Wayland on real platform sessions while keeping the completed Windows implementation frozen.
