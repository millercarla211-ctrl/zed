# DX Style Preflight Provenance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the DX Style editor write-bridge preflight source auditable in Zed so the native panel and JSON snapshot show whether evidence came from the live DX Style fixture, the generated checked-in mirror, or the fail-closed emergency path.

**Architecture:** The write bridge now resolves preflight evidence through three layers: live `G:\Dx\style` fixture, generated Zed mirror, then emergency fallback. The snapshot should carry that provenance without enabling mutation, and the GPUI shell should surface it as a compact status row.

**Tech Stack:** Rust, GPUI panel snapshot, source-only Node guards.

---

### Task 1: Snapshot Provenance

**Files:**
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\editor_write_bridge.rs`

- [x] Add resolved preflight source and source detail fields.
- [x] Preserve the existing live/generated/emergency resolution order.
- [x] Keep `can_apply=false` and `can_mutate_source=false`.

### Task 2: Native Panel Surface

**Files:**
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\panel_view.rs`

- [x] Show compact preflight provenance beside the existing bridge summary.
- [x] Avoid adding fake controls or mutation affordances.

### Task 3: Guards, Docs, Verification

**Files:**
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Guard the source fields, generated mirror provenance, and panel metric.
- [x] Document source-only behavior and keep runtime proof unclaimed.
- [x] Run targeted rustfmt check, focused Node source guards, `git diff --check`, conflict-marker scan, and commit.
