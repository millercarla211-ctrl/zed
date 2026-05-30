# DX Style Background Image Reverse Delta Plan

## Goal
- Let visual generator `background-image` output be reviewed as grouped `bg-[...]` atomics without corrupting existing color utilities.

## Scope
- Keep source mutation disabled.
- Cover `background-image` only in this slice; leave shorthand `background`, masks, transforms, containers, and animation for separate reviewed slices.
- Do not run `just run`, Cargo, builds, servers, browsers, or WebView validation.

## Steps
- [x] Add a source-owned `background-image` reverse-delta mapping in DX Style.
- [x] Keep the target utility as `bg-[...]`.
- [x] Use a background-image-specific family matcher so `bg-primary` and other color utilities are preserved.
- [x] Sync Zed's generated reverse-delta contract mirror.
- [x] Add source-only guards for the contract and Web Preview matcher.
- [x] Run lightweight source-only verification and create a focused commit.
