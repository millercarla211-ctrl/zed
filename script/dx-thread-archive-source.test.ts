import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

test("thread archive list caps query normalization and row materialization", () => {
  const source = read("crates/agent_ui/src/threads_archive_view.rs");

  assert.match(source, /const THREAD_ARCHIVE_FILTER_QUERY_CHAR_LIMIT: usize = 256;/);
  assert.match(source, /const THREAD_ARCHIVE_LIST_ENTRY_LIMIT: usize = 1_000;/);
  assert.match(source, /const THREAD_ARCHIVE_BRANCH_NAME_LIMIT: usize = 64;/);
  assert.match(source, /fn bounded_thread_archive_filter_query\(query: &str\) -> String/);
  assert.match(source, /fn collect_bounded_recent_thread_entries<'a>\(/);
  assert.match(source, /if sessions\.len\(\) < THREAD_ARCHIVE_LIST_ENTRY_LIMIT/);
  assert.match(source, /min_by_key\(\|\(_, session\)\| thread_archive_sort_key\(session\)\)/);
  assert.match(source, /sessions\[oldest_ix\] = thread\.clone\(\);/);
  assert.match(source, /sessions\.sort_by_key\(\|thread\| std::cmp::Reverse\(thread_archive_sort_key\(thread\)\)\);/);
  assert.match(source, /\.chars\(\)\s*\.take\(THREAD_ARCHIVE_FILTER_QUERY_CHAR_LIMIT\)[\s\S]*\.to_lowercase\(\)/);
  assert.match(source, /let query_text = self\.filter_editor\.read\(cx\)\.text\(cx\);[\s\S]*let query = bounded_thread_archive_filter_query\(&query_text\);/);
  assert.match(source, /let sessions =\s*collect_bounded_recent_thread_entries\(store\.entries\(\)\.filter\(\s*\|t\| match thread_filter/);
  assert.doesNotMatch(source, /\.take\(THREAD_ARCHIVE_LIST_ENTRY_LIMIT\)[\s\S]*\.cloned\(\)[\s\S]*\.collect::<Vec<_>>\(\);/);
  assert.match(source, /\.iter\(\)[\s\S]*\.take\(THREAD_ARCHIVE_BRANCH_NAME_LIMIT\)[\s\S]*\.collect\(\)/);
});

test("project picker caps recent workspace queries, candidates, and path labels", () => {
  const source = read("crates/agent_ui/src/threads_archive_view.rs");

  assert.match(source, /const PROJECT_PICKER_WORKSPACE_LIMIT: usize = 512;/);
  assert.match(source, /const PROJECT_PICKER_DB_WORKSPACE_ROW_LIMIT: usize = PROJECT_PICKER_WORKSPACE_LIMIT \* 4;/);
  assert.match(source, /const PROJECT_PICKER_QUERY_CHAR_LIMIT: usize = 256;/);
  assert.match(source, /const PROJECT_PICKER_CANDIDATE_LIMIT: usize = 256;/);
  assert.match(source, /const PROJECT_PICKER_PATH_LABEL_LIMIT: usize = 24;/);
  assert.match(source, /const PROJECT_PICKER_PATH_TEXT_CHAR_LIMIT: usize = 256;/);
  assert.match(source, /fn bounded_project_picker_query\(query: &str\) -> String/);
  assert.match(source, /fn project_picker_candidate_string\(workspace: &RecentWorkspace\) -> String/);
  assert.match(source, /recent_project_workspaces_limited\(\s*fs\.as_ref\(\),\s*PROJECT_PICKER_DB_WORKSPACE_ROW_LIMIT,\s*\)/);
  assert.match(source, /workspaces\.truncate\(PROJECT_PICKER_WORKSPACE_LIMIT\);/);
  assert.match(source, /let bounded_query = bounded_project_picker_query\(query\.trim_start\(\)\);[\s\S]*let query = bounded_query\.as_str\(\);/);
  assert.match(source, /\.filter\(\|\(_, workspace\)\| self\.is_sibling_workspace\(workspace\.workspace_id\)\)[\s\S]*\.take\(PROJECT_PICKER_CANDIDATE_LIMIT\)[\s\S]*StringMatchCandidate::new\(id, &combined_string\)/);
  assert.match(source, /\.filter\(\|\(_, workspace\)\| \{[\s\S]*!self\.is_sibling_workspace\(workspace\.workspace_id\)[\s\S]*\}\)[\s\S]*\.take\(PROJECT_PICKER_CANDIDATE_LIMIT\)[\s\S]*StringMatchCandidate::new\(id, &combined_string\)/);
  assert.match(source, /self\s*\.workspaces\s*\.iter\(\)\s*\.enumerate\(\)[\s\S]*\.take\(PROJECT_PICKER_CANDIDATE_LIMIT\)/);
  assert.match(source, /\.ordered_paths\(\)\s*\.take\(PROJECT_PICKER_PATH_LABEL_LIMIT\)/);
});

test("thread import caps agent/session materialization before UI or import lists", () => {
  const source = read("crates/agent_ui/src/thread_import.rs");

  assert.match(source, /const THREAD_IMPORT_AGENT_ENTRY_LIMIT: usize = 128;/);
  assert.match(source, /const THREAD_IMPORT_AGENT_SORT_KEY_CHAR_LIMIT: usize = 128;/);
  assert.match(source, /const THREAD_IMPORT_AGENT_ROW_LIMIT: usize = 128;/);
  assert.match(source, /const THREAD_IMPORT_CONNECTION_STORE_LIMIT: usize = 16;/);
  assert.match(source, /const THREAD_IMPORT_SESSION_LIMIT_PER_AGENT: usize = 1_000;/);
  assert.match(source, /const THREAD_IMPORT_THREAD_INSERT_LIMIT: usize = 2_000;/);
  assert.match(source, /fn thread_import_sort_key\(display_name: &str\) -> String/);
  assert.match(source, /\.external_agents\(\)\s*\.take\(THREAD_IMPORT_AGENT_ENTRY_LIMIT\)[\s\S]*agent_entries\s*\.sort_unstable_by_key\(\|entry\| thread_import_sort_key\(entry\.display_name\.as_ref\(\)\)\);/);
  assert.match(source, /\.agent_entries\s*\.iter\(\)\s*\.take\(THREAD_IMPORT_AGENT_ROW_LIMIT\)\s*\.enumerate\(\)/);
  assert.match(source, /if stores\.len\(\) >= THREAD_IMPORT_CONNECTION_STORE_LIMIT \{\s*break;\s*\}/);
  assert.match(source, /let remaining = THREAD_IMPORT_SESSION_LIMIT_PER_AGENT\.saturating_sub\(sessions\.len\(\)\);[\s\S]*sessions\.extend\(response\.sessions\.into_iter\(\)\.take\(remaining\)\);/);
  assert.match(source, /if to_insert\.len\(\) >= THREAD_IMPORT_THREAD_INSERT_LIMIT \{\s*break;\s*\}/);
});

test("thread worktree archive caps workspace, repository, and linked-thread collection", () => {
  const source = read("crates/agent_ui/src/thread_worktree_archive.rs");

  assert.match(source, /const THREAD_ARCHIVE_OPEN_WORKSPACE_LIMIT: usize = 128;/);
  assert.match(source, /const THREAD_ARCHIVE_WORKSPACE_SET_LIMIT: usize = 128;/);
  assert.match(source, /const THREAD_ARCHIVE_AFFECTED_PROJECT_LIMIT: usize = 128;/);
  assert.match(source, /const THREAD_ARCHIVE_REPOSITORY_SCAN_LIMIT: usize = 128;/);
  assert.match(source, /const THREAD_ARCHIVE_LINKED_THREAD_LIMIT: usize = 512;/);
  assert.match(source, /\.filter_map\(\|workspace\| \{[\s\S]*Some\(AffectedProject \{[\s\S]*\}\)[\s\S]*\}\)\s*\.take\(THREAD_ARCHIVE_AFFECTED_PROJECT_LIMIT\)\s*\.collect::<Vec<_>>\(\);/);
  assert.match(source, /\.repositories\(cx\)[\s\S]*\.values\(\)[\s\S]*\.cloned\(\)[\s\S]*\.take\(THREAD_ARCHIVE_REPOSITORY_SCAN_LIMIT\)[\s\S]*\.collect::<Vec<_>>\(\)/);
  assert.match(source, /if thread_ids\.len\(\) >= THREAD_ARCHIVE_LINKED_THREAD_LIMIT \{[\s\S]*anyhow::bail!/);
  assert.match(source, /thread_ids\.push\(thread\.thread_id\);/);
  assert.doesNotMatch(source, /\.take\(THREAD_ARCHIVE_LINKED_THREAD_LIMIT\)[\s\S]*\.map\(\|thread\| thread\.thread_id\)[\s\S]*\.collect\(\)/);
  assert.match(source, /if workspaces\.len\(\) >= THREAD_ARCHIVE_OPEN_WORKSPACE_LIMIT \{\s*break;\s*\}/);
  assert.match(source, /workspaces\.truncate\(THREAD_ARCHIVE_WORKSPACE_SET_LIMIT\);/);
});
