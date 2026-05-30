# DX Style Dry-Run Edit Review Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Strengthen DX Style source-apply review by proving the trusted dry-run receipt carries a bounded structured edit preview for the active cursor source span.

**Architecture:** Keep Web Preview as the visual generator host and keep source mutation disabled. DX Style owns the source-apply contract and edit-preview caps; Zed mirrors those caps, forwards bounded structured edit-preview data through Web Preview, and native review records a separate dry-run edit review block before any future writer can trust the receipt.

**Tech Stack:** Rust, GPUI Web Preview, DX Style source-owned fixtures, Zed source guards, source-only Node checks.

---

## Step-Back Checkpoint

**Current honest score:** 89/100 for the source-only integration lane. Native review now binds trusted generator sessions to source identity and same-session editor/buffer identity, but the dry-run edit preview is still mostly displayed as review metadata rather than validated as its own cursor-scoped native receipt block.

**Chosen next course:** Add source-owned caps and a review-only native `dry_run_edit_review` block. Do not enable source writes, do not run `just run`, do not run Cargo, and do not claim runtime/WebView proof.

## Task 1: Source-Owned Contract Caps

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_source_apply.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-source-apply-contract.json`
- Modify: `crates/web_preview/src/dx_style_generator_surface/source-apply-contract.generated.json`
- Modify: `crates/web_preview/src/dx_style_generator_surface/source_apply_contract.rs`

- [x] Add a required editor guard named `cursor-scoped dry-run structured edit preview`.
- [x] Add review receipt field `dry_run_edit_review`.
- [x] Add source-owned caps for dry-run edit preview count and replacement text bytes.
- [x] Mirror those fields into Zed's Web Preview contract JSON.

## Task 2: Preserve Full Bounded Replacement Text In Zed Apply Gate

**Files:**
- Modify: `crates/agent_ui/src/dx_style_panel/receipt_review.rs`

- [x] Keep existing human preview text.
- [x] Add bounded `replacement_text` to structured edit preview JSON.
- [x] Preserve compatibility with the existing `replacement` preview field.

## Task 3: Native Dry-Run Edit Review Block

**Files:**
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`

- [x] Validate the source-owned edit-preview caps from the source-apply contract.
- [x] Validate that trusted dry-run receipt summaries include a structured edit preview whose source path matches the request path, whose byte range encloses the request source span, and whose replacement text is non-empty and bounded.
- [x] Preserve a review-only `dry_run_edit_review` object in native source-apply receipts.
- [x] Keep `mutation_ready=false` and keep the native writer review-only.

## Task 4: Source Guards And Handoff Docs

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard the new source-owned contract fields and Zed mirror.
- [x] Guard native dry-run edit review validation and receipt preservation.
- [x] Record the honest source-only score.
- [x] Record that source mutation, build proof, WebView proof, and runtime proof remain unproven.

## Verification

- [x] `rustfmt --edition 2024 --check` on touched Rust files.
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`.
- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
