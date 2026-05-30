# DX Style Reverse Delta Payload Bounds Plan

## Goal

Keep reverse CSS delta review payloads bounded and source-owned before Zed records a native source-apply review receipt.

## Tasks

- [x] Add source-owned DX Style contract limits for replacement utility count, utility byte size, and replacement source declaration byte size.
- [x] Mirror the updated reverse CSS delta fixture into Zed's embedded Web Preview fallback.
- [x] Validate those contract limits in native Zed source-apply review.
- [x] Validate ready reverse-delta replacement utilities as bounded string arrays before source declaration comparison.
- [x] Add source guards for fixture strategy shape, grid row/column repeat targets, and native payload-bound diagnostics.
- [x] Run source-only verification and commit.

## Constraints

- Keep source mutation disabled.
- Do not run `just run`, Cargo, builds, servers, browsers, or runtime validation.
