# DX Style Native Writer Dry-Run Replay Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add review-only native writer dry-run replay evidence so DX Style source apply can prove what a future mutation-capable writer would do without mutating editor buffers or files.

**Architecture:** Web Preview still sends a review-only `dx-style-source-apply` request. Zed revalidates the live editor source, replays the trusted dry-run edit preview against that in-memory source, records before/after digest evidence, and keeps `source_mutation_enabled=false`, `can_apply=false`, and native mutation disabled.

**Tech Stack:** Rust, serde JSON fixtures, Zed Web Preview IPC review path, source-only Node guards, no Cargo/build/runtime commands.

---

## Step-Back Checkpoint

**Current honest score:** 89/100 for the source-only integration lane. The write bridge now names required source-apply receipt fields and runtime proofs, including native writer dry-run replay. The remaining source-only gap is that native review does not yet emit a replay receipt for that future writer proof.

**Chosen next course:** Add a bounded replay receipt and readiness gate. Do not enable Apply, do not mutate source, do not run `just run`, and do not run Cargo.

## Task 1: Source-Owned Contract

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_source_apply.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-source-apply-contract.json`
- Modify: `crates/web_preview/src/dx_style_generator_surface/source-apply-contract.generated.json`

- [x] Add `native writer dry-run replay` as a required editor guard.
- [x] Add `native_writer_dry_run_replay` as a review receipt field.
- [x] Keep the contract review-only and mutation disabled.
- [x] Sync the Zed embedded source-apply contract mirror from the DX Style fixture.

## Task 2: Native Replay Evidence

**Files:**
- Modify: `crates/web_preview/src/web_preview_view.rs`
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`

- [x] During active editor source revalidation, replay the trusted structured edit preview against the live source text in memory only.
- [x] Record status, source path, source span, edit span, before digest, after digest, before length, after length, replacement byte count, and `mutation_performed=false`.
- [x] Validate and preserve that receipt in native source-apply review.
- [x] Add a fail-closed source-write readiness blocker when replay evidence is absent or not matched.

## Task 3: Source Guards And Handoff Docs

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard the source-owned contract and generated mirror fields.
- [x] Guard the native Web Preview replay injection and native source-apply validation.
- [x] Record that this is still source-only replay evidence, not live mutation proof.

## Verification

- [x] `rustfmt --edition 2024 --check` on touched Rust files.
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`.
- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
