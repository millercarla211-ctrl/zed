# DX Style CSS Dry-Run Script Split Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move CSS declaration dry-run review logic out of the giant DX Style Web Preview generator script without changing behavior.

**Architecture:** `script.rs` remains the generator cockpit coordinator. CSS declaration dry-run contract constants, diagnostics, preview derivation, and generated CSS declaration parsing move into a focused Rust-owned browser-script module that is composed into the final Web Preview script.

**Tech Stack:** Rust raw-string browser script modules, GPUI Web Preview, Node source guards.

---

## Step-Back Checkpoint

**What has already been tried:** The DX Style/Zed lane has source-only Web Preview generator wiring, review-only native source-apply IPC, grouped-class receipt checks, reverse CSS delta review evidence, CSS declaration dry-run contracts, and several focused script/module splits. Those are useful foundations, but they do not equal runtime proof or source mutation readiness.

**Current honest score:** 72/100 at the step-back checkpoint and 74/100 after this source-only guard hardening. The source-only architecture is solid enough to keep improving, but true 100/100 still needs authorized runtime validation, native WebView proof, build proof, stronger source identity guarantees, and a mutation-capable editor write bridge.

**Chosen next course:** Finish this checkpoint as a narrow maintainability and guard-hardening commit. Do not expand into mutation, broad refactors, or build/runtime proof in this pass.

**Professional naming rule:** Use stable descriptive names such as "CSS declaration dry-run", "source-apply session", and "Web Preview generator". Avoid throwaway version labels in public docs or code names.

## Task 1: Split CSS Dry-Run Browser Logic

**Files:**
- Create: `crates/web_preview/src/dx_style_generator_surface/css_declaration_dry_run_script.rs`
- Modify: `crates/web_preview/src/dx_style_generator_surface.rs`
- Modify: `crates/web_preview/src/dx_style_generator_surface/script.rs`

- [x] Move CSS declaration dry-run constants into the focused module.
- [x] Move CSS declaration diagnostics, preview derivation, bounded diagnostic handling, and generated CSS declaration parsing into the focused module.
- [x] Compose the module into the final generator browser script with placeholders.

## Task 2: Guard The Split

**Files:**
- Modify: `crates/agent_ui/src/dx_style_panel.rs`
- Modify: `script/dx-style-panel-source.test.ts`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] Keep Style sidebar readiness aware of the focused CSS dry-run browser module.
- [x] Keep the source guard parser checking the fully composed browser script.
- [x] Move ownership assertions for CSS dry-run helpers to the new file.
- [x] Preserve source-apply contract version/scope/fixture/consumer metadata through the Web Preview adapter.
- [x] Add source guards for dual source-apply session tokens, refusal-before-review order, and focused split-file budgets.
- [x] Refresh handoff docs so the checkpoint does not advertise stale readiness.

## Verification

- [x] Targeted Rust formatting check.
- [x] DX Style source guard.
- [x] Combined source guard batch.
- [x] Fixture mirror check.
- [x] `git diff --check`.
- [x] Conflict-marker scan.
- [ ] Focused commit.
