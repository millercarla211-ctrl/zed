# DX Style Recommendation Match Vocabulary Plan

Date: 2026-05-31
Scope: `G:\Dx\style` grouped-class contract and `G:\Dx\zed` Web Preview bridge.

## Goal

Move grouped recommendation match-state labels into the DX Style grouped-class Web Preview contract so Zed does not silently invent the vocabulary used by `group_recommendation_match`.

## Constraints

- Do not run `just run`.
- Do not run Cargo or Just build/check/lint/fmt recipes.
- Do not enable source mutation.
- Keep the Zed side limited to fixture mirroring, validation, source guards, and docs.
- Keep `G:\Dx\style` source changes local because that folder is not a git repo in this workspace.

## Steps

1. Add `recommendation_match_values` to the DX Style grouped-class Web Preview context contract and fixture.
2. Mirror the fixture into Zed's embedded generated group context contract.
3. Pass the vocabulary through the Zed Web Preview contract adapter.
4. Make Web Preview report missing or unsupported match-state vocabulary.
5. Extend source guards and handoff docs.
6. Run allowed source checks and commit the Zed changes.

## Verification

- `rustfmt --edition 2024 --check G:\Dx\style\src\core\engine\grouped_class_web_preview_context.rs crates\web_preview\src\dx_style_generator_surface\group_context_contract.rs`
- `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --write`
- `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`
- `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`
- `git diff --check`
- conflict-marker source scan
