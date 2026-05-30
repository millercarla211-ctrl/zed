# DX Style Reverse Delta Coverage Guard Plan

## Goal
- Prevent the source-owned visual generator recipe catalog from drifting beyond reverse-delta review coverage.

## Scope
- Add a source-only guard in the existing Node source test.
- Check only simple `property: value;` declarations from the first 25 recipe CSS templates.
- Keep source mutation disabled and avoid runtime/build validation until authorized.

## Steps
- [x] Extract CSS declaration properties from recipe templates.
- [x] Compare them with the source-owned reverse-delta supported property list.
- [x] Fail the guard on uncovered generator declaration properties.
- [x] Run lightweight source-only verification and create a focused commit.
