# DX Style Background Shorthand Reverse Delta Plan

## Goal
- Let broad visual-generator `background` output be reviewed as grouped `bg-[...]` utilities without corrupting plain color tokens.

## Scope
- Add `background` -> `bg-[...]`.
- Reuse the background-specific matcher so `bg-primary` is preserved while existing arbitrary/gradient backgrounds can be replaced.
- Defer broad `border` shorthand and container metadata.
- Keep source mutation disabled and avoid runtime/build validation until authorized.

## Steps
- [x] Add the source-owned `background` reverse-delta mapping in DX Style.
- [x] Apply the background-specific matcher in Zed Web Preview.
- [x] Sync Zed's generated reverse-delta contract mirror.
- [x] Add source-only guards for the mapping and matcher.
- [x] Run lightweight source-only verification and create a focused commit.
