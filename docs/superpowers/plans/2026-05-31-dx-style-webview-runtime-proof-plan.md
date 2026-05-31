# DX Style WebView Runtime Proof Readiness

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make source-write readiness check the exact WebView source-review round-trip runtime proof name advertised by the DX Style editor write-bridge preflight.

**Architecture:** The bridge remains source-owned and mutation remains disabled. Web Preview and native readiness should report a specific blocker when `successful WebView source-review round trip` is missing, rather than treating any runtime proof list as enough.

**Tech Stack:** Rust, embedded Web Preview JavaScript, source-only Node guards. No Cargo/build/runtime/`just run` commands.

---

## Task 1: Readiness Blockers

**Files:**
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`

- [x] Add a specific WebView source-review round-trip runtime-proof blocker in Web Preview readiness.
- [x] Add the same specific blocker in native readiness.
- [x] Keep mutation disabled and the broader runtime proof blocker unchanged.

## Task 2: Guards And Handoff

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard the new blocker codes in Web Preview and native source-only tests.
- [x] Record source-only verification and the remaining runtime/build boundary.

## Verification

- [x] `rustfmt --edition 2024 --check crates\web_preview\src\dx_style_source_apply.rs`
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`
- [x] `git diff --check`
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
