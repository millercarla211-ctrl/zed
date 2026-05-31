# DX Style Runtime Validation Receipt Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a source-owned runtime validation receipt gate so future DX Style source mutation cannot rely on template evidence alone.

**Architecture:** DX Style owns the contract field and bridge requirement. Zed native review and the Web Preview copied packet both emit an actual runtime validation receipt today, but it is fail-closed because no authorized runtime/WebView/build proof has been run. Source-write readiness and native mutation preflight must require that receipt before a future writer can dispatch.

**Tech Stack:** Rust, Zed Web Preview IPC receipts, DX Style source-owned fixtures, embedded Web Preview JavaScript, Node source guards.

---

### Task 1: DX Style Source-Owned Contract

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_source_apply.rs`
- Modify: `G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-source-apply-contract.json`
- Modify: `G:\Dx\style\fixtures\grouped-class-editor-write-bridge-preflight.json`

- [x] Add `runtime_validation_receipt` to source-apply review receipt fields.
- [x] Add `runtime_validation_receipt` to editor write-bridge required review receipt fields.
- [x] Add a runtime validation receipt verification guard while keeping `source_mutation_enabled=false` and `can_mutate_source=false`.

### Task 2: Zed Native Review

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_source_apply.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\web_preview_view.rs`

- [x] Emit a real `runtime_validation_receipt` object from native source-apply review.
- [x] Keep the receipt fail-closed with `authorized_runtime_validation=false`, no mutation, no post-write readback digest, and no verified timestamp.
- [x] Make source-write readiness and native mutation preflight require the actual receipt before future mutation.
- [x] Include the new receipt in latest review summaries.

### Task 3: Web Preview Copied Packet And Guards

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\script.rs`
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Generated mirror: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\source-apply-contract.generated.json`

- [x] Add Web Preview copied `runtime_validation_receipt` evidence as `not_performed_in_web_preview`.
- [x] Teach Web Preview readiness that missing `runtime_validation_receipt` is a hard blocker.
- [x] Extend source guards for the source-owned contract, native receipt, copied packet, readiness blocker, and generated mirror.

### Task 4: Handoff And Verification

**Files:**
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Document that the runtime validation receipt gate exists but remains unverified without authorized runtime/WebView/build proof.
- [x] Run targeted rustfmt checks, fixture sync check, focused Node source guards, `git diff --check`, and conflict-marker scan.
- [x] Commit with `feat: add DX Style runtime validation receipt gate`.
