# DX Style Native Editor Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Strengthen trusted DX Style source-apply review by binding each generator session to same-session native editor and buffer identity evidence.

**Architecture:** Keep Web Preview as the visual generator host and keep source mutation disabled. When Zed opens a DX Style generator from an active editor context, native Web Preview capture revalidates the active editor immediately, stores editor/buffer/worktree identity beside the source identity, and later requires the same native editor identity before matched review evidence can be preserved.

**Tech Stack:** Rust, GPUI Web Preview, Zed Workspace/Editor item APIs, DX Style source-owned contracts, source-only Node guards.

---

## Step-Back Checkpoint

**Current honest score:** 88/100 for the source-only integration lane. The previous checkpoint requires request/session/live source path, span, length, and digest agreement, but the trusted session source is still derived from active-context JSON and then matched by path. The next useful source-only improvement is same-session native editor/buffer identity evidence.

**Chosen next course:** Add native editor identity as review evidence only. Do not enable source writes, do not run `just run`, do not run Cargo, and do not claim runtime/WebView proof.

**Result after implementation:** 89/100 source-only. The bridge is now tighter, but true production readiness still requires authorized live Zed/WebView proof and a mutation-capable editor write bridge.

## Task 1: Preserve Source-Owned Guard Language

**Files:**
- Inspect-only: DX Style source-owned source-apply contract files.
- Modify only if the existing umbrella guard cannot carry native editor identity evidence.

- [x] Do not add a second native identity guard; keep `native active editor source revalidation` as the source-owned umbrella guard.
- [x] Keep source mutation disabled.
- [x] Keep native editor identity nested under `native_active_editor_source_revalidation.session_source`.

## Task 2: Capture Native Editor Identity At Launch

**Files:**
- Modify: `crates/web_preview/src/web_preview_view.rs`

- [x] Add a focused native editor identity struct with editor entity id, workspace item id, active buffer entity id, active buffer remote id, multi-buffer entity id, worktree id, project path, and singleton buffer kind.
- [x] Parse the existing source identity from active-context JSON.
- [x] Revalidate that identity against the active singleton editor before storing it.
- [x] Store native editor identity inside the session source identity only when path, length, and digest match.

## Task 3: Enforce Native Editor Identity During Review

**Files:**
- Modify: `crates/web_preview/src/web_preview_view.rs`
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`

- [x] Refuse matched revalidation when the trusted session has no native editor identity.
- [x] Require the trusted session editor to still be open with the same editor, buffer, worktree, and project identity at review time; do not require it to own focus because the Web Preview generator runs in the side pane.
- [x] Compare live editor entity id, workspace item id, active buffer id, active buffer remote id, multi-buffer id, worktree id, and project path to the trusted session identity before source digest review.
- [x] Preserve native editor identity in `session_source` evidence.
- [x] Validate required native editor identity fields in source-apply review receipts.

## Task 4: Source Guards And Handoff Docs

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard native editor identity capture and review mismatch blockers.
- [x] Guard that the existing source-owned `native active editor source revalidation` contract surface remains the umbrella guard.
- [x] Record the new honest source-only score.
- [x] Record that source mutation, build proof, WebView proof, and runtime proof remain unproven.

## Verification

- [x] `rustfmt --edition 2024 --check` on touched Rust files.
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`.
- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
