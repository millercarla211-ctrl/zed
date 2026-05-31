# DX Style Utility Preview Truncation Evidence Plan

## Goal

Make grouped utility preview truncation visible in Web Preview context output so bounded atomics are never mistaken for the complete expansion.

## Scope

- Return grouped utility preview metadata alongside the preview string.
- Emit displayed/total character counts when a grouped utility preview is shown.
- Emit an explicit `group_utilities_preview_truncated: true` line when the source-owned preview cap shortens the displayed atomics.
- Keep mutation disabled and avoid runtime/build validation.

## Verification

- Focused Node source guards.
- `git diff --check` and conflict-marker scan.

## Explicitly Skipped

- `just run`.
- Cargo build/check/test/clippy.
- Local servers, browser automation, live WebView proof, live Zed launch, and source mutation.
