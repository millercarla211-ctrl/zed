# DX Style Group Context Vocabulary Validation Plan

## Goal

Make the Zed Web Preview generator consume DX Style's source-owned grouped-class vocabulary as a live contract, not only as metadata text.

## Scope

- Extend the DX Style grouped-class Web Preview context fixture with a syntax-to-status mapping.
- Mirror that fixture into Zed's generated Web Preview contract JSON.
- Pass the mapping through the Zed contract adapter.
- Validate active `group_context.syntax` and `group_context.status` in the Web Preview generator before review or apply readiness can pass.
- Keep source mutation disabled and preserve all existing source-only gates.

## Verification

- `rustfmt --edition 2024 --check` on changed Rust files only.
- `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`.
- Focused Node source guards.
- `git diff --check` and a targeted conflict-marker scan.

## Explicitly Skipped

- `just run`.
- Cargo build/check/test/clippy.
- Local servers, browser automation, live WebView proof, live Zed launch, and source mutation.
