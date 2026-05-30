# DX Style Reverse Delta Strategy Drift Guard Plan

## Goal

Prevent DX Style reverse-delta `value_strategy` additions from silently drifting past Web Preview or native Zed source-apply review support.

## Tasks

- [x] Derive the strategy set from the DX Style reverse-delta fixture.
- [x] Assert the fixture strategy set matches the supported strategy list.
- [x] Assert Web Preview and native source-apply review mention every supported strategy.
- [x] Update handoff docs.
- [x] Run source-only verification and commit.

## Constraints

- Keep source mutation disabled.
- Do not run `just run`, Cargo, builds, servers, browsers, or runtime validation.
