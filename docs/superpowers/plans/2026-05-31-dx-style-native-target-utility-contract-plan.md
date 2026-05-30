# DX Style Native Target Utility Contract Plan

## Goal

Make native reverse CSS delta review verify that a ready preview target utility still matches the source-owned property mapping.

## Tasks

- [x] Validate `target_utility` against the reverse CSS delta contract's `supported_properties`.
- [x] Preserve display keyword and negative margin utility handling.
- [x] Update source guards, docs, todo, and changelog.
- [x] Run source-only verification and commit.

## Constraints

- Keep source mutation disabled.
- Do not run `just run`, Cargo, builds, servers, browsers, or runtime validation.
