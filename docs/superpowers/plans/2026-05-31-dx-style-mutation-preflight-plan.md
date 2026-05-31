# DX Style Mutation Writer Preflight Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Emit fail-closed mutation writer preflight evidence that names the exact unproven runtime and writer gates before any source mutation can occur.

**Architecture:** DX Style remains source-owned and mutation-disabled. Zed native source-apply review and Web Preview copied packets emit a preflight receipt derived from existing commit/runtime/mutation templates, and readiness refuses to graduate unless that receipt is present and ready.

**Tech Stack:** Rust, Zed Web Preview JavaScript, Node source guards.

---

### Task 1: Source-Owned Receipt Field

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_source_apply.rs`
- Modify: `G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-source-apply-contract.json`
- Modify: `G:\Dx\style\fixtures\grouped-class-editor-write-bridge-preflight.json`

- [x] Add `native_mutation_writer_preflight` to the source-apply review receipt fields.
- [x] Require the same field from the editor write bridge.
- [x] Keep `source_mutation_enabled=false` and `can_mutate_source=false`.

### Task 2: Native And Web Preview Evidence

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_source_apply.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\script.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\web_preview_view.rs`

- [x] Emit `native_mutation_writer_preflight` from native source-apply review.
- [x] Emit the same field from Web Preview copied review packets as `not_performed_in_web_preview`.
- [x] Include the preflight in latest source-apply receipt summaries.

### Task 3: Readiness And Guards

**Files:**
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\editor_write_bridge.rs`
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`

- [x] Add fallback bridge support for the preflight receipt field.
- [x] Add missing-preflight readiness blockers.
- [x] Extend source guards for native/Web Preview output and bridge fallback.

### Task 4: Handoff And Verification

**Files:**
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Document the exact remaining blocker honestly: authorized runtime proof plus mutation-capable writer implementation.
- [x] Run targeted `rustfmt --check`.
- [x] Run the fixture sync check.
- [x] Run focused Node source guards.
- [x] Run `git diff --check` and conflict-marker scan.
- [x] Commit with `feat: add DX Style mutation writer preflight`.
