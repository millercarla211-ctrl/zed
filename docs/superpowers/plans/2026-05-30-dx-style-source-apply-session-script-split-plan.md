# DX Style Source Apply Session Script Split Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the DX Style Web Preview source-apply session browser helpers out of the giant generator script without changing behavior.

**Architecture:** Keep `script.rs` as the generator cockpit coordinator, but give the trusted source-apply session constants and IPC handler their own Rust-owned browser-script module. The parent generator surface still injects one final script into the data URL.

**Tech Stack:** Rust raw-string browser script modules, GPUI Web Preview, Node source guards.

---

## Task 1: Split The Session Script

**Files:**
- Create: `crates/web_preview/src/dx_style_generator_surface/source_apply_session_script.rs`
- Modify: `crates/web_preview/src/dx_style_generator_surface.rs`
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`

- [x] Add a focused module for source-apply session constants and handler code.
- [x] Replace inline code in `script.rs` with named placeholders.
- [x] Have `dx_style_generator_script()` compose the final browser script from focused pieces.

## Task 2: Keep Readiness And Guards Honest

**Files:**
- Modify: `crates/agent_ui/src/dx_style_panel.rs`
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Make Style sidebar readiness scan the new session-script module.
- [x] Update the source guard parser so the composed browser script remains syntax-checked.
- [x] Record the maintainability split in the handoff docs.

## Verification

- [ ] Targeted Rust formatting check.
- [ ] DX Style source guard.
- [ ] Combined source guard batch.
- [ ] `git diff --check`.
- [ ] Conflict-marker scan.
- [ ] Focused commit.
