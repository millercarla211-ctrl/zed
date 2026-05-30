# DX Style Animation Reverse Delta Plan

## Goal
- Let keyframe timeline `animation` output be reviewed as grouped `animate-[...]` utilities.

## Scope
- Cover `animation` declarations only as review evidence.
- Keep DX-owned `@keyframes` writing and all source mutation disabled until the authorized writer/runtime proof path exists.
- Avoid runtime/build validation until authorized.

## Steps
- [x] Add the source-owned `animation` reverse-delta mapping in DX Style.
- [x] Sync Zed's generated reverse-delta contract mirror.
- [x] Add source-only guards for the new mapping.
- [x] Run lightweight source-only verification and create a focused commit.
