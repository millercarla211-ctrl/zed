# DX Style Spacing Sizing Reverse Delta Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend review-only generated CSS to grouped atomic utility projection for deterministic display, spacing, sizing, and flex-content declarations without enabling source mutation.

**Architecture:** DX Style owns the reverse-delta contract and value parsing strategies. Zed mirrors the source-owned fixture into Web Preview and implements matching review-only parser branches. The slice stays conservative: only declarations with unambiguous atomic utility families are added, margin negatives are represented as DX Style's real negative utility form, and `source_mutation_enabled` remains false.

**Tech Stack:** Rust source contract, JSON fixtures, embedded Web Preview JavaScript, Node source guards, `rustfmt --check`.

---

## Task 1: Source-Owned Utility Strategies

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_reverse_css_delta.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-reverse-css-delta-contract.json`

- [x] Add deterministic strategies for display keywords, align-content keywords, and negative-aware margin spacing values.
- [x] Add mappings for spacing, margin, sizing, display, and align-content declarations.
- [x] Keep `source_mutation_enabled=false`.

## Task 2: Zed Web Preview Consumer

**Files:**
- Modify: `crates\web_preview\src\dx_style_generator_surface\script.rs`
- Modify: `crates\web_preview\src\dx_style_generator_surface\reverse-css-delta-contract.generated.json`

- [x] Sync the generated reverse-delta fixture mirror from `G:\Dx\style`.
- [x] Teach Web Preview review to parse the new strategy names and prefixless display utilities.
- [x] Keep native source apply review-only and separate from mutation readiness.

## Task 3: Guards And Handoff

**Files:**
- Modify: `script\dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard source strategy names, fixture mappings, Web Preview parser branches, and generated mirror freshness.
- [x] Record the checkpoint as source-only generator coverage for the Style sidebar.
- [x] State that runtime/WebView/build proof and mutation-capable writer proof remain unproven.

## Verification

- [x] `rustfmt --edition 2024 --check` on touched Rust files.
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`.
- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
