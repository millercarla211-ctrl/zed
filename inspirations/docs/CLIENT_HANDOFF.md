# Client Handoff

Flow is code-complete and validated for the current repository scope. The main deliverables for handoff are:

## Rust Deliverables

- `configs/production/`
  - target-specific production JSON configs
  - `manifest.json`
  - `README.txt`
- `release/`
  - `flow-release-summary.json`
  - `FLOW_RELEASE_HANDOFF.md`

## Browser Deliverables

- `extensions/flow-webext/artifacts/flow-webext-chromium-v0.1.0.zip`
- `extensions/flow-webext/artifacts/flow-webext-firefox-v0.1.0.zip`
- `extensions/flow-webext/artifacts/flow-webext-safari-v0.1.0.zip`
- matching `.sha256` checksum files

## Local Runtime Defaults On This Machine

- text: `qwen3-0.6b`
- STT: `moonshine-tiny`
- TTS: `kokoro-int8`
- device tier: `Low`
- local-only mode: enabled by default

## Remaining External Tasks

- Firebase project selection and env wiring
- browser-store submission and signing
- optional vendor-specific listing assets and screenshots

These remaining tasks are outside the Rust repository itself. Use `cargo run -- --release-summary` and `cargo run -- --export-release-summary release` to regenerate the machine-specific handoff state before final delivery.
