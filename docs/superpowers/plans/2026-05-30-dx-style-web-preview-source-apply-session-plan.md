# DX Style Web Preview Source Apply Session Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make DX Style source-apply review IPC acceptable only from the trusted Web Preview generator surface opened by Zed.

**Architecture:** Zed remains the native trust boundary. Opening the DX Style generator now creates a short native-held source-apply session token, embeds it into the local data-url generator surface, and refuses `dx-style-source-apply` messages that do not echo the token at both IPC envelope and request levels. Source mutation stays disabled.

**Tech Stack:** Rust, GPUI/WebPreview, embedded Web Preview JavaScript, Node source guards.

---

## What Was Tried

- Built the DX Style right-dock panel as a native GPUI shell, with Web Preview owning the visual generator cockpit.
- Added source-owned DX Style contracts, fixture mirrors, recipe/catalog/control checks, grouped-class context, dry-run receipt matching, reverse CSS map review, reverse declaration deltas, and CSS declaration dry-run review.
- Hardened receipt matching so active source decisions use the active workspace file and project-local receipt roots instead of process-global fallback roots.
- Moved active editor context to a pre-clone size guard so oversized files are refused before full text scanning.

## Brutal Current Assessment

- The integration is strong as a source-only architecture, but not runtime-proven because builds, `just run`, WebView proof, and Cargo checks are intentionally forbidden.
- The highest-risk remaining source-only gap was IPC trust: any Web Preview page could previously send a `dx-style-source-apply` message and receive a native review receipt, even though mutation remained disabled.
- The next maintainability gap after this slice is splitting the large generator script into focused browser modules.

## Task 1: Native Session Token

**Files:**
- Modify: `G:/Dx/style/src/core/engine/grouped_class_source_apply.rs`
- Modify: `G:/Dx/style/fixtures/grouped-class-source-apply-contract.json`
- Modify: `crates/web_preview/src/web_preview_view.rs`
- Modify: `crates/web_preview/src/dx_style_generator_surface.rs`
- Modify: `crates/web_preview/src/dx_style_generator_surface/source_apply_contract.rs`
- Modify: `crates/web_preview/src/dx_style_generator_surface/source-apply-contract.generated.json`
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`

- [x] Promote the trusted Web Preview source-apply session into the DX Style source-owned contract.
- [x] Add a native-held DX Style source-apply session token to `WebPreviewView`.
- [x] Generate a fresh token when Zed opens the DX Style generator through `OpenGeneratorPreview` or `OpenGeneratorPreviewForContext`.
- [x] Embed that token into the local generator page.
- [x] Echo the token in the IPC envelope and review request.
- [x] Refuse source-apply review if the active Web Preview item does not have a matching trusted session.
- [x] Keep native mutation disabled.

## Task 2: Source Guards

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `crates/agent_ui/src/dx_style_panel.rs`

- [x] Guard the new source-apply session placeholder in the embedded generator script parser.
- [x] Guard native Web Preview session validation markers.
- [x] Require sidebar readiness to see the session-token path, not just the review handler.

## Verification

- [ ] Run targeted Rust formatting check on edited Rust files.
- [ ] Run the DX Style source guard.
- [ ] Run fixture sync check.
- [ ] Run `git diff --check`.
- [ ] Scan touched files for conflict markers.
- [ ] Commit only this source-only hardening slice.

## Still Not Claimed

- No Cargo build/check/test/clippy proof.
- No `just run`.
- No live WebView/browser runtime proof.
- No source mutation path.
