# DX Style Grouping Efficiency Context Plan

## Goal

Expose conservative grouped-vs-atomic size evidence from Zed active group context into the DX Style Web Preview panel.

## Scope

- Add grouping efficiency fields to the DX Style source-owned grouped context contract.
- Mirror the updated fixture into Zed's Web Preview contract.
- Compute raw atomic bytes, `alias()` reference bytes, savings, and recommendation in the native active group context when utilities are known.
- Surface the fields in Web Preview diagnostics and visible grouped context review.
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
