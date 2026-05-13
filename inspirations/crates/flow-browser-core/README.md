# flow-browser-core

`flow-browser-core` is the WASM-oriented browser orchestration crate for Flow.

It is intentionally smaller than the main native `flow` crate and focuses on:

- browser capability detection inputs
- browser execution planning
- browser pack metadata
- browser message protocol payloads

The extension shell can compile this crate to WebAssembly and call it from popup,
side panel, content overlay, worker, or Safari Web Extension surfaces without
pulling in the desktop-only runtime stack.
