# DX Style Grouping Recommendation Vocabulary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Source-own the allowed grouped-vs-atomic recommendation labels and validate them in Web Preview.

**Architecture:** DX Style's grouped context contract advertises the supported recommendation values. Zed mirrors that fixture, and Web Preview rejects unknown recommendation labels through a source-owned diagnostic code while remaining review-only.

**Tech Stack:** Rust, JSON fixtures, embedded Web Preview JavaScript, Node source guards.

---

### Task 1: DX Style Contract Vocabulary

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_web_preview_context.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-web-preview-context.json`

- [x] Add `recommended_representation_values` with `grouped_reference`, `atomic_utilities`, and `group_candidate_needs_alias`.
- [x] Add `group_context_recommended_representation_unsupported` to `diagnostic_codes`.
- [x] Mirror the fixture into `crates/web_preview/src/dx_style_generator_surface/group-context-contract.generated.json`.

### Task 2: Web Preview Validation

**Files:**
- Modify: `crates/web_preview/src/dx_style_generator_surface/group_context_contract.rs`
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`

- [x] Pass the recommendation vocabulary through the Zed contract adapter.
- [x] Parse the vocabulary in Web Preview.
- [x] Report unsupported recommendation labels when active grouped context carries a value outside the source-owned list.

### Task 3: Guards And Checkpoint

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard the source-owned vocabulary and Web Preview diagnostic.
- [x] Record that source mutation and runtime proof remain disabled.
- [x] Run only lightweight checks and commit the focused Zed checkpoint.
