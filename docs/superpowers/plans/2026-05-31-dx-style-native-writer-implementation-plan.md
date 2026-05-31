# DX Style Native Writer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Install the real native editor transaction dispatcher behind the existing DX Style source-write gates without enabling mutation under the current review-only contracts.

**Architecture:** The source-apply receipt remains review-first. Zed Web Preview receives the review receipt, then runs a native dispatch gate that refuses unless source-write readiness, native mutation preflight, native mutation capability, trusted source identity, digest checks, and an editor transaction path are all valid. Current contracts keep this blocked; the important improvement is that the future writer path is real and auditable, not an imaginary next step.

**Tech Stack:** Rust, Zed Editor/GPUI, Web Preview IPC receipts, Node source guards.

---

### Task 1: Native Dispatch Path

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\web_preview_view.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_source_apply.rs`

- [x] Add a native writer dispatcher that parses the source-apply receipt and refuses unless all mutation gates are true.
- [x] When gates pass, locate the original singleton editor by same-session native editor identity, verify source path/length/digest/span, apply one editor transaction, and verify post-write digest.
- [x] Under current contracts, keep `writer_invoked=false` and `mutation_performed=false`.
- [x] Keep source-apply pure review receipts honest by saying the native dispatcher is required instead of pretending the writer is absent forever.

### Task 2: Source Guards

**Files:**
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`

- [x] Guard the dispatcher function, editor transaction call, `MultiBufferOffset` byte-range edit, before/after digest checks, and post-write readback receipt fields.
- [x] Guard that the source-only path still refuses current mutation.

### Task 3: Handoff

**Files:**
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Document that the native writer implementation path now exists but remains disabled by source-owned mutation/runtime gates.
- [x] Keep 100/100 unclaimed until authorized runtime/WebView/build proof runs.

### Task 4: Verification

- [x] Run targeted `rustfmt --check`.
- [x] Run focused Node source guards.
- [x] Run `git diff --check` and conflict-marker scan.
- [x] Commit with `feat: add DX Style native writer implementation gate`.
