# DX Style Group Context Field Contract Plan

## Goal

Keep the DX Style grouped-class Web Preview contract aligned with the active Zed group context payload after adding typed readiness flags.

## Scope

- Add the readiness flag fields to DX Style's source-owned `context_fields` list.
- Mirror the updated fixture into Zed's generated Web Preview contract.
- Have Web Preview verify that grouped contexts are backed by a contract naming those flag fields.
- Keep source review/apply fail-closed when the contract field list drifts.

## Verification

- `rustfmt --edition 2024 --check` on changed Rust files only.
- DX Style fixture mirror freshness check.
- Focused Node source guards.
- `git diff --check` and conflict-marker scan.

## Explicitly Skipped

- `just run`.
- Cargo build/check/test/clippy.
- Local servers, browser automation, live WebView proof, live Zed launch, and source mutation.
