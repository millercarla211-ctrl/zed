Can you solve this problem - please study this codebase and do a web search so the other mouse, wheel and other inteactions works and also keyboard supports works too!!!

```markdown
# Web Preview Status Report

Date: 2026-04-08  
Workspace: `F:\dx`  
Branch context: active development worktree on the Zed fork

---

## Executive Summary

This repository no longer uses the original simple child-webview approach on Windows for inline web preview.

The Windows implementation was moved toward a composition-hosted WebView2 path so GPUI can render above web content instead of being trapped behind the browser surface. That architectural shift is the core reason the z-index or airspace problem became solvable at all.

The work is **partially successful**:

- Windows can render the web preview inline through a GPUI-controlled transparent region.
- GPUI UI can be rendered above the web preview.
- The previous Windows child-webview airspace problem is no longer the only behavior available.
- Mouse interaction plumbing has been substantially implemented for the Windows composition-hosted path.

The work is **not complete**:

- Windows keyboard input inside the actual page is still the remaining unstable/unfinished interaction area.
- macOS and Linux do not yet have equivalent production-ready underlay/composition implementations.
- Several historical iterations were tried, some of which temporarily fixed one interaction while regressing another.

This document captures the current architecture, the implementation history, what changed in code, what works, what remains broken, and what should happen next.

---

## Original Problem

The initial web preview implementation used a standard native child webview model.

That model was functional for displaying pages, but it had the classic airspace problem:

- Native browser content floated above GPUI UI.
- GPUI overlays such as menus, tooltips, and similar elements could appear behind the webview.
- The editor could not reliably treat the web preview as a normal composited surface inside the application.

In practical terms:

- Showing a webpage was easy.
- Making GPUI reliably appear above it was not.

---

## Goal

The intended end state is:

1. Render the web preview inline inside the editor.
2. Keep GPUI visually above the web content.
3. Preserve native browser performance.
4. Preserve native-feeling input behavior.
5. Avoid the old “browser floats on top of the app” failure mode.

For Windows specifically, the most credible path became:

- WebView2 composition or visual hosting
- DirectComposition integration
- GPUI transparency over the preview body
- explicit pointer routing only where visual hosting requires it

---

## Current Implementation Snapshot

### Windows

Windows is the only platform that currently has a serious custom inline-hosting implementation.

The active Windows work centers around:

- [`F:\dx\crates\web_preview\src\windows_visual_webview.rs`](F:\dx\crates\web_preview\src\windows_visual_webview.rs)
- [`F:\dx\crates\web_preview\src\web_preview_view.rs`](F:\dx\crates\web_preview\src\web_preview_view.rs)
- [`F:\dx\crates\gpui_windows\src\window.rs`](F:\dx\crates\gpui_windows\src\window.rs)
- [`F:\dx\crates\gpui_windows\src\events.rs`](F:\dx\crates\gpui_windows\src\events.rs)
- [`F:\dx\crates\gpui_windows\src\platform.rs`](F:\dx\crates\gpui_windows\src\platform.rs)
- [`F:\dx\crates\gpui_windows\src\gpui_windows.rs`](F:\dx\crates\gpui_windows\src\gpui_windows.rs)

The Windows implementation no longer depends only on a simple WRY child HWND for inline preview.

Instead, the code now contains:

- a dedicated `WindowsVisualWebView` wrapper around composition-hosted WebView2
- WebView2 composition controller creation
- DirectComposition visual integration under GPUI
- a registry that tracks the active preview region and associated WebView2 controller
- Windows message-layer mouse forwarding to the composition-hosted preview
- transparent GPUI rendering over the preview region

### macOS

macOS still remains on the native webview embedding path and does not yet have a finished equivalent of the Windows custom composition/underlay pipeline.

The relevant code still lives inside:

- [`F:\dx\crates\web_preview\src\web_preview_view.rs`](F:\dx\crates\web_preview\src\web_preview_view.rs)

There is not yet a platform-complete macOS stacking/input solution at the same maturity level as the Windows work.

### Linux

Linux does not yet have a production-ready inline underlay/composition implementation for both X11 and Wayland.

This is still an explicit gap.

---

## Architecture Changes Implemented So Far

### 1. Windows composition-hosted WebView2 layer

The Windows web preview backend now creates a composition-hosted WebView2 controller through `webview2-com`.

Key file:

- [`F:\dx\crates\web_preview\src\windows_visual_webview.rs`](F:\dx\crates\web_preview\src\windows_visual_webview.rs)

Key responsibilities added there:

- create the WebView2 environment
- create the composition controller
- attach the browser output to a DirectComposition visual
- set visibility and bounds
- register cursor and focus callbacks
- keep the preview mounted and synchronized with GPUI bounds

### 2. GPUI Windows-side passthrough registry

The Windows platform code now tracks the active composition-hosted preview target.

Key file:

- [`F:\dx\crates\gpui_windows\src\window.rs`](F:\dx\crates\gpui_windows\src\window.rs)

Capabilities added there include:

- registering the active WebView2 composition controller
- storing preview bounds
- storing the current native cursor
- tracking whether the preview currently owns keyboard focus
- exposing helper functions used by the Windows event layer

### 3. DirectComposition visual placement under GPUI

The Windows renderer and platform plumbing were extended so a dedicated webview visual can live below the GPUI visual tree while remaining in the same composed scene.

The practical outcome is:

- GPUI can stay visually above the browser
- the preview no longer has to win the z-order battle by default

### 4. Web preview body transparency and passthrough region

The inline preview body in GPUI now marks the preview region as mouse-passthrough in the correct place during layout and paint synchronization.

Key file:

- [`F:\dx\crates\web_preview\src\web_preview_view.rs`](F:\dx\crates\web_preview\src\web_preview_view.rs)

That file now:

- stores preview host bounds
- keeps the native preview synced with GPUI layout
- sets the GPUI window background appearance to transparent for preview rendering
- inserts a mouse passthrough region aligned to the preview body on Windows

### 5. Windows mouse routing through the message layer

The Windows event loop now forwards browser mouse input to the composition-hosted WebView2 controller.

Key file:

- [`F:\dx\crates\gpui_windows\src\events.rs`](F:\dx\crates\gpui_windows\src\events.rs)

This includes:

- move
- leave
- button down
- button up
- wheel
- cursor updates
- click focus reassertion

That message-layer work is the main reason pointer interaction became possible in the custom-hosted path.

### 6. Accelerator translation changes

The Windows platform message pump now knows when a focused web preview should bypass GPUI’s usual keyboard accelerator path.

Key file:

- [`F:\dx\crates\gpui_windows\src\platform.rs`](F:\dx\crates\gpui_windows\src\platform.rs)

This was necessary because otherwise GPUI would eagerly consume key events meant for the page.

---

## Files Currently Changed in the Worktree

At the time of writing, `git status --short` reports these modified files:

- `Cargo.lock`
- [`F:\dx\crates\gpui_windows\Cargo.toml`](F:\dx\crates\gpui_windows\Cargo.toml)
- [`F:\dx\crates\gpui_windows\src\events.rs`](F:\dx\crates\gpui_windows\src\events.rs)
- [`F:\dx\crates\gpui_windows\src\gpui_windows.rs`](F:\dx\crates\gpui_windows\src\gpui_windows.rs)
- [`F:\dx\crates\gpui_windows\src\platform.rs`](F:\dx\crates\gpui_windows\src\platform.rs)
- [`F:\dx\crates\gpui_windows\src\window.rs`](F:\dx\crates\gpui_windows\src\window.rs)
- [`F:\dx\crates\web_preview\src\windows_visual_webview.rs`](F:\dx\crates\web_preview\src\windows_visual_webview.rs)

This reflects that the majority of the substantive custom work so far is concentrated in the Windows stack.

---

## What Is Working

The following items are implemented in code and have been exercised to varying degrees during this effort:

- Windows web preview mounts through a custom composition-hosted backend.
- The preview body participates in GPUI layout and resizes with the pane.
- GPUI can render above the preview region.
- Mouse click routing has been implemented through the Windows message layer.
- Hover and cursor ownership logic were implemented on the Windows side.
- Mouse wheel routing was implemented on the Windows side.
- URL toolbar, toolbar actions, screenshots, selector flow, and AI-related preview tooling remain part of the web preview feature set.

The strongest concrete success of the overall effort is this:

**Windows no longer has to stay locked to the original “webview always floats above GPUI” behavior.**

That was the architectural barrier that blocked the project early on.

---

## What Is Not Finished

### 1. Keyboard input inside the actual page

This remains the main unresolved Windows interaction issue.

The specific problem is:

- top-level editor input and toolbar input can still function
- but page-level text entry inside the inline browser remains unreliable or non-functional
- prior attempts to solve it sometimes regressed hover, wheel, or other pointer behavior

This means the remaining work is no longer about rendering the page.

It is specifically about:

- keyboard ownership
- focus state stability
- routing or preserving the correct key path for the composition-hosted webview

### 2. macOS parity

macOS is not yet on the same professional inline-hosting path as Windows.

### 3. Linux parity

Linux is not yet implemented at the required X11 and Wayland level.

### 4. Cross-platform polish

There is not yet a single finished cross-platform host abstraction that delivers the exact same behavior on:

- Windows
- macOS
- X11
- Wayland

---

## Implementation History Summary

The work did not proceed in one straight line. It went through several phases.

### Phase A: original native embedding

The initial implementation relied on the simpler native embedding approach.

Strengths:

- fast to display content
- native rendering quality
- straightforward integration

Weaknesses:

- GPUI layering problem remained
- native browser surface dominated z-order

### Phase B: underlay and transparency experiments

The next phase focused on making the page visible beneath GPUI and allowing GPUI to own the visible top layer.

This involved:

- transparent background handling
- workspace background changes
- preview body passthrough regions
- Windows-specific composition and window changes

This phase proved that the old airspace limitation could be attacked successfully.

### Phase C: input-routing work

Once composition and visibility were working, the next large problem was input.

That phase included repeated work on:

- click routing
- hover routing
- wheel routing
- cursor ownership
- keyboard focus

The most stable results so far came from keeping mouse handling inside the Windows message layer and not mixing too many overlapping strategies at once.

### Phase D: keyboard isolation attempts

The latest work intentionally tried to isolate keyboard fixes from the working pointer path.

The key lesson was:

**Keyboard work must stay isolated from the working pointer stack.**

Whenever the keyboard fix expanded too far into generalized input plumbing, it risked regressing:

- hover
- wheel
- cursor ownership
- or preview interactivity itself

---

## Brutal Assessment of Current Status

### Overall feature completion

Approximate status:

- Web preview feature as a whole: `70/100`
- Windows inline custom-hosting architecture: `60/100`
- Windows input completeness: `45/100`
- Windows z-index solution: `70/100`
- Windows keyboard input inside page: `20/100`
- macOS inline parity: `15/100`
- Linux inline parity: `5/100`
- professional all-platform completion: `20/100`

These numbers are directional, not contractual, but they reflect the current reality:

- The browser is no longer just a proof of concept.
- Windows has real architecture work behind it now.
- The last stubborn issue is keyboard behavior inside the page.
- Cross-platform completion is still far away.

---

## Why Keyboard Has Been Harder Than Mouse

Mouse input in visual hosting is explicitly expected to be routed by the host app.

That made the engineering direction clear:

- detect where the pointer is
- forward spatial input to WebView2

Keyboard input has been more fragile because it depends on a combination of:

- host window focus
- WebView2 controller focus
- GPUI accelerator interception
- internal focus state for the toolbar URL editor
- the composition-hosted browser’s expectation about who owns keyboard input

The result is that keyboard bugs can appear even when mouse input is already correct.

---

## Current Risk Areas

These are the main technical risk areas going forward:

### Risk 1: keyboard fixes regressing pointer behavior

This has already happened multiple times during the effort.

Any future keyboard work should avoid touching:

- hover routing
- wheel routing
- mouse passthrough registration
- cursor ownership logic
- composition visual placement

### Risk 2: focus state disagreement

There are multiple layers of “focus” in this system:

- GPUI focus
- host window focus
- WebView2 controller focus
- DOM element focus inside the page

If any two of those disagree, typing can fail even when clicking works.

### Risk 3: Windows-only progress becoming misleading

Windows now has real architecture changes, but that progress does not mean the same problem is solved on macOS or Linux.

### Risk 4: historical complexity in the working tree

Because the implementation went through multiple strategies, future cleanup and consolidation will matter.

The code should eventually be simplified so the active path is obvious and dead experimental branches are removed.

---

## Recommended Next Technical Direction

If development continues from here, the next steps should be:

1. Freeze the current working Windows pointer path.
2. Treat keyboard as a focus-state problem first, not as a generalized new routing rewrite.
3. Determine the exact authoritative source of “web preview currently owns keyboard.”
4. Keep GPUI accelerator bypass aligned with that single source of truth.
5. Only after Windows is stable should equivalent macOS and Linux work proceed.

The immediate practical rule is:

**Do not re-open hover, wheel, cursor, or z-index plumbing while fixing page keyboard input.**

---

## Command and Verification Notes

During this work, the primary build and run flow used was:

```powershell
just fmt
just run
```

The app currently builds and launches from this flow, with the known unrelated runtime warning:

```text
Error: could not find zed-cli from any of: bin/zed.exe, ./cli.exe
```

That warning is not the core web preview problem.

---

## Current Truthful Status Statement

If this project status had to be stated in one paragraph:

The Windows web preview architecture has been significantly upgraded from the original child-webview model into a composition-hosted WebView2 path that can place GPUI above the page, which is the most important architectural breakthrough of the effort. Mouse interaction work has been implemented in detail. However, keyboard input inside the actual page is still the remaining unresolved Windows interaction problem, and macOS/Linux still do not have equivalent production-ready implementations.

---

## Appendix: Most Relevant Source Files

### Windows web preview host

- [`F:\dx\crates\web_preview\src\windows_visual_webview.rs`](F:\dx\crates\web_preview\src\windows_visual_webview.rs)

### Main preview view and layout integration

- [`F:\dx\crates\web_preview\src\web_preview_view.rs`](F:\dx\crates\web_preview\src\web_preview_view.rs)

### Windows message routing

- [`F:\dx\crates\gpui_windows\src\events.rs`](F:\dx\crates\gpui_windows\src\events.rs)

### Windows passthrough registry and window integration

- [`F:\dx\crates\gpui_windows\src\window.rs`](F:\dx\crates\gpui_windows\src\window.rs)

### Windows accelerator bypass

- [`F:\dx\crates\gpui_windows\src\platform.rs`](F:\dx\crates\gpui_windows\src\platform.rs)

### Windows exports used by the preview crate

- [`F:\dx\crates\gpui_windows\src\gpui_windows.rs`](F:\dx\crates\gpui_windows\src\gpui_windows.rs)

### Original architectural intent document

- [`F:\dx\HOLE_PUNCHING.md`](F:\dx\HOLE_PUNCHING.md)

---

## Final Note

The work so far was not a waste.

The project crossed the hardest architectural line already:

- from “browser is easy but always above the editor”
- to “the editor can own the composed top layer and still host real browser content”

That is real progress.

What remains is the stabilization phase:

- keyboard correctness on Windows
- platform parity on macOS and Linux
- cleanup into a maintainable production-quality structure
```
