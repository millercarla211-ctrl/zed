# DX Style Grouping Recommendation Consistency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Web Preview verify that the grouped-vs-atomic recommendation label matches recomputed grouping evidence.

**Architecture:** DX Style owns the recommendation mismatch diagnostic code. Zed mirrors that contract, and Web Preview derives the expected recommendation from alias presence, candidate state, and grouping savings before reporting drift.

**Tech Stack:** Rust, JSON fixtures, embedded Web Preview JavaScript, Node source guards.

---

### Task 1: Source Diagnostic

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_web_preview_context.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-web-preview-context.json`

- [x] Add `group_context_recommended_representation_mismatch` to the source-owned diagnostic list.
- [x] Mirror the fixture into `crates/web_preview/src/dx_style_generator_surface/group-context-contract.generated.json`.

### Task 2: Web Preview Consistency Check

**Files:**
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`

- [x] Add a helper that derives the expected recommendation.
- [x] Compare the active recommendation label to the expected one when enough context exists.
- [x] Emit the mismatch diagnostic without enabling source mutation.

### Task 3: Guards And Commit

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard the new diagnostic and helper.
- [x] Run allowed lightweight checks.
- [x] Commit only this focused checkpoint.
