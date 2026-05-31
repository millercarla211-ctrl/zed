# DX Style Native Commit Plan Evidence

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add review-only native writer commit-plan evidence so DX Style/Zed can prove exactly what a future source mutation would need without enabling writes.

**Architecture:** DX Style owns the source-apply and editor write-bridge contract requirements. Zed mirrors those contracts, emits a native commit-plan receipt derived from the already trusted dry-run replay, and keeps mutation disabled until authorized runtime validation and a mutation-capable bridge exist.

**Tech Stack:** Rust, embedded Web Preview JavaScript, DX Style JSON fixtures, source-only Node guards. No Cargo/build/runtime/`just run` commands.

---

## Task 1: Source-Owned Contract

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_source_apply.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-source-apply-contract.json`
- Modify: `G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-editor-write-bridge-preflight.json`

- [x] Add `native writer commit plan` as a required source-apply/editor-bridge guard.
- [x] Add `native_writer_commit_plan` as a required source-apply/editor-bridge review receipt field.
- [x] Keep `source_mutation_enabled=false` and `can_mutate_source=false`.

## Task 2: Zed Native Evidence

**Files:**
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`
- Modify: `crates/agent_ui/src/dx_style_panel/editor_write_bridge.rs`

- [x] Emit `native_writer_commit_plan` in native source-apply review receipts.
- [x] Keep the plan review-only, redacted, digest-bound, and mutation-disabled.
- [x] Add readiness blockers when the commit plan or source-owned bridge field is missing.
- [x] Update the fallback editor write-bridge preflight mirror.

## Task 3: Guards And Handoff

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard source-owned contract/fixture drift.
- [x] Guard Zed Web Preview/native readiness blocker drift.
- [x] Record source-only verification and the remaining runtime/build boundary.

## Verification

- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --write`
- [x] `rustfmt --edition 2024 --check crates\web_preview\src\dx_style_source_apply.rs crates\agent_ui\src\dx_style_panel\editor_write_bridge.rs crates\web_preview\src\web_preview_view.rs G:\Dx\style\src\core\engine\grouped_class_source_apply.rs G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs`
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`
- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`
- [x] `git diff --check`
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
