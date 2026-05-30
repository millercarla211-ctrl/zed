# DX Style Native Reverse Delta Policy Guard Plan

## Goal

Make Zed source-apply review treat DX Style reverse CSS delta policy as source-owned contract data, not browser-local convention.

## Tasks

- [x] Pass `fallback_review_properties` and `existing_utility_required_properties` through the Web Preview contract adapter.
- [x] Add replacement-policy evidence to live reverse CSS delta previews.
- [x] Make native source-apply review reject ready high-risk previews that lack same-family replacement evidence.
- [x] Update source guards, todo, changelog, and DX notes.
- [x] Run source-only verification and commit.

## Constraints

- Do not run `just run`, Cargo, builds, servers, browsers, or heavy checks.
- Keep mutation disabled; this slice is review/readiness hardening only.
- Preserve existing Web Preview visual-generator behavior.
