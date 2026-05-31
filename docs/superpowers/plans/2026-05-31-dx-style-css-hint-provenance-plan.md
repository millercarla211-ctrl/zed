# DX Style CSS Hint Provenance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (already satisfied for this goal) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve the exact source-owned CSS declaration hint that routes a CSS cursor context to a Web Preview visual generator.

**Architecture:** DX Style already owns the CSS declaration hint catalog. Zed should carry the matched hint ordinal, match rule, property pattern, and value filters through the active context, panel summary, Web Preview review packets, source-write readiness, and native review receipts. Mutation remains disabled.

**Tech Stack:** Rust source-owned contracts and GPUI context, JSON fixture mirrors, Web Preview JavaScript, Node source guards.

---

### Task 1: Active Context Provenance

**Files:**
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\css_hint_catalog.rs`
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\css_cursor_context.rs`
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\active_context.rs`
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\panel_view.rs`

- [x] Parse and preserve the matched CSS hint ordinal, property pattern, match mode, and value filters.
- [x] Include the provenance fields in `zed.dx_style.active_context.v1`.
- [x] Surface the hint provenance in the native Style panel summary.

### Task 2: Web Preview And Native Review

**Files:**
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\script.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\css_declaration_dry_run_script.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_source_apply.rs`

- [x] Include a structured `css_declaration_hint` packet in Web Preview review and native review requests.
- [x] Preserve the same packet in native review receipts.
- [x] Add hint provenance to CSS declaration dry-run preview output and source-write readiness fields.

### Task 3: Source-Owned Contract Guards

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_source_apply.rs`
- Modify: `G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs`
- Modify: `G:\Dx\style\src\core\engine\css_declaration_dry_run.rs`
- Modify: matching `G:\Dx\style\fixtures\*.json`
- Modify: generated Zed fixture mirrors
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`, `G:\Dx\zed\todo.txt`, and `G:\Dx\zed\changelog.txt`

- [x] Require CSS declaration hint provenance in source-owned review guards.
- [x] Mirror fixture changes into Zed.
- [x] Add source guards proving the provenance fields are carried end to end.

## Verification

- `rustfmt --edition 2024 --check G:\Dx\style\src\core\engine\grouped_class_source_apply.rs G:\Dx\style\src\core\engine\grouped_class_editor_write_bridge.rs G:\Dx\style\src\core\engine\css_declaration_dry_run.rs crates\agent_ui\src\dx_style_panel\css_hint_catalog.rs crates\agent_ui\src\dx_style_panel\css_cursor_context.rs crates\agent_ui\src\dx_style_panel\active_context.rs crates\agent_ui\src\dx_style_panel\panel_view.rs crates\web_preview\src\dx_style_generator_surface.rs crates\web_preview\src\dx_style_source_apply.rs`
- `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --write`
- `node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check`
- `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`
- `git diff --check`
- focused conflict-marker source scan
