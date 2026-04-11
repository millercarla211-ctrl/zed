# Liquid Glass Status

**Date:** April 10, 2026  
**Project:** Zed / GPUI fork (`F:\dx`)

## Executive Summary

The Liquid Glass feature is now integrated into the editor as a **native GPUI GPU-rendered workspace item** rather than a standalone `wgpu + imgui` demo window.

The current implementation is **good as a configurable in-editor demo and renderer integration milestone**, and the core glass effect is now treated as a **separate shader-driven element** rather than as the background image itself.

## Current Status

- **Integrated native workspace item:** `95/100`
- **Cross-platform renderer backend wiring:** `90/100`
- **Reusable GPUI paint/component API:** `80/100`
- **True live-backdrop glass over arbitrary editor UI:** `25/100`
- **Overall Liquid Glass implementation status:** `85/100`

## What Is Completed

### 1. Native editor integration

Liquid Glass is no longer a separate standalone app.

Implemented:
- New **Liquid Glass** workspace item/tab in the editor
- Native GPUI controls instead of imgui controls
- Integrated asset loading through the main workspace asset pipeline
- Copy-config workflow from the control panel
- Theme-aware control styling

Relevant files:
- [F:\dx\crates\liquid_glass\src\liquid_glass_view.rs](/F:/dx/crates/liquid_glass/src/liquid_glass_view.rs)
- [F:\dx\crates\liquid_glass\src\ui_state.rs](/F:/dx/crates/liquid_glass/src/ui_state.rs)
- [F:\dx\crates\liquid_glass\src\backgrounds.rs](/F:/dx/crates/liquid_glass/src/backgrounds.rs)
- [F:\dx\crates\workspace\src\workspace.rs](/F:/dx/crates/workspace/src/workspace.rs)
- [F:\dx\crates\workspace\src\pane.rs](/F:/dx/crates/workspace/src/pane.rs)
- [F:\dx\crates\zed\src\main.rs](/F:/dx/crates/zed/src/main.rs)

### 2. GPU primitive path

Liquid Glass is rendered as a **real GPUI GPU primitive**, not a third-party wrapper.

Implemented:
- GPUI-side Liquid Glass scene primitive and paint path
- Platform shader backend support
- Shader behavior fixed so pixels **outside** the glass silhouette are transparent instead of drawing a boxed image rectangle

Relevant files:
- [F:\dx\crates\gpui\src\window.rs](/F:/dx/crates/gpui/src/window.rs)
- [F:\dx\crates\gpui\src\scene.rs](/F:/dx/crates/gpui/src/scene.rs)
- [F:\dx\crates\gpui_wgpu\src\shaders.wgsl](/F:/dx/crates/gpui_wgpu/src/shaders.wgsl)
- [F:\dx\crates\gpui_macos\src\shaders.metal](/F:/dx/crates/gpui_macos/src/shaders.metal)
- [F:\dx\crates\gpui_windows\src\shaders.hlsl](/F:/dx/crates/gpui_windows/src/shaders.hlsl)

### 3. Cross-platform backend wiring

The Liquid Glass primitive is wired into the existing renderer backends rather than staying demo-only.

Implemented:
- Windows shader/backend support
- macOS Metal shader/backend support
- shared `wgpu` / WGSL path

Relevant files:
- [F:\dx\crates\gpui_windows\build.rs](/F:/dx/crates/gpui_windows/build.rs)
- [F:\dx\crates\gpui_windows\src\shaders.hlsl](/F:/dx/crates/gpui_windows/src/shaders.hlsl)
- [F:\dx\crates\gpui_macos\build.rs](/F:/dx/crates/gpui_macos/build.rs)
- [F:\dx\crates\gpui_macos\src\metal_renderer.rs](/F:/dx/crates/gpui_macos/src/metal_renderer.rs)
- [F:\dx\crates\gpui_macos\src\shaders.metal](/F:/dx/crates/gpui_macos/src/shaders.metal)
- [F:\dx\crates\gpui_wgpu\src\wgpu_renderer.rs](/F:/dx/crates/gpui_wgpu/src/wgpu_renderer.rs)

## What The Current Demo Actually Does

The Liquid Glass tab currently has:
- a **static image preview panel**
- one **single Liquid Glass element** rendered above that panel
- GPUI-native controls that modify the glass parameters

This is now the correct demo structure:
- the background image is just the preview content
- the moving glass shape is the actual Liquid Glass effect
- the preview image itself is not supposed to be the Liquid Glass element

## What The Core Component Actually Is

