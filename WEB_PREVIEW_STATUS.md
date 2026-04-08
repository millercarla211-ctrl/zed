# Web Preview Status

**Date:** April 9, 2026  
**Repository:** `F:\dx`  
**Active branch:** `dev`  
**Current priority:** expand web preview support to macOS and Linux without changing the completed Windows path

## Executive Summary

The Windows web preview is now the stable reference implementation in this fork. It uses the native Windows browser stack and a GPUI-aware embedding path that preserves the solved z-index behavior, mouse interaction, keyboard input, URL-bar focus handoff, and editor overlays.

That Windows path must now be treated as frozen. New work should happen only in macOS and Linux code paths unless a Windows regression is explicitly reported.

The current cross-platform picture is:

- **Windows:** complete and working in daily use
- **macOS:** native host exists, but still needs platform-specific hardening and separation from the old inline implementation
- **Linux X11:** not previously wired; implementation work is now starting
- **Linux Wayland:** still needs a dedicated compositor-safe host path and cannot be treated the same as X11

## Support Matrix

| Platform | Status | Current Host Strategy | Notes |
|---|---|---|---|
| Windows | Complete | Native WebView2 + frozen Windows-specific embedding/input path | Do not modify unless fixing a confirmed Windows regression |
| macOS | Partial | Native `WKWebView` via `wry` child embedding | Needs platform-specific cleanup and long-term underlay-safe architecture |
| Linux X11 | In progress | `wry` child embedding is the practical first step | Can use a native child-host path; GTK event integration is required |
| Linux Wayland | Planned | Separate GTK / Wayland-safe host path | `build_as_child` is not the correct universal answer on Wayland |

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

### Linux

- The codebase previously stopped with a Linux-specific error instead of mounting a native preview
- X11 and Wayland need to be handled as separate backends
- Current work is focused on starting with Linux host wiring without touching Windows

## Current Technical Reality

### Windows Is Frozen

The working Windows path is complex and fragile enough that it must not be refactored casually. It is the known-good implementation and should remain the baseline while other platforms catch up.

### macOS Is Not the Same Problem as Windows

macOS already has a native `WKWebView` path, but it still needs platform-specific cleanup. The goal there is to keep native rendering while making the host integration more deliberate and maintainable.

### Linux Must Be Split

Linux support is not one implementation:

- **X11** can use a child-host embedding path as the first practical step
- **Wayland** needs its own safe host strategy because the generic child-window path is not the right long-term answer

Treating Linux as one generic target is the fastest way to end up with a broken implementation.

## Active Implementation Plan

1. Keep Windows untouched.
2. Preserve the existing macOS path while isolating non-Windows host logic from the Windows implementation.
3. Add Linux host wiring with an X11-first path.
4. Keep Wayland behind a separate backend decision instead of pretending X11 and Wayland are interchangeable.
5. Only after non-Windows hosts are stable should broader cross-platform cleanup happen.

## Files That Matter Most

- [web_preview_view.rs](/F:/dx/crates/web_preview/src/web_preview_view.rs)
- [web_preview.rs](/F:/dx/crates/web_preview/src/web_preview.rs)
- [window.rs](/F:/dx/crates/gpui_macos/src/window.rs)
- [window.rs](/F:/dx/crates/gpui_linux/src/linux/x11/window.rs)
- [window.rs](/F:/dx/crates/gpui_linux/src/linux/wayland/window.rs)
- [WINDOWS_WEB_PREVIEW_IMPLEMENTATION_REPORT.md](/F:/dx/WINDOWS_WEB_PREVIEW_IMPLEMENTATION_REPORT.md)

## Next Safe Step

The correct next step is to grow macOS and Linux support in isolated platform-specific code paths while keeping the Windows implementation frozen exactly as it is now.
