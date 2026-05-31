# DX Style Group Call Read Model Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make DX Style's source-owned read model distinguish alias-only group calls, inline grouped utilities, and source declarations so Zed and AI agents can reason about `button()`, `button(p-4 ...)`, and `@button(p-4 ...)` without loose string guessing.

**Architecture:** The existing read model detects static class attributes and provides dry-run patch previews. A bounded group-call parser should live in the same source-owned read model and emit typed syntax/provenance fields without mutating source. Zed will guard that the contract exists, but runtime/editor mutation remains disabled.

**Tech Stack:** Rust source model, serde metadata, source-only Zed guard.

---

### Task 1: DX Style Read Model

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_read_model.rs`

- [x] Add typed grouped-call syntax metadata for alias reference, inline utilities, and source declaration forms.
- [x] Add a bounded parser for a single grouped-class token.
- [x] Keep non-group utilities such as arbitrary `bg-[url(...)]` out of the grouped-call model.

### Task 2: Zed Guard And Docs

**Files:**
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Guard the source-owned parser and metadata fields.
- [x] Document that this is source-only read-model metadata.

### Task 3: Verification

- [x] Run targeted rustfmt check, focused Node source guards, `git diff --check`, conflict-marker scan, and commit Zed guard/docs changes.
