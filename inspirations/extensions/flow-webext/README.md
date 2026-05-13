# Flow WebExtension

This package contains the shared browser-extension shell for Flow.

Targets:

- Chromium browsers with MV3 background service worker and side panel
- Firefox with background document + sidebar
- Safari Web Extension packaging from the same source tree

Primary local-first browser models:

- `onnx-community/Qwen3-0.6B-DQ-ONNX`
- `Xenova/trocr-small-printed`
- `onnx-community/Qwen3.5-0.8B-ONNX`

Build commands:

- `npm install`
- `npm run typecheck`
- `npm run build:chromium`
- `npm run build:firefox`
- `npm run build:safari`
- `npm run build:all`
- `npm run package:all`

The build creates `dist/<browser>/` with a browser-specific `manifest.json`.
`npm run package:all` also creates release zip files plus `.sha256` checksum files under `artifacts/`.

Core pieces:

- `src/runtime/`
  - capability detection
  - browser-pack storage
  - local model fetch interception
  - local Transformers.js inference
- `src/background/`
  - browser-specific background entrypoints
- `src/content/`
  - floating overlay and selection replacement bridge
- `src/ui/`
  - popup, options, sidepanel, sidebar, and offscreen surfaces

The extension is local-first after download. It downloads browser-ready model packs into browser-owned storage and uses local inference by default for text, OCR, and WebGPU-gated multimodal tasks.

Current user-facing screens:

- overview with quick actions and active-tab context
- local workbench for prompt editing, plan review, and output actions
- model-pack management for install, resume, repair, and removal
- settings for local-only mode, auto-apply rewrite, context capture, and preferred models
- delivery/handoff screen for client review before Firebase wiring lands
- in-page overlay for fast confirmation that Flow is active on the current page

Production hardening now includes:

- OPFS-first storage with IndexedDB fallback and extension-storage fallback
- file-by-file pack verification before a pack is treated as ready
- partial pack recovery by skipping already-valid cached files on later downloads
- browser-specific packaged artifacts for Chromium, Firefox, and Safari

Repository-level release guidance lives in:

- `docs/BROWSER_RELEASE.md`
- `docs/CLIENT_HANDOFF.md`
