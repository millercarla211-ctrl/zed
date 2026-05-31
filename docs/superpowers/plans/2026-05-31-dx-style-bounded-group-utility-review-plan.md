# DX Style Bounded Group Utility Review Plan

## Goal

Keep the visible grouped-class utility review in Web Preview aligned with DX Style's source-owned utility count and byte limits.

## Scope

- Add a shared bounded utility helper for Web Preview grouped context diagnostics and visible review.
- Use the bounded helper for `group_utilities_preview`.
- Render only bounded utilities in the visible grouped context review and show how many utilities are displayed.
- Preserve source review/apply gates and keep source mutation disabled.

## Verification

- DX Style fixture mirror freshness check.
- Focused Node source guards.
- `git diff --check` and conflict-marker scan.

## Explicitly Skipped

- `just run`.
- Cargo build/check/test/clippy.
- Local servers, browser automation, live WebView proof, live Zed launch, and source mutation.
