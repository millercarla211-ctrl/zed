# DX Style Source Grouping Efficiency Plan

## Goal

Move grouped-vs-atomic size estimation into DX Style's own read model so Zed is not the only place with grouping-efficiency evidence.

## Scope

- Add source-owned grouping efficiency structs and recommendation enum in DX Style.
- Add a conservative `grouped_class_grouping_efficiency` helper that reports raw atomic bytes, compact `alias()` bytes, savings, and recommendation.
- Guard the DX Style source from Zed's lightweight source tests.
- Keep source mutation disabled and avoid runtime/build validation.

## Verification

- Focused Rust formatting check for the changed DX Style read-model file.
- Focused Node source guards.
- `git diff --check` and conflict-marker scan.

## Explicitly Skipped

- `just run`.
- Cargo build/check/test/clippy.
- Local servers, browser automation, live WebView proof, live Zed launch, and source mutation.
