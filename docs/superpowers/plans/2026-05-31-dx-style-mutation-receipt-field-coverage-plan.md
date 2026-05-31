# DX Style Mutation Receipt Field Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure the actual native DX Style mutation write receipt covers every source-owned required mutation receipt field before any future dispatch can be trusted.

**Architecture:** Source-write readiness already exposes the editor write bridge's required mutation write receipt fields. The native WebPreviewView dispatcher is the only place a real mutation write receipt can exist, so it must compare the dispatched receipt against that field list, report missing actual fields, and fail closed if the receipt shape drifts. Current contracts still keep dispatch blocked before mutation.

**Tech Stack:** Rust, Zed Web Preview native dispatcher, Node source guards.

---

### Task 1: Native Dispatch Receipt Coverage

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\web_preview_view.rs`

- [x] Read `source_write_readiness.required_mutation_write_receipt_fields` from the trusted source-apply receipt.
- [x] After constructing the mutation write receipt, compare actual fields against the required field list.
- [x] Emit `missing_mutation_write_receipt_fields` and `mutation_write_receipt_field_coverage_complete`.
- [x] If any required actual field is missing, return a fail-closed dispatch status instead of `dispatched`.

### Task 2: Source Guards And Handoff

**Files:**
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Guard the required-field extraction and missing-field fail-closed status.
- [x] Document that actual mutation receipt field coverage is checked but runtime proof remains unverified.
- [x] Run targeted rustfmt check, focused Node source guards, `git diff --check`, conflict-marker scan, and commit.
