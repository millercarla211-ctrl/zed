# DX Style Easing Metadata Alignment Plan

## Goal
- Align the easing/cubic-bezier generator metadata with the transition-timing recipe and reverse CSS delta contract.

## Scope
- Keep source mutation disabled.
- Do not add animation mutation support in this slice.
- Do not run `just run`, Cargo, builds, servers, browsers, or WebView validation.

## Steps
- [x] Confirm the easing recipe emits `transition-timing-function`.
- [x] Update DX Style source-owned generator metadata from `[animation-timing-function:...]` to `[transition-timing-function:...]`.
- [x] Update the CSS declaration hint to route `transition-timing-function` to `ease-*`.
- [x] Sync Zed's generated fixture mirrors from DX Style fixtures.
- [x] Add source-only guards proving the generated mirrors and source fixtures stay aligned.
- [x] Run lightweight source-only verification and create a focused commit.
