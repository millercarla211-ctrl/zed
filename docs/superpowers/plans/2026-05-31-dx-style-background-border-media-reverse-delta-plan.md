# DX Style Background Border Media Reverse Delta Plan

## Goal
- Cover specific media declarations that safely map to atomic utilities without broad shorthand projection.

## Scope
- Add `background-size` -> `bg-size-[...]`.
- Add `border-image` -> `border-image-[...]`.
- Defer broad `background` and `border` shorthand handling because those can combine color, image, position, size, repeat, width, style, and color semantics.
- Keep source mutation disabled and avoid runtime/build validation until authorized.

## Steps
- [x] Add source-owned `background-size` and `border-image` reverse-delta mappings in DX Style.
- [x] Sync Zed's generated reverse-delta contract mirror.
- [x] Add source-only guards for the new mappings.
- [x] Run lightweight source-only verification and create a focused commit.
