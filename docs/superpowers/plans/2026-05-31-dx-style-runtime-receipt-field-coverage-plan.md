# DX Style Runtime Receipt Field Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verify that the actual DX Style runtime validation receipt carries every source-owned required field before any future source write can graduate.

**Architecture:** The editor write bridge already publishes required runtime validation receipt fields. Native review and Web Preview copied packets must compare those requirements against the actual emitted runtime validation receipt, report missing fields, and keep readiness blocked on drift. This remains source-only and fail-closed until authorized runtime proof exists.

**Tech Stack:** Rust, embedded Web Preview JavaScript, Node source guards.

---

### Task 1: Native Field Coverage

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_source_apply.rs`

- [x] Add native helper to compare the actual `runtime_validation_receipt` against `required_runtime_validation_receipt_fields`.
- [x] Feed missing actual receipt fields into `source_write_readiness`.
- [x] Add a named readiness blocker for missing actual receipt fields.
- [x] Include the missing actual field list in readiness evidence.

### Task 2: Web Preview Field Coverage

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\script.rs`

- [x] Compare the copied `runtimeValidationReceiptPacket` against bridge required fields.
- [x] Add the same named readiness blocker and missing-field evidence in Web Preview readiness.

### Task 3: Source Guards And Handoff

**Files:**
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Guard native and Web Preview runtime receipt field coverage.
- [x] Document that actual receipt field coverage is checked while runtime proof remains unverified.
- [x] Run targeted rustfmt checks, focused Node source guards, `git diff --check`, conflict-marker scan, and commit.
