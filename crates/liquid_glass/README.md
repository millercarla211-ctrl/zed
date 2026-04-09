# Liquid Glass - Pure Rust + OpenGL

A complete Rust port of [OverShifted/LiquidGlass](https://github.com/OverShifted/LiquidGlass) using OpenGL directly with the exact same shaders.

## Features

- ✅ Uses the exact GLSL shaders from `./assets/shaders/`
- ✅ OpenGL 4.5 rendering
- ✅ VSync disabled for maximum FPS
- ✅ Full keyboard controls (WASD + Arrow keys)
- ✅ All shader parameters from the original
- ✅ Blur post-processing
- ✅ Background image loading

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```

## Controls

- **WASD**: Move the glass element
- **Arrow Keys**: Move the camera
- **Mouse**: Click and drag (planned)

## Architecture

This implementation:
1. Loads shaders from `./assets/shaders/BatchRenderer2D.glsl` and `Blur.glsl`
2. Uses `glutin` for OpenGL context creation
3. Uses `winit` for windowing
4. Renders with pure OpenGL calls
5. Matches the C++ version's rendering pipeline exactly

## Performance

With VSync disabled, this achieves the same unlimited FPS as the C++ version.
