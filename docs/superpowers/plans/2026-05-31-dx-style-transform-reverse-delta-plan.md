# DX Style Transform Reverse Delta Plan

## Goal
- Let transform visual generator output be reviewed as grouped `transform-[...]` utilities.

## Scope
- Cover `transform` declarations only in this slice.
- Preserve `rotate-*`, `scale-*`, `translate-*`, and similar component utilities instead of treating them as replaceable transform-family utilities.
- Keep source mutation disabled and avoid runtime/build validation until authorized.

## Steps
- [x] Confirm local DX/Tailwind inventory includes `transform-[...]`.
- [x] Add the source-owned `transform` reverse-delta mapping in DX Style.
- [x] Add transform-specific family matching in Web Preview review.
- [x] Sync Zed's generated reverse-delta contract mirror.
- [x] Add source-only guards for the contract and matcher.
- [x] Run lightweight source-only verification and create a focused commit.
