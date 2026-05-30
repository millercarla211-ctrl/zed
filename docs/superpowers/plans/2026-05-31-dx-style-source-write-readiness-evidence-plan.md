# DX Style Source Write Readiness Evidence Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add machine-readable source-write readiness evidence that explains why DX Style source mutation remains disabled and what must become true before a real writer can run.

**Architecture:** DX Style owns the review receipt field in the source-apply contract. Web Preview and native Zed both emit a whitelisted readiness object from existing gate state, editor bridge state, handler capability state, and source mutation contract state. The object is evidence only: it must not enable `Apply`, create a writer, or mutate files.

**Tech Stack:** Rust, serde JSON fixtures, embedded Web Preview browser script, Zed source guard tests, source-only Node checks.

---

## Step-Back Checkpoint

**Current honest score:** 89/100 for the source-only integration lane. The editor write-bridge preflight now names the right prerequisites, but review receipts and copied Web Preview packets still force readers to infer writer readiness from scattered fields.

**Chosen next course:** Add an explicit `source_write_readiness` block to contract review evidence. Keep it fail-closed, keep `safe_to_mutate=false` in current source-only state, and do not run runtime/build/WebView proof.

## Task 1: Source-Owned Receipt Field

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_source_apply.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-source-apply-contract.json`
- Modify: `crates/web_preview/src/dx_style_generator_surface/source-apply-contract.generated.json`

- [x] Add `source_write_readiness` to the source-owned `review_receipt_fields`.
- [x] Mirror the updated source-apply contract fixture into Zed using the existing fixture sync script.
- [x] Keep `source_mutation_enabled=false`.

## Task 2: Native Review Receipt Evidence

**Files:**
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`

- [x] Add a bounded `source_write_readiness` JSON object to native review receipts.
- [x] Include `status`, `safe_to_mutate`, source mutation contract state, apply gate readiness, trusted receipt state, editor write bridge state, Web Preview mutation declaration, native writer mutation capability, and missing requirement codes.
- [x] Add a refusal-safe readiness block to untrusted-session receipts.
- [x] Keep native writer capability `can_mutate_source=false`.

## Task 3: Web Preview Copied Packet Evidence

**Files:**
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`

- [x] Add a whitelisted `source_write_readiness` object to copied review packets.
- [x] Show a compact read-only write-readiness summary in the source apply contract review area.
- [x] Keep `Apply` blocked through existing source-apply blockers.

## Task 4: Source Guards And Handoff Docs

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard the contract field, native receipt block, browser review packet block, and disabled mutation boundary.
- [x] Record the checkpoint as source-only readiness evidence.
- [x] Record that runtime/WebView/build proof and mutation-capable writer proof remain unproven.

## Verification

- [x] `rustfmt --edition 2024 --check` on touched Rust files.
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`.
- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
