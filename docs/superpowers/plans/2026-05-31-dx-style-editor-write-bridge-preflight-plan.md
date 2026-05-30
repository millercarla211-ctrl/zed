# DX Style Editor Write Bridge Preflight Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align the DX Style source-owned editor write-bridge preflight with Zed's current trusted review evidence without enabling source mutation.

**Architecture:** DX Style owns the future writer checklist in `grouped_class_editor_write_bridge.rs` and its fixture. Zed reads that checklist as review evidence through the Style panel apply gate and Web Preview generator context, but the native writer remains disabled until runtime validation is authorized and a mutation-capable bridge exists.

**Tech Stack:** Rust, serde JSON fixtures, GPUI Style panel source guard tests, Web Preview review metadata, source-only Node verification.

---

## Step-Back Checkpoint

**Current honest score:** 89/100 for the source-only integration lane. Zed now validates trusted review packets against session source identity, native editor identity, active path/span/source length/digest, and cursor-scoped dry-run edit previews. The source-owned write-bridge preflight still lists an older loose guard, so it no longer describes the actual future mutation checklist precisely enough.

**Chosen next course:** Tighten the write-bridge preflight contract and Zed fallback/readiness evidence. Do not enable `can_mutate_source`, do not enable `can_apply`, do not add a native writer, do not run `just run`, and do not run Cargo.

## Task 1: Source-Owned Preflight Checklist

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-editor-write-bridge-preflight.json`

- [x] Replace the loose `bounded edit preview review` guard with the precise `cursor-scoped dry-run structured edit preview` guard.
- [x] Add session/source/native identity guards already enforced by native review: trusted Web Preview session, session-bound source identity, native active editor source revalidation, same-session native editor identity, and active source length match.
- [x] Add explicit receipt requirements for `dx.style.grouped-class-source-apply-contract`, `zed.web_preview.dx_style_source_apply_receipt.v1`, and `zed.web_preview.dx_style.active_editor_source_revalidation`.
- [x] Keep `status=not_enabled`, `can_mutate_source=false`, and `runtime_validation_required=true`.

## Task 2: Zed Fallback And Review Evidence

**Files:**
- Modify: `crates/agent_ui/src/dx_style_panel/editor_write_bridge.rs`
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`

- [x] Mirror the tightened fallback guard list when the DX Style fixture is unavailable.
- [x] Add source-owned preflight state, receipt count, guard count, native handler count, and runtime-validation status into the copied Web Preview review packet.
- [x] Keep Web Preview rendering read-only and keep `Apply` blocked through `editor_write_bridge_not_ready`.

## Task 3: Source Guards And Handoff Docs

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard the new DX Style fixture fields and the Zed fallback guard list.
- [x] Guard the copied Web Preview review packet preflight evidence.
- [x] Record the checkpoint as source-only preflight alignment.
- [x] Record that runtime proof, WebView proof, Cargo/build proof, and source mutation remain unproven.

## Verification

- [x] `rustfmt --edition 2024 --check` on touched Rust files.
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
