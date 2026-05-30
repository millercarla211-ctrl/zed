# DX Style Replacement Source Declaration Validation Plan

## Goal

Make reverse CSS delta review prove that the human-readable grouped source declaration matches the active group alias and replacement utility list.

## Tasks

- [x] Validate native ready previews against `@alias(replacement utilities...)`.
- [x] Add Web Preview diagnostics for missing or mismatched replacement source declarations.
- [x] Update source guards, docs, todo, and changelog.
- [x] Run source-only verification and commit.

## Constraints

- Keep source mutation disabled.
- Do not run `just run`, Cargo, builds, servers, browsers, or runtime validation.
