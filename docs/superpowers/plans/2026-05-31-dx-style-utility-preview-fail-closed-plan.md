# DX Style Utility Preview Fail-Closed Plan

## Goal

Remove the hidden Web Preview fallback for grouped utility preview length so the cap must come from the DX Style source-owned context contract.

## Scope

- Read `utility_preview_max_chars` through the existing contract-number parser.
- Emit a grouped context diagnostic when an active grouped context lacks a valid preview cap.
- Suppress grouped utility preview text when the cap is missing instead of using a Zed-only default.
- Guard the source so future edits do not reintroduce a hardcoded fallback.

## Verification

- Focused Node source guards.
- `git diff --check` and conflict-marker scan.

## Explicitly Skipped

- `just run`.
- Cargo build/check/test/clippy.
- Local servers, browser automation, live WebView proof, live Zed launch, and source mutation.
