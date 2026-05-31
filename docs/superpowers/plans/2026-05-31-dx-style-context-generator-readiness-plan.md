# DX Style Context Generator Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the native DX Style panel honest about Web Preview's context-aware generator handoff.

**Architecture:** Web Preview already selects and promotes a generator from the active Zed context. The native bridge-readiness scan should require those source markers before calling the host ready, and the native panel should surface CSS declaration generator hints before handoff.

**Tech Stack:** Rust, GPUI panel shell, Web Preview source marker scan, Node source guards.

---

### Task 1: Bridge Readiness Markers

**Files:**
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel.rs`

- [x] Require `generatorForContext`, `orderedCatalog`, and `suggested_generator` markers when declaring the Web Preview Style bridge ready.
- [x] Update the ready next action so it no longer says cursor-scoped handoff still needs wiring.

### Task 2: Native Summary Handoff

**Files:**
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\panel_view.rs`

- [x] Show the active CSS declaration generator hint in the native panel summary.
- [x] Preserve existing disabled state and Web Preview handoff behavior.

### Task 3: Guards, Handoff, Verification

**Files:**
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [x] Guard the readiness markers and native summary metric.
- [x] Document the source-only checkpoint and keep runtime proof unclaimed.
- [x] Run targeted rustfmt check, focused Node source guards, `git diff --check`, conflict-marker scan, and commit.
