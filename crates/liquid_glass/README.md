# Liquid Glass

This crate now provides the integrated Liquid Glass workspace item for this Zed fork.

It is no longer a standalone windowed demo. The effect is rendered as a native GPUI GPU primitive
and is opened from the workspace as a first-class tab beside Web Preview.

## Integration

- GPU primitive: `gpui::Window::paint_liquid_glass`
- Workspace item: `LiquidGlassView`
- Assets: loaded from the root `assets/liquid_glass/` tree
- Controls: native GPUI dropdowns, switches, and custom GPUI sliders

## Backend model

- Windows: DirectX renderer with HLSL shader support
- macOS: Metal renderer with Metal shader support
- Linux and shared GPU path: wgpu renderer with WGSL shader support

The old imgui/winit standalone path has been retired in favor of the editor-integrated GPUI path.
