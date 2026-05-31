# DX Style Grouped Review Output Fields Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Source-own the copied Web Preview grouped review-output fields that expose expected recommendation and recommendation match state.

**Architecture:** DX Style owns the contract and fixture. Zed mirrors the fixture into Web Preview, validates that the derived copied output fields are declared by DX Style, and reports drift as grouped context diagnostics without enabling source mutation.

**Tech Stack:** Rust contract fixtures, JSON fixture mirrors, Web Preview JavaScript surface, Node source guards.

---

### Task 1: Source-Owned Review Output Fields

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_web_preview_context.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-web-preview-context.json`
- Modify: `G:\Dx\style\README.md`
- Modify: `G:\Dx\style\CHANGELOG.md`

- [ ] Add `review_output_fields` to `GroupedClassWebPreviewContextContract`.
- [ ] Include `group_expected_recommended_representation` and `group_recommendation_match`.
- [ ] Add `group_context_review_output_fields_missing` and `group_context_review_output_field_missing` diagnostics.
- [ ] Document that copied grouped review output fields are source-owned by DX Style.

### Task 2: Zed Mirror And Web Preview Validation

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\group_context_contract.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\script.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\group-context-contract.generated.json`

- [ ] Pass `review_output_fields` through the Zed group-context contract adapter.
- [ ] Refresh the embedded generated fixture mirror with `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --write`.
- [ ] Make Web Preview report missing source-owned review-output field declarations before trusting derived grouped review lines.
- [ ] Emit `group_context_review_output_fields` in copied metadata.

### Task 3: Guards, Docs, Verification, Commit

**Files:**
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [ ] Extend source guards for the contract field, fixture values, diagnostics, adapter, Web Preview validation, and copied metadata line.
- [ ] Update handoff docs with the source-only boundary.
- [ ] Run focused allowed checks only.
- [ ] Commit the Zed mirror/guard checkpoint.

## Verification

- `rustfmt --edition 2024 --check G:\Dx\style\src\core\engine\grouped_class_web_preview_context.rs crates\web_preview\src\dx_style_generator_surface\group_context_contract.rs`
- `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --write`
- `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`
- `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`
- `git diff --check`
- conflict-marker source scan
