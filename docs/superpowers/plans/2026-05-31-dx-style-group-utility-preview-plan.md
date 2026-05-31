# DX Style Group Utility Preview Plan

## Goal

Expose the actual grouped atomic utilities already carried by Zed's active Style context in the Web Preview diagnostics, with a bounded preview suitable for source review.

## Scope

- Add a bounded grouped utility preview helper in the Web Preview generator script.
- Show `group_utilities_preview` only when active grouped utilities are present.
- Preserve existing review/apply gates and keep source mutation disabled.
- Guard the helper and diagnostic line through the source-only test suite.

## Verification

- Focused Node source guards.
- `git diff --check` and conflict-marker scan.

## Explicitly Skipped

- `just run`.
- Cargo build/check/test/clippy.
- Local servers, browser automation, live WebView proof, live Zed launch, and source mutation.
