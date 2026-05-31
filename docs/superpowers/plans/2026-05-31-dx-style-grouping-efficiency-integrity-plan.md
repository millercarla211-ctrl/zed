# DX Style Grouping Efficiency Integrity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Web Preview verify grouped-vs-atomic size evidence instead of only displaying the numbers supplied by Zed.

**Architecture:** DX Style owns the grouped context diagnostic code contract. Zed mirrors that fixture into the embedded Web Preview contract, and Web Preview recomputes raw atomic bytes, compact `alias()` reference bytes, and grouping savings from the active context before accepting the evidence as aligned.

**Tech Stack:** Rust, JSON fixtures, embedded Web Preview JavaScript, Node source guards.

---

### Task 1: Source-Owned Diagnostic Codes

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_web_preview_context.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-web-preview-context.json`
- Modify generated mirror through `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --write`

- [x] Add `group_context_raw_atomic_bytes_mismatch`, `group_context_grouped_reference_bytes_mismatch`, and `group_context_grouping_savings_bytes_mismatch` to the DX Style diagnostic list.
- [x] Mirror the updated fixture into `crates/web_preview/src/dx_style_generator_surface/group-context-contract.generated.json`.

### Task 2: Web Preview Integrity Check

**Files:**
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`

- [x] Add tiny helpers that recompute raw atomic bytes from the utility list, compact grouped reference bytes from `alias()`, and savings from those values.
- [x] Emit mismatch diagnostics when the active context numbers do not match recomputed values.
- [x] Keep review-only behavior and source mutation disabled.

### Task 3: Guards And Handoff

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard the new DX Style diagnostic codes and Web Preview helper names.
- [x] Record the source-only checkpoint and explicitly state that no runtime/build/source mutation proof was run.
- [x] Run only allowed lightweight checks: targeted `rustfmt --check`, fixture mirror check, focused Node source guards, `git diff --check`, and conflict-marker scan.
