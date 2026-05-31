# DX Style Dispatch Proof Revalidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Require native DX Style writer dispatch to revalidate runtime proof and explicit mutation intent directly before any future editor transaction.

**Architecture:** Source-write readiness and mutation preflight already summarize many gates. The dispatch boundary should still independently inspect the runtime validation receipt and user apply action so a future bug or forged readiness packet cannot skip the last proof check.

**Tech Stack:** Rust, Zed Web Preview native dispatcher, DX Style source-owned bridge contract, Node source guards.

---

### Task 1: Source-Owned Bridge Contract

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-editor-write-bridge-preflight.json`
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\editor_write_bridge.rs`

- [x] Add dispatch-time runtime validation receipt revalidation as a required editor guard.
- [x] Add dispatch-time explicit mutation action revalidation as a required editor guard.
- [x] Keep mutation disabled and preserve existing source-owned fixture shape.

### Task 2: Native Dispatch Guard

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\web_preview_view.rs`

- [x] Block dispatch when runtime validation receipt schema/status is not validated.
- [x] Block dispatch when authorized runtime validation, WebView review round trip, native dry-run replay, or post-write digest proof is missing.
- [x] Block dispatch unless the user action is `mutate_source_confirmed`.
- [x] Cross-check runtime validation source path and digest/readback evidence against the native dry-run replay.

### Task 3: Source Guards, Handoff, Verification

**Files:**
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Guard the new bridge contract strings and final dispatch status names.
- [x] Document the source-only checkpoint and keep runtime proof unclaimed.
- [x] Run targeted rustfmt check, focused Node source guards, `git diff --check`, conflict-marker scan, and commit.
