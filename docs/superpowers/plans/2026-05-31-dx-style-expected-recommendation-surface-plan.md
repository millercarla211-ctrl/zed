# DX Style Expected Recommendation Surface Plan

Date: 2026-05-31
Scope: `G:\Dx\zed` source-only DX Style Web Preview bridge.

## Goal

Expose the expected grouped-vs-atomic recommendation that Web Preview already derives from active grouped-class context, so the Style panel review output shows both the source-provided recommendation and the independently recomputed expectation.

## Constraints

- Do not run `just run`.
- Do not run Cargo or Just build/check/lint/fmt recipes.
- Do not enable source mutation.
- Keep the change inside the existing DX Style/Zed Web Preview bridge lane.
- Use lightweight source checks only.

## Steps

1. Add the derived expected recommendation to the visible grouped-class review.
2. Add the same value to copied Web Preview context output as `group_expected_recommended_representation`.
3. Extend source guard tests so the review label and output line cannot drift silently.
4. Update DX handoff docs, todo, and changelog.
5. Run allowed source checks and commit the bounded change.

## Verification

- `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`
- `git diff --check`
- conflict-marker source scan
