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
