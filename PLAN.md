# Zed DX Editor Feature Plan

## Summary

This checkpoint defines the next Zed DX editor feature batch. It is a planning-only document: no source implementation starts in this checkpoint commit.

## Task Groups

### 1. Project Panel Performance And Media Explorer

- Diagnose why the project panel/file explorer feels laggy, with special attention to expensive visible-tree derivation, large expanded directories, root-drive/system-file scanning, file metadata fanout, and render work done per visible row.
- Keep the existing tree explorer behavior for normal code and document files.
- Add a lazy media presentation for folders that contain images, videos, or audio:
  - Images render as direct visual previews in a grid or rectangle layout.
  - Videos render as a representative preview frame when available.
  - Audio renders as centered rectangle cards showing duration and file name.
- Keep the explorer fast by virtualizing media grids, doing thumbnail and duration extraction off the UI path, caching results, and never blocking tree navigation on media probing.
- Preserve current selection, rename, drag/drop, context menu, reveal, and keyboard behavior unless explicitly changed by the implementation task.

### 2. Web Preview Onboarding

- Replace the current Zed onboarding surface with a local Web Preview onboarding surface.
- For the first version, the onboarding page contains a minimal website with a single `Complete` button.
- Clicking `Complete` dismisses the onboarding and returns the editor to the normal workspace state.
- Keep this page local and lightweight for now so future onboarding work can expand it without requiring network access or a separate app server.
- Preserve existing onboarding completion state semantics where practical, so users do not repeatedly see onboarding after completion.

### 3. Dock And Sidebar Stacking

- Add support for showing two or three panels within one dock area, using half-height or one-third-height stacked regions.
- Begin conservatively with the right dock and AI panel layout path, then generalize only where the existing dock model supports it cleanly.
- Reuse existing dock panel registration, activation, focus, zoom, resize, icon button, tooltip, and persistence patterns.
- Add controls that let a user open compatible panels at the same time without replacing the currently visible panel.
- Keep panel stacking compatible with existing single-panel dock behavior.

### 4. Workspace Activity Bar

- Add a collapsed workspace/sidebar mode that behaves like a VS Code-style activity bar.
- In collapsed mode, show only one icon-width worth of UI and remove detailed text from the main sidebar surface.
- Show removed details through hover tooltips.
- Preserve the expanded workspace/sidebar behavior for users who want the current fuller sidebar.
- Avoid decorative or dummy controls; each icon must map to a real existing panel, workspace action, or navigation target.

### 5. Empty Workspace AI Mode

- After onboarding is complete, when no project is open, show the AI panel in a full-screen default state.
- In that empty-workspace state, include the DX/Codex progress and DCP-oriented rail on the right side.
- Keep the rail connected to real DX/Codex status data where available, and show honest empty or missing states where data is not available.
- Preserve normal editor and project behavior once a project is opened.

## Constraints

- Do not run `just run` unless the user explicitly authorizes it.
- Do not run broad Cargo commands or heavy validation unless explicitly authorized.
- Use source inspection and lightweight checks first.
- Preserve existing Zed behavior unless a task explicitly changes it.
- Avoid dummy UI, fake state, and disconnected demo-only controls.
- Keep new surfaces wired to real editor state, real panel registrations, or honest unavailable states.
- Keep changes maintainable, typed, and consistent with existing Zed/GPUI patterns.

## Checkpoint Verification

- `git status --short`
- `git diff -- PLAN.md PLAN_DEPRICATED.md`
- `git diff --check`

## Checkpoint Commit

```bash
git add PLAN.md PLAN_DEPRICATED.md
git commit -m "docs: checkpoint zed editor feature plan"
```

## Assumptions

- `PLAN_DEPRICATED.md` is intentionally spelled this way because the user requested that exact filename.
- This checkpoint is documentation-only.
- No push is included unless the user separately requests it.
