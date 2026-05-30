# DX Style Editor Integration Completion Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the DX Style integration across `G:\Dx\style` and `G:\Dx\zed` into a professional, source-backed Style sidebar where GPUI hosts editor state and dock placement while Web Preview owns the visual CSS generator cockpit.

**Architecture:** Zed GPUI remains the native shell: panel registration, editor cursor context, source identity, readiness gates, and native review receipts. Web Preview owns generator-heavy UI: controls, visual previews, copy/review actions, CSS declaration dry-run review, and source-apply request construction. DX Style owns contracts, fixtures, catalogs, recipes, and receipt shape so Zed does not hardcode style policy.

**Tech Stack:** Rust, GPUI, Zed workspace panels, Web Preview data URLs, source-owned JSON fixtures, Node source guards, lightweight `rustfmt --check`, and source-only verification until build/runtime proof is explicitly allowed.

---

## Current Score

- Source-only integration score: **72/100**.
- True production readiness score: **55/100**.

The source architecture is directionally strong: real right-dock panel, source-owned DX Style contracts, Web Preview generator cockpit, bounded active editor context, review-only native source-apply receipts, fixture mirror checks, and source guards exist. It is not 100/100 because build/runtime proof is intentionally blocked, source mutation remains disabled, several files are still untracked, and the final integration has not been compiled or run inside Zed.

## What Has Been Tried

- Built DX Style grouped-class read models, dry-run receipt contracts, source digest contract, editor write-bridge preflight, visual generator catalog, control catalog, recipe catalog, CSS declaration hint catalog, reverse CSS map receipt, reverse CSS delta contract, and CSS declaration dry-run contract.
- Mirrored DX Style fixtures into Zed Web Preview modules with bounded fixture loading and generated fallback files.
- Added the Zed right-dock DX Style panel and launch workspace Style rail.
- Wired active editor context for static `class` and `className` attributes, grouped calls, class lists, CSS declaration hints, source path, source span, and source digest.
- Built the Web Preview generator surface with first-pass visual generators, search, controls, live preview, source-owned recipe output, copy actions, source review action, and disabled mutation apply gate.
- Added review-only native handling for `dx-style-source-apply`, including receipt summaries, blockers, CSS dry-run diagnostics, preview output metadata, reverse CSS delta evidence, and source-apply contract drift checks.
- Added source guards and fixture checks covering catalog/recipe/control mirrors, source-apply contracts, CSS dry-run contracts, Web Preview script parseability, native panel wiring, and launch rail registration.
- Kept forbidden proof out of scope: no `just run`, no Cargo build/check/test/clippy, no servers, no browser/WebView runtime proof.

## Remaining Gaps

- The code is not compiled. Source guards prove shape, not Rust type correctness across the full Zed workspace.
- The Web Preview UI is not runtime-verified inside Zed. The script parse guard is useful but does not prove WebView rendering, IPC, focus behavior, or pane placement.
- Apply remains intentionally disabled. That is correct for safety, but it means the atomic/custom CSS two-way workflow is still review-first, not mutation-ready.
- The current worktree has many untracked files. A production handoff requires staging and committing the whole coherent lane, not leaving generated mirrors and source modules loose.
- Some user-facing docs are long and need a crisp checkpoint summary so the next worker sees exact status, risks, and validation boundaries.
- Existing internal schema strings use established compatibility names. New user-facing surfaces should avoid throwaway labels and version-ish naming, while preserving existing wire contracts unless a coordinated migration is planned.

## Completion Strategy

### Task 1: Plan And Score Checkpoint

**Files:**
- Create: `G:\Dx\zed\docs\superpowers\plans\2026-05-30-dx-style-production-completion-plan.md`
- Modify: `G:\Dx\zed\todo.txt`
- Modify: `G:\Dx\zed\changelog.txt`

- [ ] Write this plan with the honest score, tried work, gaps, and next tasks.
- [ ] Update `todo.txt` with the checkpoint score and the next exact source-only course of action.
- [ ] Update `changelog.txt` with the completion-plan checkpoint.

### Task 2: Six-Agent Audit

**Files:**
- Read-only unless the parent integrates a finding.

- [ ] Agent 1 audits current git status, branch, untracked files, and commit scope.
- [ ] Agent 2 audits Zed GPUI panel/action/workspace registration and readiness gating.
- [ ] Agent 3 audits Web Preview generator surface and source-apply request construction.
- [ ] Agent 4 audits DX Style contracts, fixtures, and Zed mirror alignment.
- [ ] Agent 5 audits docs/status clarity and forbidden-proof boundaries.
- [ ] Agent 6 audits code quality risks, large-file risks, stale naming, and professional handoff readiness.

### Task 3: Integrate Actionable Findings

**Files:**
- Modify only files already in the DX Style lane unless a finding proves another file is necessary.

- [ ] Fix any source-only correctness gaps that are narrow and do not require build/runtime proof.
- [ ] Keep GPUI as shell and Web Preview as generator cockpit.
- [ ] Keep mutation disabled unless source-owned contracts, trusted receipts, and editor bridge all prove readiness.
- [ ] Preserve existing Zed behavior outside the Style sidebar lane.

### Task 4: Source-Only Verification

**Allowed commands:**

```powershell
rustfmt --edition 2024 --check --config skip_children=true crates\agent_ui\src\dx_style_panel.rs crates\agent_ui\src\dx_style_panel\*.rs crates\web_preview\src\dx_style_generator_surface.rs crates\web_preview\src\dx_style_generator_surface\*.rs crates\web_preview\src\dx_style_source_apply.rs crates\web_preview\src\web_preview_view.rs
node G:\Dx\style\scripts\sync_zed_visual_generator_fixtures.mjs --check
node --test script\dx-handoff-source-guard-registry.test.ts script\dx-launch-workspace-source.test.ts script\dx-style-panel-source.test.ts
git diff --check
Select-String -Path DX.md,changelog.txt,todo.txt,crates\agent_ui\src\dx_style_panel.rs,crates\web_preview\src\dx_style_source_apply.rs,script\dx-style-panel-source.test.ts,G:\Dx\style\README.md,G:\Dx\style\PLAN.md,G:\Dx\style\CHANGELOG.md -Pattern '<<<<<<<|=======|>>>>>>>'
```

**Forbidden until the user explicitly allows it:**
- `just run`
- Cargo build/check/test/clippy
- Zed runtime launch
- WebView/browser runtime proof
- local servers

- [ ] Run the allowed source-only checks.
- [ ] Read outputs and report exact pass/fail counts.
- [ ] Do not claim production runtime readiness from source-only proof.

### Task 5: Commit Boundary

**Files:**
- Stage only this lane's source, docs, source guards, and generated fixture mirrors.

- [ ] Inspect `git status --short --untracked-files=all`.
- [ ] Stage the complete coherent DX Style/Zed Style sidebar lane.
- [ ] Verify staged diff scope.
- [ ] Commit with a professional message:

```powershell
git commit -m "feat: add DX Style Web Preview sidebar"
```

- [ ] Leave the goal active unless every source, runtime, build, and mutation requirement is actually proven.

## Professional Naming Rule

Use clear product names in new user-facing docs and UI: "DX Style", "Style sidebar", "Web Preview generator surface", "source review", "dry-run receipt", and "editor write bridge". Do not introduce throwaway labels, demo names, or decorative version names. Preserve existing wire contract identifiers unless a dedicated migration updates all producers, consumers, fixtures, and guards together.

## Next Exact Action

Run the six-agent audit, integrate only concrete source-only findings, then verify and commit the lane as one professional checkpoint.
