# DX Style Active Editor Source Revalidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add source-only native active-editor source revalidation to DX Style source-apply review without enabling source mutation.

**Architecture:** Web Preview remains the visual generator host and source mutation remains disabled. When a trusted `dx-style-source-apply` IPC reaches native review, `WebPreviewView` injects a native revalidation block by resolving the requested source path to an open editor buffer and checking live source path, span, length, and `fnv1a64:` digest before `dx_style_source_apply` builds the receipt.

**Tech Stack:** Rust, GPUI Workspace/Editor items, Zed Web Preview IPC, DX Style source-owned contracts, source-only Node guards.

---

## Step-Back Checkpoint

**Current honest score:** 82/100 before this slice. The review bridge has UUID sessions, display-URL redaction, split-token reset, digest shape/parity checks, source length bounds, and IPC byte caps. The biggest source-only gap is that native review still lacks a live editor-buffer revalidation proof.

**Chosen next course:** Add a bounded native source revalidation receipt field. Do not enable source writes, do not run `just run`, do not run Cargo, and do not claim runtime/WebView proof.

**Checkpoint score after source-only verification:** 86/100 for the source-only integration lane. True 100/100 production readiness still requires authorized runtime/WebView/build proof and a mutation-capable editor write bridge.

## Task 1: Extend The DX Style Source-Owned Contract

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_source_apply.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-source-apply-contract.json`
- Modify: `crates/web_preview/src/dx_style_generator_surface/source-apply-contract.generated.json`

- [x] Add `native active editor source revalidation` to `required_editor_guards`.
- [x] Add `native_active_editor_source_revalidation` to `review_receipt_fields`.
- [x] Keep `source_mutation_enabled` false.
- [x] Keep all existing fields and limits intact.

## Task 2: Revalidate Against Open Editor Buffers

**Files:**
- Modify: `crates/web_preview/src/web_preview_view.rs`
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`

- [x] Add a `zed.web_preview.dx_style.active_editor_source_revalidation` schema constant.
- [x] Add a small native `fnv1a64:` digest helper in `dx_style_source_apply`.
- [x] Before calling `source_apply_review_receipt`, clone the IPC payload and insert `request.native_active_editor_source_revalidation`.
- [x] Resolve the requested source path to an open singleton `Editor` item by scanning `workspace.items_of_type::<Editor>(cx)`.
- [x] Compare the editor's absolute project path to the request source path.
- [x] Refuse revalidation if the matching editor is missing, not a singleton, too large, has a mismatched span, has a mismatched source length, or has a mismatched digest.
- [x] Preserve review-only behavior even when revalidation matches.

## Task 3: Enforce Revalidation In The Review Receipt

**Files:**
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`
- Modify: `script/dx-style-panel-source.test.ts`

- [x] Validate the revalidation schema.
- [x] Require revalidation `status` to be `matched`.
- [x] Require the native revalidation source path, span, and digest to match the request/context identity.
- [x] Preserve the revalidation block in the native review receipt.
- [x] Remove the old permanent blocker that said native active editor source revalidation was not performed.

## Task 4: Handoff Docs

**Files:**
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Record that active-editor source revalidation is now source-wired.
- [x] Record that source mutation remains disabled.
- [x] Record that this is source-only proof; no `just run`, Cargo, server, browser, or live WebView validation is claimed.
- [x] Update the honest score only after source-only checks pass.

## Verification

- [x] `rustfmt --edition 2024 --check` on touched Rust files.
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`.
- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
