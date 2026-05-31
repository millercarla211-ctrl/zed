# DX Style Native Writer Dispatch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an explicit fail-closed native writer dispatch receipt so source-apply review distinguishes readiness evidence from the actual mutation writer handoff.

**Architecture:** DX Style remains source-owned and mutation-disabled. Zed native review and Web Preview copied packets emit `native_writer_dispatch` as a review receipt field. The field reports that no source writer was invoked unless a future mutation-capable implementation is explicitly wired and authorized.

**Tech Stack:** Rust, Zed Web Preview JavaScript, Node source guards.

---

### Task 1: Source-Owned Dispatch Field

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_source_apply.rs`
- Modify: `G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-source-apply-contract.json`
- Modify: `G:\Dx\style\fixtures\grouped-class-editor-write-bridge-preflight.json`

- [x] Add `native_writer_dispatch` to the source-apply review receipt fields.
- [x] Require the same field from the editor write bridge.
- [x] Keep `source_mutation_enabled=false` and `can_mutate_source=false`.

### Task 2: Native And Web Preview Dispatch Evidence

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_source_apply.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\script.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\web_preview_view.rs`

- [x] Emit `native_writer_dispatch` from native source-apply review.
- [x] Emit the same field from Web Preview copied review packets as `not_performed_in_web_preview`.
- [x] Include the dispatch packet in latest source-apply receipt summaries.

### Task 3: Guards And Handoff

**Files:**
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\editor_write_bridge.rs`
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Add fallback bridge support for the dispatch receipt field.
- [x] Extend source guards for native/Web Preview dispatch output and bridge fallback.
- [x] Document that no native writer dispatch occurs until mutation capability and runtime proof are authorized.

### Task 4: Verification

- [x] Run targeted `rustfmt --check`.
- [x] Run the fixture sync check.
- [x] Run focused Node source guards.
- [x] Run `git diff --check` and conflict-marker scan.
- [x] Commit with `feat: add DX Style native writer dispatch gate`.
