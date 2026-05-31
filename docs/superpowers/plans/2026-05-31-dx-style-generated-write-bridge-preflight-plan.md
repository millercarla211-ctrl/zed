# DX Style Generated Write-Bridge Preflight Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Zed's DX Style editor write-bridge fallback source-owned by mirroring the DX Style preflight fixture into Zed and parsing that generated mirror before using any emergency defaults.

**Architecture:** Zed currently reads `G:\Dx\style\fixtures\grouped-class-editor-write-bridge-preflight.json` at runtime and carries a hand-written Rust fallback list. That duplicates the source-owned contract. The existing DX Style fixture sync helper should mirror the preflight JSON into Zed, and Zed should parse the generated mirror when the live DX Style checkout is unavailable.

**Tech Stack:** Rust, Node fixture sync, DX Style source-owned JSON fixture, Zed GPUI panel.

---

### Task 1: Fixture Mirror

**Files:**
- Modify: `G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs`
- Add generated mirror: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\editor-write-bridge-preflight.generated.json`

- [x] Add `fixtures/grouped-class-editor-write-bridge-preflight.json` to the mirror pair list.
- [x] Generate the Zed mirror with the existing `--write` path.

### Task 2: Generated Fallback Consumer

**Files:**
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\editor_write_bridge.rs`

- [x] Include the generated preflight JSON in Zed.
- [x] Parse the generated mirror when the live DX Style fixture is missing or invalid.
- [x] Keep a tiny fail-closed emergency fallback for corrupted generated JSON.
- [x] Preserve `can_apply=false` and `can_mutate_source=false` behavior.

### Task 3: Guards, Docs, Verification

**Files:**
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Guard the sync helper, generated mirror, and generated fallback parsing path.
- [x] Document the source-only checkpoint and keep runtime proof unclaimed.
- [x] Run fixture sync check, targeted rustfmt check, focused Node source guards, `git diff --check`, conflict-marker scan, and commit.
