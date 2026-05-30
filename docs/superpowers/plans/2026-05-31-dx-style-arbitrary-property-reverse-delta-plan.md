# DX Style Arbitrary Property Reverse Delta Plan

## Goal
- Represent remaining shorthand/container CSS declarations as honest arbitrary property review utilities.

## Scope
- Add a source-owned `arbitrary_css_property_value` strategy.
- Cover `border`, `container-type`, and `container-name`.
- Keep source mutation disabled and avoid runtime/build validation until authorized.

## Steps
- [x] Add the DX Style reverse-delta value strategy.
- [x] Map supported declarations to `[property:value]` utilities.
- [x] Teach Zed Web Preview to parse and match arbitrary property utilities.
- [x] Sync Zed's generated reverse-delta contract mirror.
- [x] Add source-only guards for the strategy and mappings.
- [x] Run lightweight source-only verification and create a focused commit.
