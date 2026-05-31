# DX Style Review Receipt Field Coverage

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make source-write readiness prove coverage of every source-owned review receipt field required by the DX Style editor write-bridge preflight.

**Architecture:** The editor write bridge remains source-owned and mutation remains disabled. Zed and Web Preview keep specific blocker codes for high-value fields, and also add a generic coverage check so future required receipt fields cannot silently drift past readiness.

**Tech Stack:** Rust, embedded Web Preview JavaScript, source-only Node guards. No Cargo/build/runtime/`just run` commands.

---

## Task 1: Readiness Coverage

**Files:**
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`
- Modify: `crates/web_preview/src/dx_style_source_apply.rs`

- [x] Define the review receipt fields Zed/Web Preview can currently emit.
- [x] Compare the source-owned bridge required field list against that emitted-field set.
- [x] Surface missing required fields in readiness packets and add a generic blocker when coverage is incomplete.
- [x] Preserve existing specific blocker codes.

## Task 2: Guards And Handoff

**Files:**
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Guard Web Preview and native coverage checks.
- [x] Record source-only verification and the remaining runtime/build boundary.

## Verification

- [x] `rustfmt --edition 2024 --check crates\web_preview\src\dx_style_source_apply.rs`
- [x] `node --test script\dx-style-panel-source.test.ts script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts`
- [x] `git diff --check`
- [x] Conflict-marker scan.
- [x] Focused commit in `G:\Dx\zed`.
