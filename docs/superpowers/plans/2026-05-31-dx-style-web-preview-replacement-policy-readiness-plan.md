# DX Style Web Preview Replacement Policy Readiness Plan

## Goal

Align Web Preview readiness packets with native reverse CSS delta replacement-policy review so copied evidence cannot drift from native enforcement.

## Tasks

- [x] Expose source-apply required editor guards to the Web Preview generator script.
- [x] Add replacement-policy diagnostics for existing-utility replacement and unchanged utility count.
- [x] Gate future mutation readiness on the source-owned replacement-policy guard.
- [x] Update source guards, docs, todo, and changelog.
- [x] Run source-only verification and commit.

## Constraints

- Keep mutation disabled.
- Do not run `just run`, Cargo, builds, servers, browsers, or runtime validation.
