# Flow Release Summary

- crate version: `0.1.0`
- repo root: `F:\flow`
- device tier: `Low`
- selected text model: `qwen3-0.6b`
- selected STT model: `moonshine-tiny`
- selected TTS model: `kokoro-int8`
- production bundle ready: `true`

## Production Bundle Files
- `configs/production/dx-desktop.json`: ready
- `configs/production/browser-extension.json`: ready
- `configs/production/zed-fork.json`: ready
- `configs/production/codex-fork.json`: ready
- `configs/production/zeroclaw-fork.json`: ready
- `configs/production/manifest.json`: ready
- `configs/production/README.txt`: ready

## Browser Release Artifacts
- `extensions/flow-webext/artifacts/flow-webext-chromium-v0.1.0.zip`: ready
- `extensions/flow-webext/artifacts/flow-webext-firefox-v0.1.0.zip`: ready
- `extensions/flow-webext/artifacts/flow-webext-safari-v0.1.0.zip`: ready

## Validated Commands
- `cargo check`
- `cargo test`
- `cargo build`
- `cargo check -p flow-browser-core`
- `cargo check --features example-binaries --examples`
- `npm run typecheck (extensions/flow-webext)`
- `npm run build:all (extensions/flow-webext)`
- `npm run package:all (extensions/flow-webext)`

## External Release Tasks
- `firebase-project-linking`: PendingExternal - Run firebase login, select the production Firebase project, and apply the project env values outside the repository.
- `chromium-store-publish`: PendingExternal - Upload the packaged Chromium zip to the Chrome Web Store or Edge Add-ons dashboard.
- `firefox-amo-publish`: PendingExternal - Upload the packaged Firefox zip to addons.mozilla.org with the reviewed listing assets.
- `safari-xcode-package`: PendingExternal - Wrap the Safari WebExtension assets in Xcode, sign with the Apple team, and submit through App Store Connect.