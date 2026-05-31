# DX Style Mutation Write Template Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Emit a review-only mutation write receipt template without enabling editor source mutation.

**Architecture:** DX Style source-owned contracts require the template field. Zed native review and Web Preview copied review packets emit the template from existing commit/runtime proof inputs, while readiness remains blocked until a real mutation-capable writer and runtime proof exist.

**Tech Stack:** Rust, Zed Web Preview JavaScript, Node source guards.

---

### Task 1: Source-Owned Requirement

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_source_apply.rs`
- Modify: `G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-source-apply-contract.json`
- Modify: `G:\Dx\style\fixtures\grouped-class-editor-write-bridge-preflight.json`

- [ ] Add `mutation_write_receipt_template` to source-apply review fields.
- [ ] Require the same field from the editor write bridge.
- [ ] Keep mutation disabled.

### Task 2: Zed Template Emission

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_source_apply.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\script.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\web_preview_view.rs`

- [ ] Emit a native review-only mutation write receipt template.
- [ ] Emit a Web Preview copied template marked `not_performed_in_web_preview`.
- [ ] Include the template in latest source-apply receipt summaries.

### Task 3: Readiness And Guards

**Files:**
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\editor_write_bridge.rs`
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [ ] Add fallback bridge review field support.
- [ ] Add readiness blockers for missing mutation template evidence.
- [ ] Update source guards and handoff docs.

### Task 4: Verification

- [ ] Run targeted `rustfmt --check`.
- [ ] Run the fixture sync check.
- [ ] Run the focused Node source guards.
- [ ] Run `git diff --check` and conflict-marker scan.
- [ ] Commit with `feat: add DX Style mutation write template`.
