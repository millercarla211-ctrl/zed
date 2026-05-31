# DX Style Mutation Writer Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Define the source-owned mutation writer receipt contract without enabling mutation.

**Architecture:** DX Style owns the disabled editor write-bridge preflight. Zed reads that preflight into the Style panel and Web Preview review packets, while native source-write readiness fails closed on unsupported schema or receipt-field drift.

**Tech Stack:** Rust, GPUI/Zed source inspection, embedded Web Preview JavaScript, Node source guards.

---

### Task 1: Source-Owned Contract

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-editor-write-bridge-preflight.json`

- [ ] Add `mutation_write_receipt_schema`.
- [ ] Add `required_mutation_write_receipt_fields`.
- [ ] Keep `can_mutate_source=false`.
- [ ] Assert the schema and a post-write digest match field in the local Rust contract tests.

### Task 2: Zed Bridge Snapshot

**Files:**
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\editor_write_bridge.rs`

- [ ] Read the mutation write receipt schema and required fields from the source-owned fixture.
- [ ] Include both in `StyleEditorWriteBridgeSnapshot::to_json`.
- [ ] Add a fallback list that matches DX Style when the source fixture cannot be read.

### Task 3: Native And Web Preview Readiness

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_source_apply.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\script.rs`

- [ ] Add a known mutation write receipt schema and field list.
- [ ] Report named drift blockers when the bridge schema is missing or the required field list contains unsupported fields.
- [ ] Surface the schema and missing field list in native/Web Preview readiness packets.

### Task 4: Guards And Handoff

**Files:**
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [ ] Extend source guards for the new source-owned schema and fields.
- [ ] Update handoff docs to 98/100 source-only readiness.
- [ ] Keep the final boundary honest: runtime proof and mutation-capable writer are still unproven.

### Task 5: Verification And Commit

**Files:**
- Commit only the Zed mirror/code/docs changes.

- [ ] Run targeted `rustfmt --check`.
- [ ] Run the DX Style fixture sync check.
- [ ] Run the focused Node source guards.
- [ ] Run `git diff --check` and a conflict-marker scan.
- [ ] Commit with `feat: define DX Style mutation writer contract`.
