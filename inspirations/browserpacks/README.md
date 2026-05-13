# Flow Browserpacks

Browserpacks are Flow's browser-ready local model manifests.

Rules:

- browserpacks reference ONNX or MLC assets, never GGUF
- weights are downloaded on demand into OPFS or IndexedDB-backed browser storage
- the extension resolves local pack files through the browserpack fetch layer
- each pack declares whether WebGPU is required
- each file may declare `sha256` and `size_bytes` for stronger local verification
- manifests stay lightweight and point to remote model assets instead of bundling weights inside the extension package

The example manifests in this directory match the first browser release:

- `qwen3-0.6b.browserpack.json`
- `trocr-small-printed.browserpack.json`
- `qwen3.5-0.8b.browserpack.json`
