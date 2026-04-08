# Windows Web Preview Implementation Report

**Date:** April 9, 2026  
**Project:** Codex fork of Zed  
**Platform status:** Windows web preview completed and working  
**Status policy:** Frozen implementation. Do not casually modify this codepath.  
**Architecture:** Native WebView2 under GPUI with Windows-specific underlay rendering, hit-testing, focus arbitration, and URL-bar handoff

## Executive Summary

The Windows web preview in this fork is now a completed, working implementation that combines a native WebView2 browser surface with GPUI editor chrome and overlays. The core achievement was solving the Windows airspace problem without falling back to a fake browser strategy such as screenshots, image streaming, or video-texture mirroring.

The final Windows result delivers:

- Native page rendering through WebView2
- GPUI controls, toasts, and top chrome rendered above the page
- Stable mouse click, right click, hover, wheel, and keyboard behavior
- Working focus handoff between the page body and the GPUI URL bar
- A browser pane that feels like part of the editor rather than a floating foreign window

This report exists for one reason: the Windows path is complex enough that future work must treat it as **frozen infrastructure**, not as a casual refactor target.

## Freeze Policy

### Windows Is the Reference Implementation

As of April 9, 2026, the Windows web preview is the only fully completed desktop implementation in this fork. It is therefore the reference behavior for:

- z-index correctness
- page interactivity
- URL-bar focus handoff
- toolbar coexistence
- GPUI-over-browser rendering

### Do Not Change These Windows Paths Casually

The following files are part of the known-good Windows behavior and must be treated as sensitive:

- `crates/web_preview/src/web_preview_view.rs`
- `crates/web_preview/src/web_preview.rs`
- `crates/gpui_windows/src/events.rs`
- `crates/gpui_windows/src/window.rs`
- `crates/gpui/src/window.rs`

Even though some of those files are shared or high-level, they contain Windows-sensitive behavior that was tuned through repeated iteration. Any future non-Windows work must avoid refactoring through them unless a confirmed Windows regression requires it.

### Safe Rule for Future Development

For all new macOS and Linux work:

1. Prefer platform-specific files and `#[cfg(...)]` gates.
2. Do not rewrite shared Windows event, preview, or focus logic as part of non-Windows work.
3. If a shared abstraction is unavoidable, preserve the exact Windows execution path first and branch new behavior away from it.

## What Was Actually Built

### 1. Native Web Browser Rendering

The page is rendered by **Microsoft Edge WebView2**, which means:

- HTML, CSS, JavaScript, and layout are native browser work
- media playback is native browser work
- GPU compositing is native browser work
- navigation, security, and browser process behavior stay in the platform browser stack

This is not a fake browser.

It is not:

- a screenshot-updating preview
- a streamed bitmap surface
- a video overlay pretending to be interactive
- a CPU-rendered HTML canvas embedded inside GPUI

That distinction matters because it explains why the final Windows browser can be both fast and visually correct.

### 2. GPUI as the Visible Application Layer

The editor remains a GPUI application. The browser does not get to own the visible application hierarchy. Instead, the working implementation allows the page to render in the preview area while GPUI continues to own:

- the preview toolbar
- the URL input
- editor chrome
- workspace UI
- top-right toasts
- editor overlays and interaction surfaces

That is the real reason the Windows implementation is valuable. It restores editor control over the application interface instead of letting the browser dominate the entire pane.

### 3. Windows-Specific Input Arbitration

The hard part was not page rendering. The hard part was keeping these behaviors alive at the same time:

- click
- right click
- hover
- wheel
- keyboard
- cursor updates
- focus transitions
- URL-bar handoff
- GPUI toolbar interaction

Those were stabilized by carefully coordinating Windows input ownership instead of assuming that the normal child-webview model would solve it.

## Why the Standard Child-Webview Model Was Not Enough

The original `wry`-style child embedding path is acceptable for simple browser panes, but it breaks down for this editor use case because a standard child webview on Windows tends to reclaim visual or input priority in ways that conflict with GPUI.

The specific failures that had to be solved were:

- the browser appearing above GPUI instead of below it
- the page rendering but not receiving real interaction
- hover lag caused by GPUI and the native page fighting over pointer state
- wheel loss because the browser was not the effective recipient
- keyboard ambiguity between the native page and GPUI-owned controls
- URL-bar focus loss when transitioning directly from page focus to toolbar focus

The final Windows path is the result of solving those together, not solving them one by one in isolation.

## Final Windows Architecture

### Rendering Layers

The working mental model is:

1. **Native WebView2 page layer**
2. **GPUI workspace / preview chrome layer**
3. **GPUI overlays and transient UI**

The preview is therefore not treated like a normal child widget sitting on top of the app. It is treated like a page surface integrated into the workspace while GPUI remains the visible application shell.

### Input Layers

The input model is intentionally split:

- native browser input remains native where the page itself must behave like a page
- GPUI input remains GPUI where the toolbar and editor shell must behave like editor UI
- transitions between those two are coordinated explicitly

The important practical outcome is that the user can:

- click links and controls in the page
- scroll normally
- hover normally
- type in page inputs
- then move directly back to the URL bar and type there too

