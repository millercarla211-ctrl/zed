# DX Style Multi Strategy Reverse Delta Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let reverse CSS delta review try every source-owned strategy for a declaration before refusing it, then add conservative arbitrary-value fallbacks for real visual generator CSS output that cannot be represented as a design-token suffix.

**Architecture:** DX Style remains the source of truth for supported declaration strategies. Zed mirrors the fixture and Web Preview evaluates all matching strategies in fixture order. Token strategies stay review-only; they produce replacement previews and native review evidence but never enable source mutation.

**Tech Stack:** Rust source contract, JSON fixture mirror, embedded Web Preview JavaScript, Node source guards, `rustfmt --check`.

---

## Task 1: Multi-Strategy Source Contract

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_reverse_css_delta.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-reverse-css-delta-contract.json`

- [x] Try all mappings for a supported declaration before returning `UnsupportedValue`.
- [x] Add arbitrary-value fallback rows after token rows for `border-radius`, `padding*`, `gap*`, `width`, and `height`.
- [x] Keep `source_mutation_enabled=false`.

## Task 2: Zed Web Preview Consumer

**Files:**
- Modify: `crates\web_preview\src\dx_style_generator_surface\script.rs`
- Modify: `crates\web_preview\src\dx_style_generator_surface\reverse-css-delta-contract.generated.json`

- [x] Sync the generated reverse-delta fixture mirror from `G:\Dx\style`.
- [x] Make Web Preview evaluate all matching mappings for each generated declaration.
- [x] Preserve display as a fallback behind richer non-display declarations.

## Task 3: Guards And Handoff

**Files:**
- Modify: `script\dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard multi-strategy matching, fallback mappings, generated mirror freshness, and mutation-disabled status.
- [x] Record the checkpoint as source-only real-generator fallback coverage.
- [x] State that runtime/WebView/build proof and mutation-capable writer proof remain unproven.

## Verification

- [x] `rustfmt --edition 2024 --check` on touched Rust files.
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`.
- [x] `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
