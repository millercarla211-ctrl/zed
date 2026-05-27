import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const sliceBetween = (source: string, start: string, end: string) => {
  const startIndex = source.indexOf(start);
  assert.notEqual(startIndex, -1, `missing start marker: ${start}`);
  const endIndex = source.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `missing end marker after ${start}: ${end}`);
  return source.slice(startIndex, endIndex);
};

test("Agent panel DX launch status reads the deferred workspace snapshot", () => {
  const source = read("crates/agent_ui/src/agent_panel.rs");

  assert.match(source, /dx_workspace_snapshot: DxWorkspaceSnapshot,/);
  assert.match(source, /struct DxWorkspaceSnapshot \{\s+roots: Vec<String>,\s+has_no_editor_file: bool,\s+has_editor_and_browser: bool,\s+\}/);
  assert.match(source, /let dx_workspace_snapshot = DxWorkspaceSnapshot::from_workspace\(workspace, cx\);/);
  assert.match(source, /this\.schedule_dx_workspace_snapshot_refresh\(cx\);/);
  assert.match(
    source,
    /cx\.defer\(move \|cx\| \{\s+let snapshot = \{\s+let workspace = workspace\.read\(cx\);\s+DxWorkspaceSnapshot::from_workspace\(workspace, cx\)\s+\};/,
  );

  const status = sliceBetween(
    source,
    "fn dx_launch_workspace_status(&self",
    "fn dx_active_status(&self",
  );
  assert.match(status, /let workspace_roots = self\.dx_workspace_snapshot\.roots\.clone\(\);/);
  assert.doesNotMatch(status, /self\.workspace\.upgrade\(\)/);
  assert.doesNotMatch(status, /workspace\.read\(cx\)/);
  assert.doesNotMatch(status, /DxWorkspaceSnapshot::from_workspace/);

  const noEditorFile = sliceBetween(
    source,
    "fn workspace_has_no_editor_file(&self",
    "fn workspace_has_editor_and_browser(&self",
  );
  assert.match(noEditorFile, /self\.dx_workspace_snapshot\.has_no_editor_file/);
  assert.doesNotMatch(noEditorFile, /self\.workspace\.upgrade\(\)/);
  assert.doesNotMatch(noEditorFile, /\.read\(cx\)/);

  const editorAndBrowser = sliceBetween(
    source,
    "fn workspace_has_editor_and_browser(&self",
    "fn should_use_dx_coding_panel_size(&self",
  );
  assert.match(editorAndBrowser, /self\.dx_workspace_snapshot\.has_editor_and_browser/);
  assert.doesNotMatch(editorAndBrowser, /self\.workspace\.upgrade\(\)/);
  assert.doesNotMatch(editorAndBrowser, /\.read\(cx\)/);
});

test("MultiWorkspace active-workspace event carries the active workspace handle", () => {
  const source = read("crates/workspace/src/multi_workspace.rs");

  assert.match(
    source,
    /ActiveWorkspaceChanged \{\s+active_workspace: WeakEntity<Workspace>,\s+source_workspace: Option<WeakEntity<Workspace>>,\s+\}/,
  );
  assert.match(
    source,
    /cx\.emit\(MultiWorkspaceEvent::ActiveWorkspaceChanged \{\s+active_workspace: self\.active_workspace\.downgrade\(\),\s+source_workspace,\s+\}\);/,
  );
  assert.doesNotMatch(
    source,
    /cx\.emit\(MultiWorkspaceEvent::ActiveWorkspaceChanged \{\s+source_workspace,\s+\}\);/,
  );
  assert.match(
    source,
    /WorkspaceRemoved \{\s+removed_workspace: EntityId,\s+active_workspace: WeakEntity<Workspace>,\s+\}/,
  );
  assert.match(
    source,
    /cx\.emit\(MultiWorkspaceEvent::WorkspaceRemoved \{\s+removed_workspace: workspace\.entity_id\(\),\s+active_workspace: self\.active_workspace\.downgrade\(\),\s+\}\);/,
  );
});

test("Sidebar active workspace helpers use the cached event payload", () => {
  const source = read("crates/sidebar/src/sidebar.rs");

  assert.match(source, /active_workspace: Option<Entity<Workspace>>,/);
  assert.match(
    source,
    /MultiWorkspaceEvent::ActiveWorkspaceChanged \{\s+active_workspace, \.\.\s+\} => \{\s+this\.active_workspace = active_workspace\.upgrade\(\);/,
  );
  assert.match(
    source,
    /MultiWorkspaceEvent::WorkspaceRemoved \{\s+removed_workspace,\s+active_workspace,\s+\} => \{\s+this\.forget_cached_workspace\(\*removed_workspace\);\s+this\.active_workspace = active_workspace\.upgrade\(\);/,
  );
  assert.match(
    source,
    /active_workspace: Some\(multi_workspace\.read\(cx\)\.workspace\(\)\.clone\(\)\),/,
  );

  const syncHelper = sliceBetween(
    source,
    "fn sync_active_entry_from_active_workspace(&mut self",
    "fn replace_archived_panel_thread(&mut self",
  );
  assert.match(syncHelper, /\.active_workspace\(cx\)/);
  assert.doesNotMatch(syncHelper, /self\.multi_workspace\.upgrade\(\)/);
  assert.doesNotMatch(syncHelper, /multi_workspace\.read\(cx\)\.workspace\(\)/);

  const activeWorkspaceHelper = sliceBetween(
    source,
    "fn active_workspace(&self",
    "fn focus_agent_panel(&self",
  );
  assert.match(activeWorkspaceHelper, /self\.active_workspace\.clone\(\)/);
  assert.doesNotMatch(activeWorkspaceHelper, /self\.multi_workspace\.upgrade\(\)/);
  assert.doesNotMatch(activeWorkspaceHelper, /multi_workspace\.read\(cx\)\.workspace\(\)/);

  const showArchive = sliceBetween(
    source,
    "fn show_archive(&mut self",
    "fn show_thread_list(&mut self",
  );
  assert.match(showArchive, /self\.active_workspace\(cx\)/);
  assert.doesNotMatch(showArchive, /self\.multi_workspace\.upgrade\(\)/);
  assert.doesNotMatch(showArchive, /multi_workspace\.read\(cx\)\.workspace\(\)/);
});

test("Call integration consumes active workspace events without re-reading them", () => {
  const source = read("crates/call/src/call_impl/mod.rs");

  assert.match(
    source,
    /MultiWorkspaceEvent::ActiveWorkspaceChanged \{\s+active_workspace, \.\.\s+\} => active_workspace\.upgrade\(\),/,
  );
  assert.match(source, /Some\(multi_workspace\.workspace\(\)\.clone\(\)\)/);
  assert.doesNotMatch(source, /multi_workspace\.workspace\(\)\.read\(cx\)\.project\(\)/);
});
