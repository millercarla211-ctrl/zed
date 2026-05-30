# DX Style Source Apply Session Hardening Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make DX Style Web Preview source-apply review sessions harder to replay or pressure without enabling source mutation.

**Architecture:** Keep source apply review-only. Replace predictable session-token material with UUID-backed tokens, consume the active token only after a trusted source-apply IPC reaches native review, avoid leaking the token through visible Web Preview URLs, and add aggregate queued-byte limits beside the existing per-message and queue-count limits.

**Tech Stack:** Rust, GPUI Web Preview, Zed source guards, DX Style Web Preview contracts.

---

## Step-Back Checkpoint

**Current honest score:** 82/100 after this source-only slice. The source-only DX Style/Zed architecture is real and the review bridge is materially harder to replay or spoof, but true 100/100 still requires authorized WebView/runtime/build proof and a mutation-capable editor write bridge.

**Chosen next course:** Harden the trust envelope around the review-only bridge. Do not add mutation. Do not broaden into runtime validation. Keep the patch small enough that source review can confidently reason about behavior.

## Task 1: Harden Session Token Lifecycle

**Files:**
- Modify: `crates/web_preview/Cargo.toml`
- Modify: `crates/web_preview/src/web_preview_view.rs`
- Modify: `script/dx-style-panel-source.test.ts`

- [x] Add `uuid.workspace = true` to the Windows Web Preview dependency set.
- [x] Generate DX Style source-apply session tokens from `uuid::Uuid::new_v4()` plus existing session context instead of sequence/time-only material.
- [x] Clear the active DX Style source-apply session token after a trusted `dx-style-source-apply` review receipt.
- [x] Keep refused/untrusted packets from burning the active trusted session token.
- [x] Keep normal navigation clearing the token.
- [x] Keep the token out of the visible active URL by showing a stable `zed://dx-style/generator` display URL for the loaded generator data URL.
- [x] Prevent split panes from inheriting a live source-apply session token.
- [x] Add source guards proving UUID-backed generation, trusted-token consumption, refused-token preservation, display-URL redaction, and split-token reset.

## Task 2: Cap Aggregate Queued IPC Bytes

**Files:**
- Modify: `crates/web_preview/src/web_preview_view.rs`
- Modify: `script/dx-style-panel-source.test.ts`

- [x] Add an aggregate deferred IPC byte limit constant.
- [x] Apply that limit when queueing messages from native deferred IPC.
- [x] Apply the same limit when queueing browser IPC events before they move into the deferred queue.
- [x] Keep the existing per-message byte limit and message-count limit.
- [x] Add source guards for the aggregate limit helper and both queueing call sites.

## Task 3: Strengthen Source Identity Review

**Files:**
- Modify: `crates/agent_ui/src/dx_style_panel/active_context.rs`
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`
- Modify: `script/dx-style-panel-source.test.ts`

- [x] Carry active source byte length beside the existing `fnv1a64:` digest in Style active-context JSON.
- [x] Forward the context digest as a top-level source-apply request field from the Web Preview generator script.
- [x] Require complete `fnv1a64:` digest shape and request/context digest parity during native review.
- [x] Refuse spans exceeding the active context source length.
- [x] Keep native review blocked until active editor source revalidation exists.
- [x] Add source guards for the new digest, source-length, and apply-gate consistency checks.

## Task 4: Handoff Docs

**Files:**
- Modify: `todo.txt`
- Modify: `changelog.txt`
- Modify: `DX.md`

- [x] Record that source mutation remains disabled.
- [x] Record that this is source-only proof; no `just run`, Cargo, server, browser, or WebView validation is claimed.
- [x] Update the honest score only if source-only checks pass.

## Verification

- [x] Targeted Rust formatting.
- [x] DX Style source guard.
- [x] Combined source guard batch.
- [x] Fixture mirror check.
- [x] `rustfmt --edition 2024 --check` on touched Rust files.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [x] Focused commit.
