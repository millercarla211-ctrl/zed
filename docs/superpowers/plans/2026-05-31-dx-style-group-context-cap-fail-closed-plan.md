# DX Style Group Context Cap Fail-Closed Plan

## Goal

Require DX Style's grouped context count, byte, alias, and candidate caps before Web Preview renders grouped utility evidence.

## Scope

- Parse grouped context caps through the existing contract-number helper.
- Add explicit diagnostics for missing alias, utility count, utility byte, candidate-min, and overlong alias evidence.
- Refuse to render bounded grouped utility lists when count or byte caps are missing.
- Keep source mutation disabled and avoid runtime/build validation.

## Verification

- Focused Node source guards.
- `git diff --check` and conflict-marker scan.

## Explicitly Skipped

- `just run`.
- Cargo build/check/test/clippy.
- Local servers, browser automation, live WebView proof, live Zed launch, and source mutation.
