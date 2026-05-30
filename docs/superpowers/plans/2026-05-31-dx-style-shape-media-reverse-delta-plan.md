# DX Style Shape Media Reverse Delta Plan

## Goal
- Let shape and mask visual generator output be reviewed as grouped arbitrary utilities.

## Scope
- Cover `clip-path` and `mask-image` only in this slice.
- Leave broad `background`, `border`, transforms, containers, and animation for separate reviewed slices.
- Keep source mutation disabled and avoid runtime/build validation until authorized.

## Steps
- [x] Add source-owned `clip-path` and `mask-image` reverse-delta mappings in DX Style.
- [x] Map the declarations to `clip-path-[...]` and `mask-image-[...]`.
- [x] Sync Zed's generated reverse-delta contract mirror.
- [x] Add source-only guards for the new mappings.
- [x] Run lightweight source-only verification and create a focused commit.
