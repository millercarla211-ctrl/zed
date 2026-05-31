# DX Style Active Group Readiness Flags Plan

## Goal

Expose source-owned grouped-class readiness facts in Zed's active Style context so Web Preview can validate group state from typed booleans instead of inferring everything from string status values.

## Scope

- Add `requires_registry_receipt`, `source_owned`, and `can_expand_inline` to the active Zed `group_context` JSON payload.
- Surface those booleans in the Web Preview generator diagnostics.
- Fail source review/apply readiness when grouped context metadata contradicts the source-owned vocabulary.
- Keep source mutation disabled and preserve all existing editor write gates.

## Verification

- `rustfmt --edition 2024 --check` on touched Rust files only.
- DX Style fixture mirror freshness check.
- Focused Node source guards.
- `git diff --check` and conflict-marker scan.

## Explicitly Skipped

- `just run`.
- Cargo build/check/test/clippy.
- Local servers, browser automation, live WebView proof, live Zed launch, and source mutation.
