Flow Production Bundle
======================

crate_version=0.1.0
device_tier=Low
text_model=qwen3-0.6b
stt_model=moonshine-tiny
tts_model=kokoro-int8
all_models_ready=true

Included configs:
  - dx-desktop -> dx-desktop.json
  - browser-extension -> browser-extension.json
  - zed-fork -> zed-fork.json
  - codex-fork -> codex-fork.json
  - zeroclaw-fork -> zeroclaw-fork.json

Validated commands:
  - cargo check
  - cargo test
  - cargo build
  - cargo check -p flow-browser-core
  - cargo check --features example-binaries --examples
  - npm run typecheck (extensions/flow-webext)
  - npm run build:all (extensions/flow-webext)
  - npm run package:all (extensions/flow-webext)

Browser release artifacts:
  - extensions/flow-webext/artifacts/flow-webext-chromium-v0.1.0.zip
  - extensions/flow-webext/artifacts/flow-webext-firefox-v0.1.0.zip
  - extensions/flow-webext/artifacts/flow-webext-safari-v0.1.0.zip