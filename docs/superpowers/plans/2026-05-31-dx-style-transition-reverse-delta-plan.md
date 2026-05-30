# DX Style Transition Reverse Delta Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend review-only generated CSS to grouped atomic utility projection for transition and easing declarations already emitted by DX Style visual generator recipes.

**Architecture:** DX Style owns the mappings for `transition-property`, `transition-duration`, `transition-delay`, and `transition-timing-function`. Zed mirrors the fixture and Web Preview consumes the same strategy names. Known timing functions map to named `ease-*` utilities first; arbitrary timing values fall back to `ease-[...]`. Transition property replacement uses a dedicated family matcher so `transition-discrete` and `transition-normal` are not confused with property utilities.

**Tech Stack:** Rust source contract, JSON fixture mirror, embedded Web Preview JavaScript, Node source guards, `rustfmt --check`.

---

## Task 1: Source-Owned Transition Strategies

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_reverse_css_delta.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-reverse-css-delta-contract.json`

- [x] Add transition property and timing-function strategies.
- [x] Add mappings for `transition-property`, `transition-duration`, `transition-delay`, and `transition-timing-function`.
- [x] Keep `source_mutation_enabled=false`.

## Task 2: Zed Web Preview Consumer

**Files:**
- Modify: `crates\web_preview\src\dx_style_generator_surface\script.rs`
- Modify: `crates\web_preview\src\dx_style_generator_surface\reverse-css-delta-contract.generated.json`

- [x] Sync the generated reverse-delta fixture mirror from `G:\Dx\style`.
- [x] Add the same transition property and easing strategy parsers in Web Preview.
- [x] Keep native source apply review-only and separate from mutation readiness.

## Task 3: Guards And Handoff

**Files:**
- Modify: `script\dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard source strategy names, fixture mappings, Web Preview parser branches, and generated mirror freshness.
- [x] Record the checkpoint as source-only transition/easing coverage.
- [x] State that runtime/WebView/build proof and mutation-capable writer proof remain unproven.

## Verification

- [x] `rustfmt --edition 2024 --check` on touched Rust files.
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`.
- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