The core Liquid Glass effect is:
- a translucent shader-driven rectangle/shape
- rendered separately from the background content
- configurable through GPUI controls
- reusable anywhere a caller can provide the source content it should refract/sample

That means the **core component itself does not require a whole-editor live-backdrop system**.

## Important Current Limitation

The current Liquid Glass element is still **source-surface-backed**, not a true **live-backdrop editor-wide glass layer**.

That means:
- it can correctly show Liquid Glass over provided preview/source content
- it can correctly refract/sample that source content
- but it does **not yet** sample the actual live GPUI scene behind arbitrary components across the whole editor

In plain terms:
- **Current:** “glass over preview content”
- **Not finished yet:** “glass over any real editor UI behind it”

## Why This Limitation Exists

The current GPUI Liquid Glass paint path requires an explicit source surface/image:

- [F:\dx\crates\gpui\src\window.rs](/F:/dx/crates/gpui/src/window.rs) `paint_liquid_glass(...)`

That source is currently a `RenderImage`, which works well for:
- demo backgrounds
- preview panels
- controlled component-backed glass effects

That is enough for:
- a real Liquid Glass element over demo content
- a reusable “glass over provided source content” API

It does **not** yet provide:
- a live scene backdrop texture of already-rendered GPUI UI
- per-component backdrop sampling from the current frame
- automatic sampling of arbitrary already-rendered editor UI without an explicit source

## What Is Still Missing

### 1. True live-backdrop Liquid Glass

Needed:
- renderer-level backdrop capture of already-rendered GPUI content
- ability for Liquid Glass to sample that live backdrop texture
- correct ordering so components behind the glass are rendered first, then sampled by the glass pass

This is the most important missing piece.

### 2. Reusable component API

Needed:
- a proper GPUI-level abstraction such as:
  - `liquid_glass_background(...)`
  - or a dedicated reusable Liquid Glass container/background element

Current state:
- demo item exists
- primitive exists
- reusable paint helper exists
- higher-level GPUI composition API is still incomplete

### 3. Whole-editor mouse-follow with correct live content

The current request is for the glass element to be able to move throughout the whole editor while still looking like real glass over whatever UI is behind it.

That requires:
- live backdrop sampling
- not just moving the panel-backed demo glass around

Without the live backdrop, whole-editor movement just turns into dragging preview-backed imagery around, which is the wrong visual model.

## Architectural Truth

### What is correct right now

- The shader math and primitive integration are real
- The effect is GPU-native inside GPUI
- The editor integration is real
- The control system is native GPUI

### What is not correct to claim yet

- That Liquid Glass is already a universal background for any GPUI element
- That it already samples the real live editor scene behind it
- That the preview background image is irrelevant to the current renderer path

The preview background is still important because it is the current source texture.

## Recommended Next Step

The next major implementation step should be:

1. Finish the higher-level reusable GPUI Liquid Glass component API on top of the current paint helper
2. Keep the demo item using that shared component path
3. Separately, add a **backdrop texture / scene capture pass** only if whole-editor live UI sampling is still required later

This is the correct path to the real product goal.

## Current Conclusion

Liquid Glass is now:
- integrated into the editor
- GPU-native
- platform-renderer-backed
- visually working as a demo effect over preview content

Liquid Glass is **not yet fully complete** as:
- a polished universal GPUI component API
- a true live-backdrop glass system over arbitrary GPUI UI

## File Inventory

Primary current implementation files:
- [F:\dx\crates\liquid_glass\src\liquid_glass_view.rs](/F:/dx/crates/liquid_glass/src/liquid_glass_view.rs)
- [F:\dx\crates\liquid_glass\src\ui_state.rs](/F:/dx/crates/liquid_glass/src/ui_state.rs)
- [F:\dx\crates\liquid_glass\src\backgrounds.rs](/F:/dx/crates/liquid_glass/src/backgrounds.rs)
- [F:\dx\crates\gpui\src\window.rs](/F:/dx/crates/gpui/src/window.rs)
- [F:\dx\crates\gpui\src\scene.rs](/F:/dx/crates/gpui/src/scene.rs)
- [F:\dx\crates\gpui_wgpu\src\shaders.wgsl](/F:/dx/crates/gpui_wgpu/src/shaders.wgsl)
- [F:\dx\crates\gpui_macos\src\shaders.metal](/F:/dx/crates/gpui_macos/src/shaders.metal)
- [F:\dx\crates\gpui_windows\src\shaders.hlsl](/F:/dx/crates/gpui_windows/src/shaders.hlsl)
