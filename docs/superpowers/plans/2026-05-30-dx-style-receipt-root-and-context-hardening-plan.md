# DX Style Receipt Root And Context Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the DX Style sidebar trust project-local receipts from the active workspace file while keeping active editor context cheap enough for large style-bearing files.

**Architecture:** Zed should derive the active file's absolute path and workspace root from the workspace project before scanning project-local `.dx/receipts/style` roots. Active source decisions should not race against global DX fallback receipts; active context should check the editor buffer size before cloning full text, keeping Web Preview generator context source-only and review-first.

**Tech Stack:** Rust, GPUI/Zed workspace project paths, bounded filesystem receipt scans, Node source guards, lightweight rustfmt checks only.

---

### Task 1: Shared Receipt Root Resolver

**Files:**
- Create: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\receipt_roots.rs`
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel.rs`
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\group_registry.rs`
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\apply_gate.rs`

- [ ] Add a focused helper that accepts an optional active source path and workspace root.
- [ ] Only walk source ancestors when the source path is absolute, so relative `ProjectPath` values do not accidentally resolve against Zed's process cwd.
- [ ] De-duplicate roots by Windows-friendly slash-normalized, case-folded path keys.
- [ ] Use the helper in grouped-class registry receipt scanning and dry-run apply-gate receipt scanning without global fallback roots for active-source decisions.

### Task 2: Absolute Active Source Context

**Files:**
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\active_context.rs`

- [ ] Read the active item's `ProjectPath` as the display-relative label.
- [ ] Resolve the absolute active source path through `workspace.project().read(cx).absolute_path(&project_path, cx)`.
- [ ] Send the absolute path to group context, dry-run receipt matching, and Web Preview `source_path`.
- [ ] Keep the relative path as a fallback if the project cannot absolutize the path.

### Task 3: Active Buffer Size Guard

**Files:**
- Modify: `G:\Dx\zed\crates\agent_ui\src\dx_style_panel\active_context.rs`

- [ ] Use `display_snapshot.buffer_snapshot().len().0` to reject oversized active buffers before calling `editor.text(cx)`.
- [ ] Keep the existing post-clone byte check as a defensive second guard.
- [ ] Preserve current statuses and copy for large files.

### Task 4: Source Guards And Docs

**Files:**
- Modify: `G:\Dx\zed\script\dx-style-panel-source.test.ts`
- Modify: `G:\Dx\zed\DX.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [ ] Guard the new receipt root module, absolute path resolution, source-root fallback ordering, and pre-clone buffer size check.
- [ ] Record that this is source-only hardening and does not claim runtime/WebView proof.

### Task 5: Verification And Commit

**Allowed commands:**

```powershell
rustfmt --edition 2024 --check --config skip_children=true crates\agent_ui\src\dx_style_panel.rs crates\agent_ui\src\dx_style_panel\*.rs crates\agent_ui\src\dx_style_panel\readiness\expected_files.rs
node --test script\dx-style-panel-source.test.ts script\dx-launch-workspace-source.test.ts script\dx-handoff-source-guard-registry.test.ts
git diff --check
rg -n "^(<<<<<<<|=======|>>>>>>>)" DX.md changelog.txt todo.txt crates\agent_ui\src\dx_style_panel.rs crates\agent_ui\src\dx_style_panel script\dx-style-panel-source.test.ts
```

**Forbidden until explicitly authorized:**
- `just run`
- Cargo build/check/test/clippy
- Zed runtime launch
- WebView/browser proof
- local servers

- [ ] Run only allowed source-only checks.
- [ ] Commit the coherent hardening slice if checks pass.