without the whole preview collapsing into broken focus state.

## Core Problems Solved

### Airspace / Z-Index Problem

**Problem:**  
The browser initially behaved like a foreign surface that wanted to float above GPUI, which broke the editor illusion immediately.

**Final outcome:**  
The page now renders as part of the editor while GPUI stays visually above it.

**What that enables:**  
The preview toolbar and GPUI interaction surfaces remain usable even while the page is visible and interactive.

### Mouse Click Reliability

**Problem:**  
At several intermediate stages the page looked correct, but click delivery was either incomplete, over-relayed, or inconsistent with native page expectations.

**Final outcome:**  
The page now receives reliable click behavior that is close to native page behavior and supports normal in-page interaction.

### Hover and Cursor Stability

**Problem:**  
Hover was one of the most fragile behaviors because cursor ownership was repeatedly contested between GPUI and the native page.

**Final outcome:**  
Hover and cursor updates now behave correctly enough for normal page browsing, including rich sites such as video-heavy or button-dense pages.

### Mouse Wheel Delivery

**Problem:**  
Wheel routing was especially sensitive because it often succeeded once or partially, then failed depending on the active recipient window.

**Final outcome:**  
Wheel behavior is now stable in the working Windows path and should be considered part of the frozen implementation.

### Keyboard Input

**Problem:**  
Page typing and GPUI typing needed to coexist without destroying the working pointer path.

**Final outcome:**  
The page can accept keyboard input when it owns focus, and GPUI controls can accept keyboard input when focus returns to them.

### URL-Bar Handoff

**Problem:**  
The most subtle user-facing bug was that moving directly from page focus to the GPUI URL input could leave focus effectively nowhere.

**Final outcome:**  
The direct page-to-URL-bar transition works in the completed Windows path and is one of the reasons this implementation must not be casually rewritten.

## Performance Characteristics

### Why This Is Fast

The Windows implementation is fast because the browser remains native.

That means:

- no frame-by-frame CPU readback
- no browser-as-image transport
- no synthetic remote-preview rendering loop
- no fake browser video layer

Instead:

- WebView2 renders the page
- GPUI renders the editor
- the custom work exists only where composition and focus ownership had to be corrected

### What This Is Not

This project did **not** solve Windows by embedding a slow browser simulation into GPUI.

It did **not** become:

- a screenshot browser
- an offscreen image slideshow
- a streamed DOM viewer
- a "video browser"

That is the main reason the final result still feels performant. The page remains a real browser.

## File-Level Ownership Map

### Windows-Sensitive Web Preview Files

- `crates/web_preview/src/web_preview_view.rs`
  - owns web preview surface behavior, toolbar, and shared preview coordination
- `crates/gpui_windows/src/events.rs`
  - owns Windows-side message and input coordination
- `crates/gpui_windows/src/window.rs`
  - owns Windows host window behavior relevant to preview composition and interaction
- `crates/gpui/src/window.rs`
  - contains shared window behavior that still affects Windows preview focus or passthrough decisions

### Supporting Documentation

- `WEB_PREVIEW_IMPLEMENTATION.md`
- `HOLE_PUNCHING.md`

This report supersedes earlier ad hoc Windows fix notes by providing the stable architecture summary in one place.

## Known Invariants

These invariants should be treated as part of the finished Windows contract:

1. The page must stay natively rendered.
2. GPUI must remain visually above the page.
3. Pointer behavior must not be "fixed" by breaking z-index again.
4. Keyboard behavior must not be "fixed" by breaking pointer interaction again.
5. URL-bar focus handoff must continue to work directly from page focus.
6. Non-Windows work must branch away from this path instead of rewriting it.

## Safe vs Unsafe Changes

### Safe

- adding documentation
- adding platform-specific non-Windows files
- branching new macOS/Linux behavior behind `#[cfg(not(target_os = "windows"))]` or narrower platform gates
- improving non-Windows host abstractions while leaving the Windows branch untouched

### Unsafe

- refactoring shared preview logic without proving the Windows path remains behaviorally identical
- rewriting Windows focus handoff as part of a macOS/Linux feature
- replacing the native page with a simulated or texture-streamed browser path
- simplifying Windows input routing because it "looks complicated"

## Recommended Rule for Future Contributors

When working on web preview after April 9, 2026:

- treat Windows as done
- develop macOS and Linux separately
- use the Windows path as the behavior target, not as the refactor playground
- if Windows must be touched, do it only for a confirmed Windows regression and verify the full interaction matrix afterward

## Verification Checklist

The completed Windows path should be manually rechecked against this list before claiming a Windows change is safe:

- page renders inside the editor
- toolbar stays visible above the page
- click works
- right click works
- hover works
- wheel works
- keyboard works in page inputs
- keyboard works in the URL bar after leaving the page
- direct page-to-URL-bar focus handoff works
- opening GPUI chrome does not permanently kill page interactivity

## Bottom Line

The Windows web preview is no longer an experiment. It is a completed native browser integration with solved editor-over-browser layering and a stabilized interaction model.

That makes it valuable, but it also makes it fragile.

The correct policy from this point forward is simple:

**Do not mess with the Windows web preview unless there is a confirmed Windows bug.**
