# DX Style Group Diagnostic Contract Plan

## Goal

Make grouped context diagnostic names source-owned by DX Style so Web Preview cannot silently invent new grouped-state warnings.

## Scope

- Add grouped context diagnostic code ownership to the DX Style Web Preview context contract and fixture.
- Mirror the updated fixture into Zed's generated Web Preview contract.
- Pass diagnostic code ownership through the Zed contract adapter.
- Report any active Web Preview grouped diagnostics whose base code is not listed by DX Style.
- Keep source mutation disabled and avoid runtime/build validation.

## Verification

- Focused Rust formatting checks for changed Rust files.
- DX Style fixture mirror freshness check.
- Focused Node source guards.
- `git diff --check` and conflict-marker scan.

## Explicitly Skipped

- `just run`.
- Cargo build/check/test/clippy.
- Local servers, browser automation, live WebView proof, live Zed launch, and source mutation.
