# DX Style Visual Generator Control Dependencies Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent the Style Web Preview generator cockpit from exposing controls that do not affect emitted class/CSS output.

**Architecture:** DX Style owns recipe runtime value dependencies and generator control metadata. Zed mirrors the fixtures, Web Preview computes whether each control is connected to a recipe placeholder or declared runtime dependency, and copied review output reports unused controls as metadata drift.

**Tech Stack:** Rust read models, JSON fixtures, Web Preview JavaScript, Node source guards.

---

### Task 1: Source-Owned Runtime Dependencies

**Files:**
- Modify: `G:\Dx\style\src\core\engine\visual_generator_recipe_catalog.rs`
- Modify: `G:\Dx\style\fixtures\visual-generator-recipe-catalog.json`

- [x] Add `VisualGeneratorRuntimeValueDependency`.
- [x] Add `VISUAL_GENERATOR_RECIPE_RUNTIME_VALUE_DEPENDENCIES`.
- [x] Include dependencies for derived values such as `css_linear`, `css_mesh`, `css_noise`, `angle_sixth`, `glass_blur`, and `gap_plus_20`.

### Task 2: Connected First-25 Controls

**Files:**
- Modify: `G:\Dx\style\src\core\engine\visual_generator_control_catalog.rs`
- Modify: `G:\Dx\style\fixtures\visual-generator-control-catalog.json`
- Modify: `G:\Dx\style\src\core\engine\visual_generator_recipe_catalog.rs`
- Modify: `G:\Dx\style\fixtures\visual-generator-recipe-catalog.json`

- [x] Remove color controls from effect generators that only use blur.
- [x] Remove layout controls from responsive generators where the recipe does not use them.
- [x] Make clip-path radius, transform duration/easing, mesh angle, and noise strength affect output.

### Task 3: Zed Mirror, Diagnostics, And Guards

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\recipes.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\script.rs`
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Mirror `runtime_value_dependencies` into Web Preview recipe metadata.
- [x] Add `metadata_unused_controls` diagnostics.
- [x] Add source guards proving every control is connected to a recipe placeholder or declared runtime dependency.
- [x] Refresh generated fixture mirrors and commit only the Zed-side checkpoint.

## Verification

- `rustfmt --edition 2024 --check G:\Dx\style\src\core\engine\visual_generator_recipe_catalog.rs G:\Dx\style\src\core\engine\visual_generator_control_catalog.rs crates\web_preview\src\dx_style_generator_surface\recipes.rs`
- `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --write`
- `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`
- `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`
- `git diff --check`
- conflict-marker source scan
