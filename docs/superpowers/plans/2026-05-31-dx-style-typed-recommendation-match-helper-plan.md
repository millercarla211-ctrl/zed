# DX Style Typed Recommendation Match Helper Plan

Date: 2026-05-31
Scope: `G:\Dx\style` grouped-class read model and `G:\Dx\zed` source guards.

## Goal

Expose a typed DX Style read-model helper for comparing source-provided grouped recommendations with independently derived expected recommendations, so editor integrations do not invent their own match-state semantics.

## Constraints

- Do not run `just run`.
- Do not run Cargo or Just build/check/lint/fmt recipes.
- Do not enable source mutation.
- Keep the Zed side limited to source guards and handoff docs.
- Keep `G:\Dx\style` source changes local because that folder is not a git repo in this workspace.

## Steps

1. Add `GroupedClassRecommendationMatch` to the DX Style grouped-class read model.
2. Add `grouped_class_recommendation_match` for source-vs-expected recommendation comparison.
3. Export the new enum and helper through `src/core/mod.rs`.
4. Guard the source-owned helper from Zed's lightweight source checks.
5. Update handoff docs and run allowed source checks.

## Verification

- `rustfmt --edition 2024 --check G:\Dx\style\src\core\engine\grouped_class_read_model.rs G:\Dx\style\src\core\mod.rs`
- `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`
- `git diff --check`
- conflict-marker source scan
