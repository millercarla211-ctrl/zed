# DX Style Web Preview Replacement Payload Diagnostics Plan

## Goal

Surface reverse CSS delta replacement payload limit blockers in the Web Preview generator cockpit before native source-apply review.

## Tasks

- [x] Read replacement utility count, per-utility byte, and replacement source declaration byte limits from the source-owned reverse CSS delta contract.
- [x] Add Web Preview diagnostics for oversized target utilities, replacement utility arrays, individual utilities, and source declarations.
- [x] Include those diagnostics in source-apply payload diagnostics, copied review packets, and the reverse CSS delta contract review surface.
- [x] Update source guards for the new diagnostics and review-packet fields.
- [x] Run source-only verification and commit.

## Constraints

- Keep source mutation disabled.
- Do not run `just run`, Cargo, builds, servers, browsers, or runtime validation.
