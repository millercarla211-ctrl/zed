# DX Style Utility Count Integrity Plan

## Goal

Make Web Preview validate that grouped utility count metadata matches the active utility list supplied by Zed.

## Scope

- Add diagnostics for missing grouped utility counts when utilities are present.
- Add diagnostics for reported utility counts that do not match the utility array length.
- Render the actual utility array length as a fallback display count when the reported count is absent.
- Keep source mutation disabled and avoid runtime/build validation.

## Verification

- Focused Node source guards.
- `git diff --check` and conflict-marker scan.

## Explicitly Skipped

- `just run`.
- Cargo build/check/test/clippy.
- Local servers, browser automation, live WebView proof, live Zed launch, and source mutation.
