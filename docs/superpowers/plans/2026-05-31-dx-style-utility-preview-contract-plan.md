# DX Style Utility Preview Contract Plan

## Goal

Move the grouped utility preview length cap into DX Style's source-owned Web Preview context contract so Zed does not hardcode that review limit.

## Scope

- Add `utility_preview_max_chars` to the DX Style grouped-class Web Preview context fixture and Rust contract.
- Mirror the updated fixture into Zed's generated Web Preview contract.
- Pass the cap through Zed's group-context contract adapter.
- Use the source-owned cap in the Web Preview generator script diagnostics and preview helper.

## Verification

- `rustfmt --edition 2024 --check` on changed Rust files only.
- DX Style fixture mirror freshness check.
- Focused Node source guards.
- `git diff --check` and conflict-marker scan.

## Explicitly Skipped

- `just run`.
- Cargo build/check/test/clippy.
- Local servers, browser automation, live WebView proof, live Zed launch, and source mutation.
