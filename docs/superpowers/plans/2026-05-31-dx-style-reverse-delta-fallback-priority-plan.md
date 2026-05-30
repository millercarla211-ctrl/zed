# DX Style Reverse Delta Fallback Priority Plan

## Goal
- Keep broad generator declarations from masking more specific reverse-delta review candidates in the same generated CSS output.

## Scope
- Source-own the fallback property list in DX Style's reverse-delta contract.
- Use the source-owned list in Zed Web Preview.
- Keep source mutation disabled and avoid runtime/build validation until authorized.

## Steps
- [x] Add `fallback_review_properties` to the DX Style reverse-delta contract.
- [x] Include `display`, `transition-property`, `background`, and `border`.
- [x] Sync Zed's generated reverse-delta contract mirror.
- [x] Update Web Preview fallback selection to use the source-owned list.
- [x] Add source-only guards for the contract list and Web Preview usage.
- [x] Run lightweight source-only verification and create a focused commit.
