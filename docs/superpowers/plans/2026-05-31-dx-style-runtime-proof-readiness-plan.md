# DX Style Runtime Proof Readiness Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make source-write readiness check the exact runtime proof names advertised by the DX Style editor write-bridge preflight.

**Architecture:** DX Style remains source-owned for the required proof list. Zed and Web Preview continue to keep mutation disabled, but readiness now distinguishes missing native writer replay proof from missing post-write digest proof instead of treating any runtime proof list as sufficient.

**Tech Stack:** Rust, embedded Web Preview JavaScript, source-only Node guards, no Cargo/build/runtime commands.

---

## Task 1: Web Preview Readiness

**Files:**
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`

- [x] Require `successful native writer dry-run replay` in `bridge.required_runtime_proofs` when runtime validation is required.
- [x] Require `post-write source digest verification` in `bridge.required_runtime_proofs` when runtime validation is required.
- [x] Keep `runtime_webview_build_proof_missing` and mutation-disabled behavior unchanged.

## Task 2: Native Readiness

**Files:**
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`

- [x] Require `successful native writer dry-run replay` in the native source-write readiness bridge snapshot.
- [x] Keep the existing `post-write source digest verification` requirement, but report a specific missing-code for digest proof drift.
- [x] Preserve `safe_to_mutate=false` while the contract, bridge, Web Preview handler, and native writer are still review-only.

## Task 3: Guards And Handoff

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard both specific runtime proof blocker codes in Web Preview and native readiness.
- [x] Record the source-only checkpoint and that runtime/WebView/build proof remains unrun.

## Verification

- [x] `rustfmt --edition 2024 --check crates\web_preview\src\dx_style_source_apply.rs`
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`
- [x] `git diff --check`
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
