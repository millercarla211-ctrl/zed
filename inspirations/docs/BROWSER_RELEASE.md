# Browser Release

This document covers the production release flow for the shared Flow WebExtension.

## Artifacts

Packaged browser artifacts are generated under `extensions/flow-webext/artifacts/`:

- `flow-webext-chromium-v0.1.0.zip`
- `flow-webext-firefox-v0.1.0.zip`
- `flow-webext-safari-v0.1.0.zip`
- matching `.sha256` checksum files

These are produced by:

- `npm run package:all` in `extensions/flow-webext`

## Pre-Publish Validation

Validated command matrix for the current repo scope:

- `cargo check`
- `cargo test`
- `cargo build`
- `cargo check -p flow-browser-core`
- `cargo check --features example-binaries --examples`
- `npm run typecheck`
- `npm run build:all`
- `npm run package:all`

Optional repo-side exports:

- `cargo run -- --export-production-bundle configs/production`
- `cargo run -- --export-release-summary release`

## Chromium / Edge

1. Load `dist/chromium/` unpacked in Chrome or Edge for a final smoke pass.
2. Verify:
   - popup opens
   - side panel opens
   - content overlay appears
   - local pack manager can resolve `qwen3-0.6b`
3. Upload `flow-webext-chromium-v0.1.0.zip` to the Chrome Web Store.
4. Reuse the same package for Edge Add-ons unless store-specific metadata changes are required.

## Firefox

1. Load `dist/firefox/` as a temporary add-on.
2. Verify:
   - popup opens
   - sidebar opens
   - content overlay appears
   - local pack manager can resolve `qwen3-0.6b`
3. Upload `flow-webext-firefox-v0.1.0.zip` to addons.mozilla.org.

## Safari

1. Use the shared Safari-targeted assets from `dist/safari/`.
2. In Xcode:
   - create or open the Safari Web Extension host app
   - import the built extension assets
   - apply the production bundle identifier and signing team
   - archive and validate the app
3. Submit through App Store Connect.

The repo does not perform the Apple signing or App Store Connect submission automatically. That remains vendor-side release work.

## External Dependencies

Browser-store publication is external to this repository:

- Chrome Web Store developer account
- Edge Add-ons developer account if published separately
- Mozilla Add-ons developer account
- Apple Developer team and App Store Connect access
