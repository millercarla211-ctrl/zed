# DX Style Group Context Syntax Alignment Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align Zed's active group context vocabulary with DX Style's source-owned grouped-call read model so alias references, inline utilities, and source declarations use the same names.

**Architecture:** Zed already parses grouped class tokens for Web Preview context. Keep that parser local and lightweight, but rename the emitted syntax/status values so `button()`, `button(p-4 ...)`, and `@button(p-4 ...)` map cleanly to the source-owned read-model categories.

---

### Task 1: Zed Context Alignment

**Files:**
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\group_context.rs`

- [x] Rename alias-call syntax output to `alias_reference`.
- [x] Rename inline utility syntax output to `inline_utilities`.
- [x] Preserve `@alias(...)` as `source_declaration` / `source_group_declaration`.
- [x] Keep the existing line-budget guard intact.

### Task 2: Guards And Docs

**Files:**
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Guard the aligned syntax names and source declaration status.
- [x] Document source-only behavior and keep mutation disabled.
- [x] Run targeted rustfmt check, focused Node source guards, `git diff --check`, conflict-marker scan, and commit.
