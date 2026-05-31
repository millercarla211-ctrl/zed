# DX Style Explicit User Apply Action Evidence

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the existing `explicit user apply action` bridge requirement into structured review evidence carried from Web Preview into native source-apply receipts.

**Architecture:** DX Style owns the source-apply/editor-write bridge requirement. Web Preview emits a bounded `user_apply_action` packet only from review/apply button handlers. Native review validates and preserves the packet while keeping mutation disabled.

**Tech Stack:** Rust, embedded Web Preview JavaScript, DX Style JSON fixtures, source-only Node guards. No Cargo/build/runtime/`just run` commands.

---

## Task 1: Source-Owned Contract

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_source_apply.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-source-apply-contract.json`
- Modify: `G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-editor-write-bridge-preflight.json`

- [x] Add `user_apply_action` as a required source-apply/editor-bridge review receipt field.
- [x] Preserve `explicit user apply action` as a guard.
- [x] Keep mutation disabled.

## Task 2: Web Preview And Native Review

**Files:**
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`
- Modify: `crates/agent_ui/src/dx_style_panel/editor_write_bridge.rs`

- [x] Emit a structured review action packet from the Review Source button.
- [x] Emit a structured mutation action packet from the disabled future Apply path without enabling it.
- [x] Validate and preserve the action packet in native source-apply review receipts.
- [x] Gate future readiness on action evidence and the source-owned bridge field.

## Task 3: Guards And Handoff

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard action packet wiring and native receipt validation.
- [x] Record source-only verification and the remaining runtime/build boundary.

## Verification

- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --write`
- [x] `rustfmt --edition 2024 --check crates\web_preview\src\dx_style_source_apply.rs crates\agent_ui\src\dx_style_panel\editor_write_bridge.rs crates\web_preview\src\web_preview_view.rs G:\Dx\style\src\core\engine\grouped_class_source_apply.rs G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs`
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`
- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`
- [x] `git diff --check`
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
