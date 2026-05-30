# DX Style Source-Owned Replacement Policy Guard Plan

## Goal

Promote reverse CSS delta replacement-policy evidence from a native implementation detail into the DX Style source-apply and editor write-bridge contracts.

## Tasks

- [x] Add `reverse CSS delta replacement policy match` to the DX Style source-apply contract.
- [x] Add the same guard to the DX Style editor write-bridge preflight contract.
- [x] Sync the Zed embedded source-apply contract mirror.
- [x] Make native source-apply review refuse contracts missing the guard.
- [x] Run source-only verification and commit.

## Constraints

- Keep `source_mutation_enabled` false.
- Do not run `just run`, Cargo, builds, servers, browsers, or runtime validation.
