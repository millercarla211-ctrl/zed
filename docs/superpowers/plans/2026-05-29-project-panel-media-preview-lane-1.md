# Project Panel Media Preview Lane 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete PLAN.md Lane 1 tasks 1-5 by adding a lazy, bounded Project Panel media presentation for expanded image/video folders while preserving normal tree explorer behavior.

**Architecture:** Keep normal Project Panel rows and selection/open/context-menu/drag behavior unchanged. Add a focused `media_preview` helper module that classifies direct child media from already-indexed worktree entries, caches per expanded folder including no-media misses, and renders bounded image/video preview cards inside the existing uniform-height Project Panel row end slot. Video cards use sidecar image frames when available and otherwise render a lightweight play-card fallback.

**Tech Stack:** Rust, GPUI, Project Panel source guards via Node `node --test`, source-only verification.

---

### Task 1: Source Guard Contract

**Files:**
- Modify: `script/dx-project-panel-source.test.ts`

- [x] **Step 1: Add failing source guard**

Add tests asserting a `media_preview` module exists, direct image previews use `img(...).object_fit(ObjectFit::Cover)`, video previews use available `video_frame_path`, folder media scans are capped, and `details_for_entry` only builds media previews for expanded directories.

- [x] **Step 2: Run source guard to verify RED**

Run: `node --test script/dx-project-panel-source.test.ts`

Expected: FAIL because `crates/project_panel/src/media_preview.rs` and the Project Panel media-preview wiring do not exist yet.

### Task 2: Lazy Media Model And Cache

**Files:**
- Create: `crates/project_panel/src/media_preview.rs`
- Modify: `crates/project_panel/src/project_panel.rs`

- [x] **Step 1: Add media-preview data model**

Create `FolderMediaPreview`, `MediaPreviewItem`, and `MediaPreviewKind`, with named caps for child scan and rendered preview items.

- [x] **Step 2: Build previews from direct child entries**

Classify image/video/audio extensions from direct child files only, cap scanning with `MAX_PROJECT_PANEL_MEDIA_CHILD_SCAN`, cap rendered items with `MAX_PROJECT_PANEL_MEDIA_PREVIEW_ITEMS`, and derive video sidecar frame paths from same-stem image children when available.

- [x] **Step 3: Add Project Panel cache**

Add `folder_media_previews` beside `folder_file_counts`, clear it on worktree entry/settings changes, and retain per-worktree entries on removal.

### Task 3: GPUI Preview Rendering

**Files:**
- Modify: `crates/project_panel/src/media_preview.rs`
- Modify: `crates/project_panel/src/project_panel.rs`

- [x] **Step 1: Render preview cards**

Render image cards with direct local `img(path)` previews. Render video cards with sidecar frame images when available, plus a play badge; otherwise render a compact play placeholder.

- [x] **Step 2: Attach previews without changing tree behavior**

Add `media_preview` to `EntryDetails`, populate it only when `entry.kind.is_dir()` and the entry is expanded, and render the compact preview inside the existing `ListItem` end slot while blocking mouse interactions inside the preview so row selection/open/toggle behavior stays on the tree row and `uniform_list` row heights remain stable.

### Task 4: Handoff Docs And Verification

**Files:**
- Modify: `DX.md`
- Modify: `todo.txt`
- Modify: `changelog.txt`

- [x] **Step 1: Update handoff files**

Record the source-only media preview implementation, bounded verification, skipped runtime/Cargo proof, and exact next governed runtime proof step.

- [x] **Step 2: Run focused checks**

Run only allowed lightweight checks:
- `node --test script/dx-project-panel-source.test.ts`
- `git diff --check`
- targeted conflict-marker scan on touched files

Expected: source guard and diff hygiene pass; Cargo/build/runtime proof remains skipped by direct instruction.
