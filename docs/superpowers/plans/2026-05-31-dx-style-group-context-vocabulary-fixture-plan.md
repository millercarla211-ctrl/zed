# DX Style Group Context Vocabulary Fixture Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make DX Style's grouped-class Web Preview context fixture source-own the group-call syntax/status vocabulary and mirror that metadata into Zed's Web Preview contract.

**Architecture:** The Rust read model and Zed active context now use aligned names. The generated Web Preview contract should advertise the same vocabulary so the browser cockpit can inspect it from source-owned JSON instead of hardcoded assumptions.

---

### Task 1: Source Fixture

**Files:**
- Modify: `G:\Dx\style\src\core\engine\grouped_class_web_preview_context.rs`
- Modify: `G:\Dx\style\fixtures\grouped-class-web-preview-context.json`

- [x] Add group-call syntax values.
- [x] Add group-call status values.

### Task 2: Zed Mirror And Web Preview

**Files:**
- Modify generated mirror: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\group-context-contract.generated.json`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\group_context_contract.rs`
- Modify: `G:\Dx\zed\crates\web_preview\src\dx_style_generator_surface\script.rs`

- [x] Mirror the fixture into Zed.
- [x] Pass the new fields through the Web Preview contract adapter.
- [x] Surface vocabulary counts in Web Preview metadata diagnostics.

### Task 3: Guards And Docs

- [x] Update source guards, docs, todo, and changelog.
- [x] Run targeted rustfmt check, fixture sync check, focused Node source guards, `git diff --check`, conflict-marker scan, and commit.
