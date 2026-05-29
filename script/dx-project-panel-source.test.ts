import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const functionBody = (source: string, name: string) => {
  const start = source.search(new RegExp(`fn\\s+${name}(?:<[^>]+>)?\\s*\\(`));
  assert.ok(start >= 0, `expected ${name}`);

  const bodyStart = source.indexOf("{", start);
  assert.ok(bodyStart > start, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(start, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
};

const assertBefore = ({
  body,
  before,
  after,
  message,
}: {
  body: string;
  before: string | RegExp;
  after: string | RegExp;
  message: string;
}) => {
  const beforeIndex =
    typeof before === "string" ? body.indexOf(before) : body.match(before)?.index ?? -1;
  const afterIndex =
    typeof after === "string" ? body.indexOf(after) : body.match(after)?.index ?? -1;
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("project panel visible tree materialization has named caps before collection", () => {
  const source = read("crates/project_panel/src/project_panel.rs");
  const updateVisibleEntries = functionBody(source, "update_visible_entries");

  assert.match(source, /const MAX_PROJECT_PANEL_VISIBLE_WORKTREES: usize = 256;/);
  assert.match(source, /const MAX_PROJECT_PANEL_VISIBLE_ENTRIES: usize = 200_000;/);
  assert.match(source, /const MAX_PROJECT_PANEL_VISIBLE_ENTRIES_PER_WORKTREE: usize = 50_000;/);
  assert.match(source, /fn project_panel_cap_hit\(boundary: &'static str, cap: usize\)/);
  assertBefore({
    body: updateVisibleEntries,
    before: ".take(MAX_PROJECT_PANEL_VISIBLE_WORKTREES)",
    after: ".collect()",
    message: "visible worktrees must be capped before snapshot vector collection",
  });
  assertBefore({
    body: updateVisibleEntries,
    before:
      /visible_worktree_entries\.len\(\)\s*>=\s*MAX_PROJECT_PANEL_VISIBLE_ENTRIES_PER_WORKTREE/,
    after: "visible_worktree_entries.push(entry.to_owned())",
    message: "per-worktree entries must be capped before visible row pushes",
  });
  assertBefore({
    body: updateVisibleEntries,
    before: "visible_entries_total >= MAX_PROJECT_PANEL_VISIBLE_ENTRIES",
    after: "new_state.visible_entries.push(VisibleEntriesForWorktree",
    message: "total visible entries must be capped before state materialization",
  });
});

test("project panel expansion and selection fanout is bounded", () => {
  const source = read("crates/project_panel/src/project_panel.rs");
  const expandAllForEntry = functionBody(source, "expand_all_for_entry");
  const collapseAllForEntry = functionBody(source, "collapse_all_for_entry");
  const renderEntry = functionBody(source, "render_entry");
  const pushExpandedDir = functionBody(source, "push_project_panel_expanded_dir");

  assert.match(source, /const MAX_PROJECT_PANEL_EXPANDED_DIRS_PER_WORKTREE: usize = 50_000;/);
  assert.match(source, /const MAX_PROJECT_PANEL_SELECTION_RANGE_ENTRIES: usize = 20_000;/);
  assert.match(source, /fn push_project_panel_expanded_dir\(/);
  assert.match(
    expandAllForEntry,
    /push_project_panel_expanded_dir\(expanded_dir_ids, entry\.id\)/,
    "entry expansion must use the capped insert helper",
  );
  assertBefore({
    body: pushExpandedDir,
    before: "expanded_dir_ids.len() >= MAX_PROJECT_PANEL_EXPANDED_DIRS_PER_WORKTREE",
    after: "expanded_dir_ids.insert",
    message: "expanded directory ids must check the cap before sorted insertion",
  });
  assertBefore({
    body: collapseAllForEntry,
    before:
      /dirs_to_collapse\.len\(\)\s*>=\s*MAX_PROJECT_PANEL_EXPANDED_DIRS_PER_WORKTREE/,
    after: "dirs_to_collapse.push(child.id)",
    message: "recursive collapse worklists must be capped before push",
  });
  assertBefore({
    body: renderEntry,
    before: "MAX_PROJECT_PANEL_SELECTION_RANGE_ENTRIES",
    after: "for_each_visible_entry",
    message: "shift range selection must be capped before visible-entry materialization",
  });
});

test("project panel previous selection uses checked visible-entry lookups", () => {
  const source = read("crates/project_panel/src/project_panel.rs");
  const selectPrevious = functionBody(source, "select_previous");

  assert.doesNotMatch(
    selectPrevious,
    /visible_entries\s*\[\s*worktree_ix\s*\]/,
    "select_previous must not index visible_entries with a stale worktree_ix",
  );
  assert.doesNotMatch(
    selectPrevious,
    /entries\s*\[\s*entry_ix\s*\]/,
    "select_previous must not index entries with a stale entry_ix",
  );
  assertBefore({
    body: selectPrevious,
    before: /\.visible_entries\s*\.\s*get\(\s*worktree_ix\s*\)/,
    after: "let selection = SelectedEntry",
    message: "select_previous must check the target worktree before creating a selection",
  });
  assertBefore({
    body: selectPrevious,
    before: /entries\.get\(\s*entry_ix\s*\)/,
    after: "let selection = SelectedEntry",
    message: "select_previous must check the target entry before creating a selection",
  });
});

test("project panel active indent guide uses checked visible-entry lookups", () => {
  const source = read("crates/project_panel/src/project_panel.rs");
  const findActiveIndentGuide = functionBody(source, "find_active_indent_guide");

  assert.doesNotMatch(
    findActiveIndentGuide,
    /visible_entries\s*\[\s*worktree_ix\s*\]/,
    "active indent guide lookup must not index visible_entries with a stale worktree_ix",
  );
  assertBefore({
    body: findActiveIndentGuide,
    before: /\.visible_entries\s*\.\s*get\(\s*worktree_ix\s*\)/,
    after: "let child_paths =",
    message: "active indent guide lookup must check the target worktree before reading entries",
  });
});

test("project panel visible-entry range materialization skips stale ranges", () => {
  const source = read("crates/project_panel/src/project_panel.rs");
  const iterVisibleEntries = functionBody(source, "iter_visible_entries");
  const forEachVisibleEntry = functionBody(source, "for_each_visible_entry");

  assert.doesNotMatch(
    iterVisibleEntries,
    /visible\.entries\s*\[\s*entry_range\s*\]/,
    "iter_visible_entries must not directly slice a potentially stale visible entry range",
  );
  assertBefore({
    body: iterVisibleEntries,
    before: /visible\.entries\s*\.\s*get\(\s*entry_range\s*\)/,
    after: "for (i, entry)",
    message: "iter_visible_entries must check the entry range before iterating it",
  });
  assert.doesNotMatch(
    forEachVisibleEntry,
    /visible\.entries\s*\[\s*entry_range\s*\]/,
    "for_each_visible_entry must not directly slice a potentially stale visible entry range",
  );
  assertBefore({
    body: forEachVisibleEntry,
    before: /visible\.entries\s*\.\s*get\(\s*entry_range\s*\)/,
    after: "let status =",
    message: "for_each_visible_entry must check the entry range before materializing details",
  });
});

test("project panel edit-state display handles stale ancestor relations", () => {
  const source = read("crates/project_panel/src/project_panel.rs");
  const forEachVisibleEntry = functionBody(source, "for_each_visible_entry");

  assert.doesNotMatch(
    forEachVisibleEntry,
    /\.expect\(\s*"Edited sub-entry should be an ancestor of selected leaf entry"\s*\)/,
    "edit-state display must not panic when a stale leaf no longer contains the edited ancestor",
  );
  assertBefore({
    body: forEachVisibleEntry,
    before:
      /if let Some\(position\)\s*=\s*ancestors\s*\.\s*ancestors\s*\.\s*iter\(\)\s*\.\s*position\(\|entry_id\|\s*\*entry_id\s*==\s*edit_state\.entry_id\)/,
    after: "let all_components = ancestors.ancestors.len();",
    message: "edit-state display must check the edited ancestor position before deriving path components",
  });
  assertBefore({
    body: forEachVisibleEntry,
    before:
      /if let Some\(position\)\s*=\s*ancestors\s*\.\s*ancestors\s*\.\s*iter\(\)\s*\.\s*position\(\|entry_id\|\s*\*entry_id\s*==\s*edit_state\.entry_id\)/,
    after: /details\s*\.\s*filename\s*\.\s*push_str\(\s*processing_filename\.as_unix_str\(\)\s*\)/,
    message: "edit-state display must fall back to the processing filename when ancestor lookup is stale",
  });
});

test("project panel drag, drop, and download materialization is bounded", () => {
  const source = read("crates/project_panel/src/project_panel.rs");
  const dropExternalFiles = functionBody(source, "drop_external_files");
  const dragOnto = functionBody(source, "drag_onto");
  const paste = functionBody(source, "paste");
  const downloadFromRemote = functionBody(source, "download_from_remote");

  assert.match(source, /const MAX_PROJECT_PANEL_EXTERNAL_DROP_PATHS: usize = 4_096;/);
  assert.match(source, /const MAX_PROJECT_PANEL_DRAG_SELECTION_ENTRIES: usize = 4_096;/);
  assert.match(source, /const MAX_PROJECT_PANEL_DOWNLOAD_FILES: usize = 10_000;/);
  assertBefore({
    body: dropExternalFiles,
    before: ".take(MAX_PROJECT_PANEL_EXTERNAL_DROP_PATHS)",
    after: "paths_to_replace.push",
    message: "external drops must be bounded before replacement and copy vectors",
  });
  assertBefore({
    body: dragOnto,
    before: "cap_project_panel_entry_set(",
    after: "copy_tasks.push(task)",
    message: "drag selections must be bounded before copy task fanout",
  });
  assertBefore({
    body: paste,
    before: ".take(MAX_PROJECT_PANEL_DRAG_SELECTION_ENTRIES)",
    after: "paste_tasks.push(task)",
    message: "paste selections must be bounded before task fanout",
  });
  assertBefore({
    body: downloadFromRemote,
    before: "files_to_download.len() >= MAX_PROJECT_PANEL_DOWNLOAD_FILES",
    after: "files_to_download.push",
    message: "remote download lists must be bounded before recursive file collection",
  });
});

test("project panel display strings, sticky rows, and undo batches are bounded", () => {
  const source = read("crates/project_panel/src/project_panel.rs");
  const utils = read("crates/project_panel/src/utils.rs");
  const undo = read("crates/project_panel/src/undo.rs");
  const detailsForEntry = functionBody(source, "details_for_entry");
  const renderStickyEntries = functionBody(source, "render_sticky_entries");
  const record = functionBody(undo, "record");

  assert.match(utils, /pub\(crate\) const MAX_PROJECT_PANEL_DISPLAY_LABEL_CHARS: usize = 1_024;/);
  assert.match(utils, /pub\(crate\) fn bounded_project_panel_label\(/);
  assert.match(source, /const MAX_PROJECT_PANEL_STICKY_PARENTS: usize = 128;/);
  assert.match(undo, /const MAX_PROJECT_PANEL_UNDO_BATCH_CHANGES: usize = 4_096;/);
  assert.match(detailsForEntry, /utils::bounded_project_panel_label\(filename\)/);
  assertBefore({
    body: renderStickyEntries,
    before: "sticky_parents.len() >= MAX_PROJECT_PANEL_STICKY_PARENTS",
    after: "sticky_parents.push",
    message: "sticky parent rows must be capped before vector push",
  });
  assertBefore({
    body: record,
    before: ".take(MAX_PROJECT_PANEL_UNDO_BATCH_CHANGES + 1)",
    after: "UndoMessage::Changed(changes)",
    message: "undo batches must be capped before sending to the manager task",
  });
});

test("project panel media preview is lazy, bounded, and preserves normal tree rows", () => {
  const source = read("crates/project_panel/src/project_panel.rs");
  const media = read("crates/project_panel/src/media_preview.rs");
  const detailsForEntry = functionBody(source, "details_for_entry");
  const renderEntry = functionBody(source, "render_entry");

  assert.match(source, /mod media_preview;/);
  assert.match(source, /folder_media_previews:\s*RefCell<HashMap<\(WorktreeId, ProjectEntryId\), Option<media_preview::FolderMediaPreview>>>/);
  assert.match(source, /media_preview:\s*Option<media_preview::FolderMediaPreview>/);

  assert.match(media, /pub\(crate\) const MAX_PROJECT_PANEL_MEDIA_CHILD_SCAN: usize = 512;/);
  assert.match(media, /pub\(crate\) const MAX_PROJECT_PANEL_MEDIA_PREVIEW_ITEMS: usize = 12;/);
  assert.match(media, /pub\(crate\) const MAX_PROJECT_PANEL_MEDIA_INLINE_CARDS: usize = 4;/);
  assert.match(media, /pub\(crate\) enum MediaPreviewKind/);
  assert.match(media, /Image/);
  assert.match(media, /Video/);
  assert.match(media, /Audio/);
  assert.match(media, /fn video_preview_frame_path/);
  assert.match(media, /fn media_stem_key/);
  assert.match(media, /fn media_preview_card_tooltip_meta/);
  assert.match(media, /video_frame_path:\s*Option<PathBuf>/);
  assert.match(media, /audio_duration_label:\s*Option<String>/);
  assert.match(
    source,
    /let preview = media_preview::build_folder_media_preview\(parent_abs_path, children\);[\s\S]*insert\(cache_key, preview\.clone\(\)\);[\s\S]*preview/,
    "media preview cache must store both populated previews and no-media misses",
  );

  assertBefore({
    body: detailsForEntry,
    before: /entry\.kind\.is_dir\(\)\s*&&\s*is_expanded/,
    after: /self\.folder_media_preview\(/,
    message: "media previews must be built only after confirming an expanded directory",
  });
  const mediaPreviewBranch = detailsForEntry.match(
    /let media_preview = if entry\.kind\.is_dir\(\) && is_expanded \{[\s\S]*?\n        \} else \{\n            None\n        \};/,
  );
  assert.ok(
    mediaPreviewBranch,
    "details_for_entry must isolate media probing inside the expanded-directory branch",
  );
  assert.match(mediaPreviewBranch[0], /self\.folder_media_preview\(/);
  assert.doesNotMatch(
    detailsForEntry.replace(mediaPreviewBranch[0], ""),
    /self\.folder_media_preview\(/,
    "details_for_entry must not probe media outside the expanded-directory branch",
  );
  assertBefore({
    body: renderEntry,
    before: /let media_preview = \(!is_sticky\)\s*\.then\(\|\| details\.media_preview\.clone\(\)\)\s*\.flatten\(\);/,
    after: /media_preview::render_folder_media_preview/,
    message: "render_entry must use the cached media preview instead of probing from render code",
  });
  assertBefore({
    body: renderEntry,
    before: /\.end_slot::<AnyElement>/,
    after: /media_preview::render_folder_media_preview/,
    message: "media previews must render inside the existing uniform-height row end slot",
  });
  assert.doesNotMatch(
    renderEntry,
    /\.when\(!is_sticky && kind\.is_dir\(\) && is_expanded[\s\S]*media_preview::render_folder_media_preview/,
    "media previews must not add variable-height children under uniform_list rows",
  );
  assert.match(renderEntry, /block_mouse_except_scroll\(\)/);
});

test("project panel media preview renders direct image previews and video frames when available", () => {
  const media = read("crates/project_panel/src/media_preview.rs");
  const renderFolderMediaPreview = functionBody(media, "render_folder_media_preview");
  const renderMediaPreviewCard = functionBody(media, "render_media_preview_card");
  const buildFolderMediaPreview = functionBody(media, "build_folder_media_preview");

  assertBefore({
    body: buildFolderMediaPreview,
    before: /children\.take\(MAX_PROJECT_PANEL_MEDIA_CHILD_SCAN \+ 1\)/,
    after: /scanned_cap_hit/,
    message: "media child scans must be capped before classification work",
  });
  assertBefore({
    body: buildFolderMediaPreview,
    before: /items\.len\(\)\s*<\s*MAX_PROJECT_PANEL_MEDIA_PREVIEW_ITEMS/,
    after: /items\.push/,
    message: "media preview items must be capped before render data collection",
  });
  assertBefore({
    body: renderFolderMediaPreview,
    before: /preview\s*\.items\s*\.iter\(\)/,
    after: /take\(MAX_PROJECT_PANEL_MEDIA_INLINE_CARDS\)/,
    message: "folder media preview must cap inline cards before render mapping",
  });
  assertBefore({
    body: renderFolderMediaPreview,
    before: /take\(MAX_PROJECT_PANEL_MEDIA_INLINE_CARDS\)/,
    after: /render_media_preview_card/,
    message: "folder media preview must render from bounded preview items",
  });
  assert.match(renderFolderMediaPreview, /\.h_6\(\)/);
  assert.match(renderFolderMediaPreview, /\.overflow_hidden\(\)/);
  assert.match(renderFolderMediaPreview, /Tooltip::with_meta/);
  assert.match(
    renderMediaPreviewCard,
    /MediaPreviewKind::Image[\s\S]*img\(item\.absolute_path\.clone\(\)\)[\s\S]*object_fit\(ObjectFit::Cover\)/,
    "image media cards must render direct visual previews from local paths",
  );
  assert.match(
    renderMediaPreviewCard,
    /MediaPreviewKind::Video[\s\S]*item\.video_frame_path\.as_ref\(\)[\s\S]*img\(frame_path\.clone\(\)\)[\s\S]*IconName::PlayOutlined/,
    "video media cards must use representative frame images when available and keep a play affordance",
  );
  assert.match(
    renderMediaPreviewCard,
    /MediaPreviewKind::Video[\s\S]*Icon::new\(IconName::PlayOutlined\)/,
    "video media cards need a lightweight fallback when no preview frame exists",
  );
  assert.match(
    renderMediaPreviewCard,
    /let tooltip_title = item\.name\.clone\(\);[\s\S]*MediaPreviewKind::Audio[\s\S]*Duration unavailable[\s\S]*Tooltip::with_meta\(tooltip_title\.clone\(\), None, tooltip_meta\.clone\(\), cx\)/,
    "audio media cards must render an honest duration state and keep the filename in the tooltip",
  );
  assert.doesNotMatch(
    media,
    /path\.is_file\(\)|std::fs::metadata|fs::metadata/,
    "media preview classification must stay snapshot-derived and avoid UI-path filesystem probes",
  );
});
