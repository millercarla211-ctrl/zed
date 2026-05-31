# DX Style Dispatch Success Invariant Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make native DX Style writer dispatch report success only when a real editor transaction, post-write digest readback, and mutation receipt field coverage all pass.

**Architecture:** WebPreviewView already computes transaction id, post-write digest readback, and missing mutation receipt fields. The status selection must use all three pieces of evidence so future authorized mutation cannot report `dispatched` from digest match alone.

**Tech Stack:** Rust, Zed Web Preview native dispatcher, Node source guards.

---

### Task 1: Dispatch Status Semantics

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\web_preview_view.rs`

- [x] Compute `single_editor_transaction` and `mutation_performed` before status selection.
- [x] Return `failed_no_editor_transaction` when no editor transaction id exists.
- [x] Return `failed_post_write_digest_mismatch` when post-write readback digest does not match.
- [x] Return `failed_mutation_write_receipt_fields_missing` when required mutation receipt fields are absent.
- [x] Return `dispatched` only after all success evidence is present.

### Task 2: Source Guards And Handoff

**Files:**
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Guard the success invariant and failure statuses.
- [x] Document the stricter dispatch success semantics.
- [x] Run targeted rustfmt check, focused Node source guards, `git diff --check`, conflict-marker scan, and commit.
