# DX Style Reverse Delta Replacement Guard Plan

## Goal
- Prevent high-risk reverse-delta mappings from inventing new source intent by appending utilities when no same-family utility exists.

## Scope
- Source-own the high-risk property list in DX Style's reverse-delta contract.
- Enforce the list in Zed Web Preview review previews.
- Keep source mutation disabled and avoid runtime/build validation until authorized.

## Steps
- [x] Add `existing_utility_required_properties` to the DX Style reverse-delta contract.
- [x] Include high-risk background, border, shape, transform, transition-property, animation, and container metadata properties.
- [x] Sync Zed's generated reverse-delta contract mirror.
- [x] Update Web Preview to refuse ready previews for those properties without an existing same-family source utility.
- [x] Add source-only guards for the contract list and Web Preview refusal path.
- [x] Run lightweight source-only verification and create a focused commit.
