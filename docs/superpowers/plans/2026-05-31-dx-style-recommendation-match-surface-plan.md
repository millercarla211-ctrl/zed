# DX Style Recommendation Match Surface Plan

Date: 2026-05-31
Scope: `G:\Dx\zed` source-only DX Style Web Preview bridge.

## Goal

Show whether the active grouped-class recommendation agrees with the recommendation independently derived by Web Preview from the current utilities, alias, candidate state, and byte savings.

## Constraints

- Do not run `just run`.
- Do not run Cargo or Just build/check/lint/fmt recipes.
- Do not enable source mutation.
- Keep the change inside the existing DX Style/Zed bridge lane.
- Use source inspection and lightweight guards only.

## Steps

1. Add a small Web Preview helper that returns a recommendation match state.
2. Surface the match state in the grouped-class review.
3. Emit `group_recommendation_match` in copied Web Preview output.
4. Extend source guards and handoff docs.
5. Run allowed source checks and commit.

## Verification

- `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`
- `git diff --check`
- conflict-marker source scan
