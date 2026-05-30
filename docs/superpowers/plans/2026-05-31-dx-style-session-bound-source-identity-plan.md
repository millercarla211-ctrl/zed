# DX Style Session-Bound Source Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bind each trusted DX Style source-apply review to the source identity that opened that Web Preview generator session.

**Architecture:** The DX Style generator remains Web Preview-hosted and source mutation remains disabled. When Zed opens the generator from an editor context, `WebPreviewView` stores a bounded native session source identity beside the trusted session token, injects that identity into native revalidation, and refuses review if the request, session source, and live open editor source do not all match path, span, length, and digest.

**Tech Stack:** Rust, GPUI Web Preview, Zed Workspace/Editor items, DX Style source-owned contracts, source-only Node guards.

---

## Step-Back Checkpoint

**Current honest score:** 88/100 for the source-only integration lane after this checkpoint. Native review validates the request against the trusted generator session source and then against the live open editor source. True 100/100 still needs an authorized runtime/WebView/build proof window and the future mutation-capable editor write bridge.

**Chosen next course:** Store a session-bound source identity at generator launch and require it during review. Do not enable source writes, do not run `just run`, do not run Cargo, and do not claim runtime/WebView proof.

## Task 1: Extend The Source-Owned Contract

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_source_apply.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-source-apply-contract.json`
- Modify: `crates/web_preview/src/dx_style_generator_surface/source-apply-contract.generated.json`

- [x] Add `session-bound source identity` to `required_editor_guards`.
- [x] Keep the existing `native_active_editor_source_revalidation` receipt field instead of adding a second receipt surface.
- [x] Keep `source_mutation_enabled` false.

## Task 2: Store Session Source Identity

**Files:**
- Modify: `crates/web_preview/src/web_preview_view.rs`

- [x] Add a small `DxStyleSourceApplySessionSourceIdentity` struct that stores source path, digest, length, span, workspace root, and context kind from the launch context.
- [x] Parse session source identity from the bounded active-context JSON passed into `load_dx_style_generator_url`.
- [x] Reject malformed launch identity before storing it, including wrong schema, invalid digest, invalid span, oversized values, or source paths outside the workspace root.
- [x] Store the parsed source identity beside `dx_style_source_apply_session_token`.
- [x] Clear the stored source identity whenever the session token is cleared, navigation leaves the generator, or the view is split.

## Task 3: Enforce Session Source During Native Revalidation

**Files:**
- Modify: `crates/web_preview/src/web_preview_view.rs`
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`

- [x] Require session source identity before native revalidation can return `matched`.
- [x] Compare request path, digest, source length, and source span to the session source identity before scanning open editors.
- [x] Include the session source identity in `native_active_editor_source_revalidation`.
- [x] In `source_apply_review_receipt`, validate the nested session source identity against the request identity.
- [x] Preserve review-only behavior even when all identities match.

## Task 4: Source Guards And Handoff Docs

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard session-bound source storage and reset paths.
- [x] Guard request/session/live source comparisons and honest blocked evidence.
- [x] Record that source mutation remains disabled.
- [x] Record that this is source-only proof; no `just run`, Cargo, server, browser, live WebView, or mutation proof is claimed.

## Verification

- [x] `rustfmt --edition 2024 --check` on touched Rust files.
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`.
- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
