# DX Style Layout Reverse Delta Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend review-only generated CSS to grouped atomic utility projection for layout, flex alignment, and grid track declarations without enabling source mutation.

**Architecture:** DX Style owns the reverse-delta contract and value parsing strategies. Zed mirrors the source-owned fixture into Web Preview, then uses the same strategy names in the copied review packet and source-apply review path. The slice stays review-only: it adds evidence for future source edits but does not enable `Apply`, a writer, or mutation.

**Tech Stack:** Rust source contract, JSON fixtures, embedded Web Preview JavaScript, Node source guards, `rustfmt --check`.

---

## Task 1: Source-Owned Layout Strategies

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_reverse_css_delta.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-reverse-css-delta-contract.json`

- [x] Add value strategies for alignment keywords, justify-content keywords, and grid repeat track counts.
- [x] Add mappings for `align-items`, `justify-content`, `grid-template-columns`, `grid-template-rows`, `column-gap`, and `row-gap`.
- [x] Keep `source_mutation_enabled=false`.

## Task 2: Zed Web Preview Strategy Consumer

**Files:**
- Modify: `crates\web_preview\src\dx_style_generator_surface\script.rs`
- Modify: `crates\web_preview\src\dx_style_generator_surface\reverse-css-delta-contract.generated.json`

- [x] Sync the generated reverse-delta fixture mirror from `G:\Dx\style`.
- [x] Teach Web Preview reverse-delta preview to parse the new strategy names.
- [x] Keep the preview path read-only and separate from `sourceApplyReady`.

## Task 3: Guards And Handoff

**Files:**
- Modify: `script\dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard source contract strategy names, fixture mappings, generated mirror freshness, and browser parser branches.
- [x] Record the checkpoint as source-only reverse-delta review coverage.
- [x] State that runtime/WebView/build proof and mutation-capable writer proof remain unproven.

## Verification

- [x] `rustfmt --edition 2024 --check` on touched Rust files.
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`.
- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
