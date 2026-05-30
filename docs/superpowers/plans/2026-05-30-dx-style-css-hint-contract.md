# DX Style CSS Hint Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move CSS declaration-to-visual-generator hints into a DX Style-owned contract consumed by the Zed Style sidebar.

**Architecture:** DX Style owns the hint read model and checked-in fixture. Zed consumes a generated mirror for lightweight cursor-time lookup and sends read-only generator hints to Web Preview; source mutation remains gated by existing trusted receipts and apply contracts.

**Tech Stack:** Rust source contracts, JSON fixtures, Zed GPUI active editor context, Web Preview JavaScript diagnostics, Node source guards.

---

### Task 1: Source-Owned CSS Hint Contract

**Files:**
- Create: `G:\Dx\style\src\core\engine\visual_generator_css_hint_catalog.rs`
- Create: `G:\Dx\style\fixtures\visual-generator-css-declaration-hint-catalog.json`
- Modify: `G:\Dx\style\src\core\engine\mod.rs`
- Modify: `G:\Dx\style\src\core\mod.rs`

- [ ] Add a `dx.style.visual-generator-css-declaration-hint-catalog` read model with property match mode, value hints, generator id, and emitted token hint.
- [ ] Add a checked-in fixture with the same schema and entries.
- [ ] Re-export the contract so source guards and editor consumers can find it.

### Task 2: Zed Consumption

**Files:**
- Create: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\css_hint_catalog.rs`
- Create: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\css-declaration-hint-catalog.generated.json`
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel.rs`
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\css_cursor_context.rs`
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\active_context.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\script.rs`

- [ ] Parse the generated mirror through a small cached lookup helper.
- [ ] Replace Zed-owned CSS property matching with the source-owned catalog lookup.
- [ ] Forward a read-only `css_generator` hint in active context JSON.
- [ ] Let Web Preview prioritize a valid `css_generator` before token/class-list matching and label it as `css_declaration`.

### Task 3: Guardrails And Docs

**Files:**
- Modify: `G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs`
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`
- Modify: `G:\Dx\style\README.md`
- Modify: `G:\Dx\style\PLAN.md`
- Modify: `G:\Dx\style\CHANGELOG.md`

- [ ] Extend the fixture mirror check to include the CSS hint catalog.
- [ ] Add source guards proving DX Style owns the hint contract and Zed consumes the generated mirror.
- [ ] Update handoff docs with the read-only CSS hint boundary.
- [ ] Run only lightweight source checks: targeted Node guard, mirror check, targeted rustfmt, conflict/trailing whitespace scans, and `git diff --check`.
