# DX Style Strategy-Aware Target Validation Plan

## Goal

Make native reverse CSS delta target utility validation mirror the source-owned `value_strategy` shapes instead of accepting any target with a matching prefix.

## Tasks

- [x] Inspect Web Preview and DX Style reverse-delta token generation shapes.
- [x] Validate native target utilities by strategy.
- [x] Update source guards and handoff docs.
- [x] Run source-only verification and commit.

## Constraints

- Keep source mutation disabled.
- Do not run `just run`, Cargo, builds, servers, browsers, or runtime validation.
