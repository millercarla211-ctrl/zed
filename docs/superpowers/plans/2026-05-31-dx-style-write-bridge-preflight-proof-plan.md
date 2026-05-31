# DX Style Write Bridge Preflight Proof Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the DX Style editor write-bridge preflight name the exact source-apply review receipt fields and runtime proofs required before any future mutation-capable editor writer can be trusted.

**Architecture:** DX Style remains the source of truth for the write-bridge preflight. Zed reads that fixture into the native Style panel and Web Preview review packets, exposes the missing proof requirements for handoff, and keeps `can_apply`, `can_mutate_source`, and source mutation disabled.

**Tech Stack:** Rust, serde JSON fixtures, Web Preview generator browser script, source-only Node guards, no runtime/build commands.

---

## Step-Back Checkpoint

**Current honest score:** 89/100 for the source-only DX Style/Zed integration lane. The editor/source identity, session token, dry-run edit preview, reverse-delta replacement diagnostics, and source-write readiness paths are source-wired. The remaining source-only weakness is that the write-bridge preflight does not yet explicitly require the receipt fields and runtime proofs that a mutation-capable writer must show.

**Chosen next course:** Add the proof lists to DX Style, surface them through Zed and Web Preview, guard them with source tests, and keep source mutation disabled until the user authorizes runtime/WebView/build validation and a real mutation-capable writer.

## Task 1: Source-Owned Preflight Proof Lists

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-editor-write-bridge-preflight.json`

- [x] Add `required_source_apply_review_receipt_fields` to the preflight model and fixture.
- [x] Include the replacement payload diagnostics receipt field so reverse-delta replacement blockers cannot be dropped before writes.
- [x] Add `required_runtime_proofs`, including WebView review round trip, native writer dry-run replay, and post-write source digest verification.
- [x] Keep `status=not_enabled`, `can_mutate_source=false`, and `runtime_validation_required=true`.

## Task 2: Zed Bridge And Web Preview Evidence

**Files:**
- Modify: `crates/agent_ui/src/dx_style_panel/editor_write_bridge.rs`
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`

- [x] Read the new proof lists from the source-owned preflight fixture with bounded list parsing.
- [x] Expose proof-list counts and values in copied Web Preview review packets.
- [x] Display the required receipt fields and runtime proofs in the Web Preview bridge review.
- [x] Preserve those lists in native source-apply review receipts and source-write readiness evidence.
- [x] Keep mutation fail-closed when the bridge is not mutation-capable.

## Task 3: Source Guards And Handoff Docs

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard the DX Style source and fixture proof lists.
- [x] Guard the Zed bridge snapshot fallback and Web Preview packet/rendering evidence.
- [x] Record the checkpoint in DX docs, todo, and changelog.
- [x] Run the allowed source-only checks and commit the lane slice.

## Verification

- [x] `rustfmt --edition 2024 --check` on touched Rust files.
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`.
- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
