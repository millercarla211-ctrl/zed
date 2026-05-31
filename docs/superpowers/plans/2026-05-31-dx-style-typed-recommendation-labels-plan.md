# DX Style Typed Recommendation Labels Plan

Date: 2026-05-31
Scope: `G:\Dx\style` grouped-class read model/context contract and `G:\Dx\zed` source guards.

## Goal

Tie grouped recommendation and recommendation-match contract labels to typed DX Style read-model enums instead of maintaining duplicated string lists in the Web Preview context contract.

## Constraints

- Do not run `just run`.
- Do not run Cargo or Just build/check/lint/fmt recipes.
- Do not enable source mutation.
- Keep the Zed side limited to source guards and handoff docs.
- Keep `G:\Dx\style` source changes local because that folder is not a git repo in this workspace.

## Steps

1. Add `as_str()` methods to `GroupedClassRecommendedRepresentation` and `GroupedClassRecommendationMatch`.
2. Use those enum labels in the DX Style grouped-class Web Preview context contract.
3. Guard the typed label source from Zed's lightweight source checks.
4. Update handoff docs and run allowed source checks.

## Verification

- `rustfmt --edition 2024 --check G:\Dx\style\src\core\engine\grouped_class_read_model.rs G:\Dx\style\src\core\engine\grouped_class_web_preview_context.rs`
- `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`
- `git diff --check`
- conflict-marker source scan
