# DX Style Native Replacement Diagnostics Receipt Plan

## Goal

Keep Web Preview reverse CSS delta replacement payload diagnostics visible and blocking inside native Zed source-apply review receipts.

## Tasks

- [x] Add `reverse_css_delta_replacement_payload_diagnostics` to the source-owned DX Style source-apply review receipt contract.
- [x] Mirror the updated source-apply contract fixture into Zed's embedded Web Preview fallback.
- [x] Validate and preserve bounded replacement payload diagnostics in native source-apply receipts.
- [x] Mark native review blocked when Web Preview reports replacement payload diagnostics.
- [x] Include the diagnostics in the latest Web Preview source-apply receipt summary.
- [x] Run source-only verification and commit.

## Constraints

- Keep source mutation disabled.
- Do not run `just run`, Cargo, builds, servers, browsers, or runtime validation.
