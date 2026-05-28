import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

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

function sliceBetween(haystack: string, start: string, end: string): string {
  const startIndex = haystack.indexOf(start);
  assert.notEqual(startIndex, -1, `expected ${start}`);
  const endIndex = haystack.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `expected ${end} after ${start}`);
  return haystack.slice(startIndex, endIndex);
}

function assertBefore(
  haystack: string,
  before: string | RegExp,
  after: string | RegExp,
  message: string,
) {
  const beforeIndex =
    typeof before === "string"
      ? haystack.indexOf(before)
      : haystack.match(before)?.index ?? -1;
  const afterIndex =
    typeof after === "string"
      ? haystack.indexOf(after)
      : haystack.match(after)?.index ?? -1;
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("branch picker declares and uses bounded branch materialization caps", () => {
  const source = productionSource(read("crates/git_ui/src/branch_picker.rs"));
  const processBranches = functionBody(source, "process_branches");
  const updateMatches = sliceBetween(
    source,
    "fn update_matches(",
    "fn confirm(",
  );
  const renderMatch = functionBody(source, "render_match");

  assert.match(source, /const MAX_BRANCH_PICKER_BRANCHES: usize = 2_048;/);
  assert.match(
    source,
    /const MAX_BRANCH_PICKER_MATCH_CANDIDATES: usize = MAX_BRANCH_PICKER_BRANCHES;/,
  );
  assert.match(source, /const MAX_BRANCH_PICKER_MATCHES: usize = 512;/);
  assert.match(source, /const MAX_BRANCH_PICKER_LABEL_CHARS: usize = 512;/);
  assert.match(source, /fn bounded_branch_label\(label: &str\) -> String/);
  assertBefore(
    processBranches,
    ".take(MAX_BRANCH_PICKER_BRANCHES)",
    "remote_upstreams",
    "branch upstream de-duplication must only inspect a bounded branch slice",
  );
  assertBefore(
    processBranches,
    "branch.is_head",
    "remote_upstreams",
    "the current branch must be preserved before upstream de-duplication runs",
  );
  assertBefore(
    processBranches,
    "preserved_branch == branch.name()",
    "remote_upstreams",
    "the selected branch must be preserved before upstream de-duplication runs",
  );
  assertBefore(
    updateMatches,
    ".take(MAX_BRANCH_PICKER_MATCH_CANDIDATES)",
    "StringMatchCandidate::new",
    "branch fuzzy candidates must be capped before candidate allocation",
  );
  assertBefore(
    updateMatches,
    "MAX_BRANCH_PICKER_MATCHES",
    "fuzzy_nucleo::match_strings_async",
    "branch fuzzy search must pass a named result cap",
  );
  assert.match(
    renderMatch,
    /HighlightedLabel::new\(bounded_branch_label\(branch\.name\(\)\), positions\.clone\(\)\)/,
    "branch rows must compact branch names before label rendering",
  );
});

test("stash picker bounds stash entries, fuzzy candidates, and rendered labels", () => {
  const source = productionSource(read("crates/git_ui/src/stash_picker.rs"));
  const newInner = functionBody(source, "new_inner");
  const updateMatches = sliceBetween(
    source,
    "fn update_matches(",
    "fn confirm(",
  );
  const renderMatch = functionBody(source, "render_match");

  assert.match(source, /const MAX_STASH_PICKER_ENTRIES: usize = 1_024;/);
  assert.match(
    source,
    /const MAX_STASH_PICKER_MATCH_CANDIDATES: usize = MAX_STASH_PICKER_ENTRIES;/,
  );
  assert.match(source, /const MAX_STASH_PICKER_MATCHES: usize = 512;/);
  assert.match(source, /const MAX_STASH_PICKER_LABEL_CHARS: usize = 512;/);
  assert.match(source, /fn bounded_stash_entries\(stash: &GitStash\) -> Vec<StashEntry>/);
  assert.match(source, /fn bounded_stash_label\(label: &str\) -> String/);
  assert.doesNotMatch(
    newInner,
    /cached_stash\(\)\.entries\.to_vec\(\)/,
    "stash picker should not clone every cached stash entry into UI state",
  );
  assertBefore(
    updateMatches,
    ".take(MAX_STASH_PICKER_MATCH_CANDIDATES)",
    "StringMatchCandidate::new",
    "stash fuzzy candidates must be capped before candidate allocation",
  );
  assertBefore(
    updateMatches,
    "MAX_STASH_PICKER_MATCHES",
    "fuzzy::match_strings",
    "stash fuzzy search must pass a named result cap",
  );
  assert.match(
    renderMatch,
    /Self::bounded_format_message\(entry_match\.entry\.index, &entry_match\.entry\.message\)/,
    "stash rows must compact stash messages before label rendering",
  );
  assert.match(
    renderMatch,
    /let\s+Some\(entry_match\)\s*=\s*self\.matches\.get\(ix\)\s*else\s*\{\s*return\s+None;\s*\};/,
    "stash row rendering must fail closed when a stale row index arrives",
  );
  assert.doesNotMatch(
    renderMatch,
    /self\.matches\s*\[\s*ix\s*\]/,
    "stash row rendering must not direct-index stale row indexes",
  );
});

test("git pickers guard stale fuzzy IDs and selected indexes", () => {
  const branchSource = productionSource(read("crates/git_ui/src/branch_picker.rs"));
  const stashSource = productionSource(read("crates/git_ui/src/stash_picker.rs"));
  const worktreeSource = productionSource(read("crates/git_ui/src/worktree_picker.rs"));

  const branchUpdateMatches = sliceBetween(
    branchSource,
    "fn update_matches(",
    "fn confirm(",
  );
  const branchSetSelectedIndex = functionBody(branchSource, "set_selected_index");
  const branchConfirm = functionBody(branchSource, "confirm");

  assert.match(
    branchUpdateMatches,
    /branches\s*\.get\(candidate\.candidate_id\)\s*\.cloned\(\)/,
    "branch fuzzy results must ignore stale candidate IDs",
  );
  assert.doesNotMatch(
    branchUpdateMatches,
    /branches\s*\[\s*candidate\.candidate_id\s*\]/,
    "branch fuzzy results must not index stale candidate IDs directly",
  );
  assert.match(
    branchSource,
    /fn clamp_selected_index\(&self, ix: usize\) -> usize/,
    "branch picker must have a shared selected-index clamp",
  );
  assert.match(
    branchSetSelectedIndex,
    /self\.selected_index = self\.clamp_selected_index\(ix\);/,
    "branch selected-index setter must clamp against current matches",
  );
  assert.doesNotMatch(
    branchSetSelectedIndex,
    /self\.selected_index = ix;/,
    "branch selected-index setter must not store raw indexes",
  );
  assert.match(
    branchConfirm,
    /self\.matches\.get\(self\.selected_index\(\)\)/,
    "branch confirm must read through the clamped selected index",
  );
  assert.doesNotMatch(
    branchSource,
    /\.delete_at\(picker\.delegate\.selected_index,/,
    "branch delete actions must not pass raw selected indexes",
  );

  const stashUpdateMatches = sliceBetween(
    stashSource,
    "fn update_matches(",
    "fn confirm(",
  );
  const stashSetSelectedIndex = functionBody(stashSource, "set_selected_index");
  const stashConfirm = functionBody(stashSource, "confirm");

  assert.match(
    stashUpdateMatches,
    /all_stash_entries\s*\.get\(candidate\.candidate_id\)\s*\.cloned\(\)/,
    "stash fuzzy results must ignore stale candidate IDs",
  );
  assert.doesNotMatch(
    stashUpdateMatches,
    /all_stash_entries\s*\[\s*candidate\.candidate_id\s*\]/,
    "stash fuzzy results must not index stale candidate IDs directly",
  );
  assert.match(
    stashSource,
    /fn clamp_selected_index\(&self, ix: usize\) -> usize/,
    "stash picker must have a shared selected-index clamp",
  );
  assert.match(
    stashSetSelectedIndex,
    /self\.selected_index = self\.clamp_selected_index\(ix\);/,
    "stash selected-index setter must clamp against current matches",
  );
  assert.doesNotMatch(
    stashSetSelectedIndex,
    /self\.selected_index = ix;/,
    "stash selected-index setter must not store raw indexes",
  );
  assert.match(
    stashConfirm,
    /self\.matches\.get\(self\.selected_index\(\)\)/,
    "stash confirm must read through the clamped selected index",
  );

  const worktreeUpdateMatches = sliceBetween(
    worktreeSource,
    "fn update_matches(",
    "fn confirm(",
  );
  const worktreeSetSelectedIndex = functionBody(worktreeSource, "set_selected_index");
  const worktreeSyncSelectedIndex = functionBody(worktreeSource, "sync_selected_index");
  const worktreeConfirm = functionBody(worktreeSource, "confirm");

  assert.match(
    worktreeUpdateMatches,
    /repo_worktrees_clone\s*\.get\(candidate\.candidate_id\)\s*\.cloned\(\)/,
    "worktree fuzzy results must ignore stale candidate IDs",
  );
  assert.doesNotMatch(
    worktreeUpdateMatches,
    /repo_worktrees_clone\s*\[\s*candidate\.candidate_id\s*\]/,
    "worktree fuzzy results must not index stale candidate IDs directly",
  );
  assert.match(
    worktreeSource,
    /fn clamp_selected_index\(&self, ix: usize\) -> usize/,
    "worktree picker must have a shared selected-index clamp",
  );
  assert.match(
    worktreeSetSelectedIndex,
    /self\.selected_index = self\.clamp_selected_index\(ix\);/,
    "worktree selected-index setter must clamp against current matches",
  );
  assert.doesNotMatch(
    worktreeSetSelectedIndex,
    /self\.selected_index = ix;/,
    "worktree selected-index setter must not store raw indexes",
  );
  assert.match(
    worktreeSyncSelectedIndex,
    /self\.selected_index = self\.clamp_selected_index\(self\.selected_index\);/,
    "worktree selected-index sync must clamp when preserving selection",
  );
  assert.match(
    worktreeConfirm,
    /self\.matches\.get\(self\.selected_index\(\)\)/,
    "worktree confirm must read through the clamped selected index",
  );
  assert.doesNotMatch(
    worktreeSource,
    /let ix = picker\.delegate\.selected_index;/,
    "worktree action handlers must not pass raw selected indexes",
  );
  assert.doesNotMatch(
    worktreeSource,
    /self\.matches\.get\(self\.selected_index\)/,
    "worktree footer and confirm paths must read through the clamped selected index",
  );
});

test("repository selector bounds repository lists and prompt labels", () => {
  const source = productionSource(read("crates/git_ui/src/repository_selector.rs"));
  const newSelector = functionBody(source, "new");
  const updateRepositoryEntries = functionBody(source, "update_repository_entries");
  const updateMatches = sliceBetween(
    source,
    "fn update_matches(",
    "fn confirm(",
  );
  const renderMatch = functionBody(source, "render_match");

  assert.match(source, /const MAX_REPOSITORY_SELECTOR_ENTRIES: usize = 1_024;/);
  assert.match(source, /const MAX_REPOSITORY_SELECTOR_MATCHES: usize = 256;/);
  assert.match(source, /const MAX_REPOSITORY_SELECTOR_LABEL_CHARS: usize = 512;/);
  assert.match(source, /fn bounded_repository_entries\(/);
  assert.match(source, /fn bounded_repository_label\(label: &str\) -> String/);
  assertBefore(
    newSelector,
    "bounded_repository_entries(",
    "let filtered_repositories = repository_entries.clone();",
    "repository selector must cap repositories before picker state is cloned",
  );
  assertBefore(
    updateRepositoryEntries,
    "all_repositories.truncate(MAX_REPOSITORY_SELECTOR_ENTRIES)",
    "self.repository_entries = all_repositories.clone();",
    "repository updates must cap incoming lists before storing picker state",
  );
  assertBefore(
    updateMatches,
    ".take(MAX_REPOSITORY_SELECTOR_MATCHES)",
    ".collect()",
    "repository filtering must cap matches before result vector materialization",
  );
  assert.match(
    renderMatch,
    /Label::new\(bounded_repository_label\(display_name\.as_ref\(\)\)\)/,
    "repository rows must compact display names before rendering",
  );
});

test("git panel bounds status, history, remote, and worktree materialization", () => {
  const source = productionSource(read("crates/git_ui/src/git_panel.rs"));
  const updateVisibleEntries = functionBody(source, "update_visible_entries");
  const fetchCommitHistory = functionBody(source, "fetch_commit_history_shas");
  const getFetchOptions = functionBody(source, "get_fetch_options");
  const getRemote = functionBody(source, "get_remote");
  const gitInit = functionBody(source, "git_init");
  const renderStatusEntry = functionBody(source, "render_status_entry");

  assert.match(source, /const MAX_GIT_PANEL_STATUS_ENTRIES: usize = 20_000;/);
  assert.match(source, /const MAX_GIT_PANEL_COMMIT_HISTORY_SHAS: usize = 1_000;/);
  assert.match(source, /const MAX_GIT_PANEL_REMOTE_PROMPT_OPTIONS: usize = 128;/);
  assert.match(source, /const MAX_GIT_PANEL_WORKTREE_PROMPT_OPTIONS: usize = 256;/);
  assert.match(source, /const MAX_GIT_PANEL_STATUS_LABEL_CHARS: usize = 512;/);
  assert.match(source, /fn bounded_git_panel_label\(label: &str\) -> String/);
  assertBefore(
    updateVisibleEntries,
    "let mut materialized_status_entries = 0usize;",
    "for entry in repo.cached_status()",
    "status materialization must track a named cap before walking cached status",
  );
  assertBefore(
    updateVisibleEntries,
    "if materialized_status_entries >= MAX_GIT_PANEL_STATUS_ENTRIES",
    "let entry = GitStatusEntry",
    "status entries must be capped before row structs are materialized",
  );
  assert.match(
    fetchCommitHistory,
    /graph_data\(\s*log_source,\s*log_order,\s*0\.\.MAX_GIT_PANEL_COMMIT_HISTORY_SHAS,\s*cx,\s*\)/,
    "commit history must request a bounded graph range",
  );
  assertBefore(
    getFetchOptions,
    ".take(MAX_GIT_PANEL_REMOTE_PROMPT_OPTIONS)",
    "picker_prompt::prompt",
    "fetch remote prompt options must be capped before prompting",
  );
  assertBefore(
    getRemote,
    ".take(MAX_GIT_PANEL_REMOTE_PROMPT_OPTIONS)",
    "picker_prompt::prompt",
    "push remote prompt options must be capped before prompting",
  );
  assert.match(
    getRemote,
    /current_remotes\s*\.get\(selection\)\s*\.cloned\(\)/,
    "push remote prompt result must ignore stale selections",
  );
  assert.doesNotMatch(
    getRemote,
    /current_remotes\s*\[\s*selection\s*\]/,
    "push remote prompt result must not direct-index stale selections",
  );
  assertBefore(
    gitInit,
    ".take(MAX_GIT_PANEL_WORKTREE_PROMPT_OPTIONS)",
    ".collect_vec()",
    "git-init worktree choices must be capped before prompt vector materialization",
  );
  assert.match(
    renderStatusEntry,
    /let display_name = bounded_git_panel_label\(&entry\.display_name\(path_style\)\);/,
    "status rows must compact path labels before id and label rendering",
  );
});

test("git panel stale indexes fail closed through checked lookups", () => {
  const source = productionSource(read("crates/git_ui/src/git_panel.rs"));
  const gitInit = functionBody(source, "git_init");
  const selectFirst = functionBody(source, "select_first");
  const selectPrevious = functionBody(source, "select_previous");
  const renderEntries = functionBody(source, "render_entries");
  const uniformListRender = sliceBetween(
    renderEntries,
    "uniform_list(",
    ".when(is_tree_view",
  );

  assert.match(
    gitInit,
    /worktrees\s*\.get\(ix\)\s*\.cloned\(\)/,
    "git init prompt selections must ignore stale worktree indexes",
  );
  assert.doesNotMatch(
    gitInit,
    /worktrees\s*\[\s*ix\s*\]\s*\.clone\(\)/,
    "git init prompt selections must not direct-index stale worktree indexes",
  );

  assert.match(
    selectFirst,
    /index\.and_then\(\|index\|\s*state\s*\.logical_indices\s*\.get\(index\)\s*\.copied\(\)\s*\)/,
    "tree select-first must map visible rows through checked logical indexes",
  );
  assert.doesNotMatch(
    selectFirst,
    /state\s*\.logical_indices\s*\[\s*index\s*\]/,
    "tree select-first must not direct-index logical indexes",
  );

  assert.match(
    selectPrevious,
    /state\s*\.logical_indices\s*\.get\(\s*current_logical_index\.saturating_sub\(1\)\s*\)\s*\.copied\(\)/,
    "tree select-previous must read previous logical indexes through checked lookup",
  );
  assert.doesNotMatch(
    selectPrevious,
    /state\s*\.logical_indices\s*\[\s*current_logical_index\.saturating_sub\(1\)\s*\]/,
    "tree select-previous must not direct-index previous logical indexes",
  );

  assert.match(
    uniformListRender,
    /range\.into_iter\(\)\.filter_map\(\|ix\| match &this\.view_mode/,
    "tree uniform-list rendering must drop stale logical row indexes",
  );
  assert.match(
    uniformListRender,
    /GitPanelViewMode::Tree\(state\) => \{\s*state\s*\.logical_indices\s*\.get\(ix\)\s*\.copied\(\)\s*\}/,
    "tree uniform-list rendering must use checked logical-index lookups",
  );
  assert.doesNotMatch(
    uniformListRender,
    /state\s*\.logical_indices\s*\[\s*ix\s*\]/,
    "tree uniform-list rendering must not direct-index logical indexes",
  );
});
