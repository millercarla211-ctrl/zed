import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const settingsSource = productionSource(
  read("crates/settings_ui/src/settings_ui.rs"),
);
const componentsSource = read("crates/settings_ui/src/components.rs");
const dropdownSource = read("crates/settings_ui/src/components/dropdown.rs");
const fontPickerSource = read("crates/settings_ui/src/components/font_picker.rs");
const themePickerSource = read("crates/settings_ui/src/components/theme_picker.rs");
const iconThemePickerSource = read(
  "crates/settings_ui/src/components/icon_theme_picker.rs",
);
const ollamaPickerSource = read(
  "crates/settings_ui/src/components/ollama_model_picker.rs",
);
const keymapSource = productionSource(
  read("crates/keymap_editor/src/keymap_editor.rs"),
);
const actionCompletionSource = read(
  "crates/keymap_editor/src/action_completion_provider.rs",
);

function sliceBetween(haystack: string, start: string, end: string): string {
  const startIndex = haystack.indexOf(start);
  assert.notEqual(startIndex, -1, `expected ${start}`);
  const endIndex = haystack.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `expected ${end} after ${start}`);
  return haystack.slice(startIndex, endIndex);
}

function assertBefore(
  haystack: string,
  before: string,
  after: string,
  message: string,
) {
  const beforeIndex = haystack.indexOf(before);
  const afterIndex = haystack.indexOf(after);
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

function matchCount(haystack: string, pattern: RegExp): number {
  return Array.from(haystack.matchAll(pattern)).length;
}

test("settings UI declares named search and render fanout caps", () => {
  assert.match(settingsSource, /const MAX_SETTINGS_LANGUAGE_SUBPAGES: usize = \d+;/);
  assert.match(settingsSource, /const MAX_SETTINGS_NAVBAR_ENTRIES: usize = \d+;/);
  assert.match(settingsSource, /const MAX_SETTINGS_VISIBLE_PAGE_ITEMS: usize = \d+;/);
  assert.match(settingsSource, /const MAX_SETTINGS_JSON_PATH_MATCHES: usize = \d+;/);
  assert.match(settingsSource, /const MAX_SETTINGS_EXACT_MATCHES: usize = \d+;/);
  assert.match(settingsSource, /const MAX_SETTINGS_FUZZY_MATCHES: usize = \d+;/);
  assert.match(settingsSource, /const MAX_SETTINGS_MATCH_INDICES: usize = \d+;/);
});

test("settings language, navbar, and visible row materialization are bounded", () => {
  const allLanguageNames = sliceBetween(
    settingsSource,
    "fn all_language_names(cx: &App) -> Vec<SharedString> {",
    "#[allow(unused)]",
  );
  const buildNavbar = sliceBetween(
    settingsSource,
    "fn build_navbar(&mut self, cx: &App) {",
    "fn setup_navbar_focus_subscriptions",
  );
  const visiblePageItems = sliceBetween(
    settingsSource,
    "fn visible_page_items(&self) -> impl Iterator<Item = (usize, &SettingsPageItem)> {",
    "fn render_sub_page_breadcrumbs",
  );

  assertBefore(
    allLanguageNames,
    ".take(MAX_SETTINGS_LANGUAGE_SUBPAGES)",
    ".map(Into::into)",
    "language settings subpages must be capped before SharedString materialization",
  );
  assert.match(
    buildNavbar,
    /if navbar_entries\.len\(\) >= MAX_SETTINGS_NAVBAR_ENTRIES/,
    "navbar construction must stop before creating unbounded focus handles",
  );
  assert.match(
    visiblePageItems,
    /\.filter\(move \|\&\(item_index, _\)\| self\.visible_page_item_matches\(page_idx, item_index\)\)\s+\.take\(MAX_SETTINGS_VISIBLE_PAGE_ITEMS\)/,
    "visible page row iteration must be capped at the iterator boundary",
  );
});

test("settings search result paths are capped before applying filters", () => {
  const openPath = sliceBetween(
    settingsSource,
    "fn open_path(",
    "    let existing_window = cx",
  );
  const filterByJsonPath = sliceBetween(
    settingsSource,
    "fn filter_by_json_path(&self, query: &str) -> Vec<usize> {",
    "fn apply_match_indices",
  );
  const applyMatchIndices = sliceBetween(
    settingsSource,
    "fn apply_match_indices",
    "fn update_matches",
  );
  const updateMatches = sliceBetween(
    settingsSource,
    "fn update_matches(&mut self, cx: &mut Context<SettingsWindow>) {",
    "fn build_filter_table",
  );

  assertBefore(
    filterByJsonPath,
    ".take(MAX_SETTINGS_JSON_PATH_MATCHES)",
    ".collect()",
    "json path search matches must be capped before collecting indices",
  );
  assert.match(
    applyMatchIndices,
    /for match_index in match_indices\.take\(MAX_SETTINGS_MATCH_INDICES\)/,
    "applied setting match indices must be capped at the filter-table write boundary",
  );
  assert.match(
    applyMatchIndices,
    /if let Some\(page\) = self\.filter_table\.get_mut\(page_index\)/,
    "applied setting match indices must guard stale filter-table page indexes",
  );
  assert.match(
    applyMatchIndices,
    /page\.get_mut\(header_index\)/,
    "applied setting match indices must guard stale header indexes",
  );
  assert.match(
    applyMatchIndices,
    /page\.get_mut\(item_index\)/,
    "applied setting match indices must guard stale item indexes",
  );
  assert.match(
    openPath,
    /search_index\.key_lut\.get\(\*index\)/,
    "settings deep-link navigation must guard stale search-index entries",
  );
  assert.match(
    openPath,
    /settings_window\.visible_page_item_matches\(page_index, item_index\)/,
    "settings deep-link navigation must guard stale filter-table entries",
  );
  assert.match(
    openPath,
    /settings_window\.pages\.get\(page_index\)/,
    "settings deep-link navigation must guard stale page indexes",
  );
  assert.match(
    openPath,
    /page\.items\.get\(item_index\)/,
    "settings deep-link navigation must guard stale item indexes",
  );
  assert.match(
    openPath,
    /page\.items\.get\(header_index\)/,
    "settings deep-link navigation must guard stale header indexes",
  );
  assert.doesNotMatch(
    openPath,
    /(?:key_lut|filter_table|pages|items)\[[^\]]+\]/,
    "settings deep-link navigation must not directly index search, filter, page, or item tables",
  );
  assertBefore(
    updateMatches,
    ".take(MAX_SETTINGS_EXACT_MATCHES)",
    ".map(|doc| doc.id)",
    "exact setting matches must be capped before result-vector collection",
  );
  assertBefore(
    updateMatches,
    "MAX_SETTINGS_FUZZY_MATCHES",
    "cx.background_executor().clone()",
    "settings fuzzy search must receive a named result cap",
  );
});

test("settings navigation render paths guard stale page and item indexes", () => {
  const visiblePageItemMatches = sliceBetween(
    settingsSource,
    "fn visible_page_item_matches(&self, page_idx: usize, item_index: usize) -> bool {",
    "fn visible_page_items",
  );
  const navEntrySearchFilter = sliceBetween(
    settingsSource,
    "fn nav_entry_in_search_filter(&self, entry: &NavBarEntry) -> bool {",
    "fn visible_navbar_entries",
  );
  const toggleNavbarEntry = sliceBetween(
    settingsSource,
    "fn toggle_navbar_entry(&mut self, nav_entry_index: usize) {",
    "fn toggle_and_focus_navbar_entry",
  );
  const toggleAndFocusNavbarEntry = sliceBetween(
    settingsSource,
    "fn toggle_and_focus_navbar_entry(",
    "fn toggle_navbar_entry_on_double_click",
  );
  const openNavbarEntryPage = sliceBetween(
    settingsSource,
    "fn open_navbar_entry_page(&mut self, navbar_entry: usize) {",
    "fn open_best_matching_nav_page",
  );
  const openAndScrollNavbar = sliceBetween(
    settingsSource,
    "fn open_and_scroll_to_navbar_entry(",
    "fn scroll_to_content_item",
  );
  const focusAndScrollNavbar = sliceBetween(
    settingsSource,
    "fn focus_and_scroll_to_nav_entry(",
    "fn current_sub_page_scroll_handle",
  );
  const changeFile = sliceBetween(
    settingsSource,
    "fn change_file(&mut self, ix: usize, window: &mut Window, cx: &mut Context<SettingsWindow>) {",
    "fn render_files_header",
  );
  const renderFilesHeader = sliceBetween(
    settingsSource,
    "fn render_files_header(",
    "fn render_search",
  );
  const visibleNavbarEntries = sliceBetween(
    settingsSource,
    "fn visible_navbar_entries(&self) -> impl Iterator<Item = (usize, &NavBarEntry)> {",
    "fn filter_matches_to_file",
  );
  const contentFocusHandle = sliceBetween(
    settingsSource,
    "fn focus_handle_for_content_element(",
    "fn focused_nav_entry",
  );
  const rootEntryContaining = sliceBetween(
    settingsSource,
    "fn root_entry_containing(",
    "\n}",
  );
  const currentPageItems = sliceBetween(
    settingsSource,
    "fn render_current_page_items(",
    "fn render_sub_page_items",
  );

  assert.match(
    visiblePageItemMatches,
    /self\.filter_table\s*\.get\(page_idx\)\s*\.and_then\(\|page_matches\| page_matches\.get\(item_index\)\)/,
    "visible page filters must guard stale page and item indexes",
  );
  assert.match(
    visiblePageItemMatches,
    /\.copied\(\)\s*\.unwrap_or\(false\)/,
    "visible page filters must fail closed when the filter table is stale",
  );
  assert.match(
    navEntrySearchFilter,
    /self\.filter_table\s*\.get\(entry\.page_index\)/,
    "navbar search visibility must guard stale page indexes before reading the filter table",
  );
  assert.match(
    navEntrySearchFilter,
    /page_matches\.get\(item_index\)\.copied\(\)\.unwrap_or\(false\)/,
    "navbar search visibility must fail closed for stale item indexes",
  );
  assert.match(
    navEntrySearchFilter,
    /None => false/,
    "navbar search visibility must fail closed for missing filter pages",
  );
  assert.match(
    visibleNavbarEntries,
    /self\.nav_entry_in_search_filter\(entry\)/,
    "visible navbar iteration must delegate filter-table reads through the guarded helper",
  );
  assert.doesNotMatch(
    visibleNavbarEntries,
    /search_matches\[[^\]]+\]/,
    "visible navbar iteration must not directly index search match pages",
  );
  assert.match(
    toggleNavbarEntry,
    /let Some\(entry\) = self\.navbar_entries\.get_mut\(nav_entry_index\)/,
    "navbar toggles must guard stale target indexes",
  );
  assert.doesNotMatch(
    toggleNavbarEntry,
    /navbar_entries\[[^\]]+\]/,
    "navbar toggles must not directly index navbar entries",
  );
  assert.match(
    toggleAndFocusNavbarEntry,
    /let Some\(focus_handle\) = self\s*\.navbar_entries\s*\.get\(nav_entry_index\)/,
    "navbar toggle focus must guard stale target indexes",
  );
  assert.doesNotMatch(
    toggleAndFocusNavbarEntry,
    /navbar_entries\[[^\]]+\]/,
    "navbar toggle focus must not directly index navbar entries",
  );
  assert.match(
    openNavbarEntryPage,
    /let Some\(target_page_index\) = self\s*\.navbar_entries\s*\.get\(navbar_entry\)/,
    "opening a navbar entry must guard stale target indexes",
  );
  assert.doesNotMatch(
    openNavbarEntryPage,
    /navbar_entries\[[^\]]+\]/,
    "opening a navbar entry must not directly index navbar entries",
  );
  assert.match(
    openAndScrollNavbar,
    /let Some\(entry\) = self\.navbar_entries\.get\(navbar_entry_index\)/,
    "scrolling to a navbar entry must guard stale target indexes",
  );
  assert.doesNotMatch(
    openAndScrollNavbar,
    /navbar_entries\[[^\]]+\]/,
    "scrolling to a navbar entry must not directly index navbar entries",
  );
  assert.match(
    focusAndScrollNavbar,
    /let Some\(focus_handle\) = self\s*\.navbar_entries\s*\.get\(nav_entry_index\)/,
    "focusing a navbar entry must guard stale target indexes",
  );
  assert.doesNotMatch(
    focusAndScrollNavbar,
    /navbar_entries\[[^\]]+\]/,
    "focusing a navbar entry must not directly index navbar entries",
  );
  assert.match(
    changeFile,
    /let Some\(next_file\) = self\.files\.get\(ix\)\.map\(\|\(file, _\)\| file\.clone\(\)\)/,
    "file switching must guard stale file indexes before reading the selected file",
  );
  assert.doesNotMatch(
    changeFile,
    /self\.files\[ix\]/,
    "file switching must not directly index selected files",
  );
  assert.match(
    renderFilesHeader,
    /let Some\(\(file, focus_handle\)\) = self\.files\.get\(selected_file_ix\)/,
    "file header rendering must guard stale selected overflow indexes",
  );
  assert.doesNotMatch(
    renderFilesHeader,
    /self\.files\[[^\]]+\]/,
    "file header rendering must not directly index files",
  );
  assert.match(
    contentFocusHandle,
    /self\.content_handles\s*\.get\(page_index\)\s*\.and_then\(\|page_handles\| page_handles\.get\(actual_item_index\)\)/,
    "content focus lookup must guard stale page and item indexes",
  );
  assert.match(
    rootEntryContaining,
    /-> Option<usize>/,
    "root lookup must fail closed instead of panicking on stale navbar state",
  );
  assert.match(
    rootEntryContaining,
    /self\.navbar_entries\.get\(prev_index\)/,
    "root lookup must guard stale parent indexes",
  );
  assert.doesNotMatch(
    rootEntryContaining,
    /navbar_entries\[[^\]]+\]|expect\("No root entry found"\)/,
    "root lookup must not directly index or panic on stale navbar state",
  );
  assert.doesNotMatch(
    settingsSource,
    /navbar_entries\[(?:focused_entry|focused_entry_parent|prev_index)\]/,
    "collapse and expand nav actions must not direct-index focused navbar entries",
  );
  assert.match(
    currentPageItems,
    /let Some\(item_focus_handle\) = this\.focus_handle_for_content_element\(\s*current_page_index,\s*actual_item_index,\s*cx,\s*\) else \{/,
    "settings page rendering must fail closed when a stale item lacks a focus handle",
  );
  assert.doesNotMatch(
    currentPageItems,
    /content_handles\[[^\]]+\]\[[^\]]+\]/,
    "settings page rendering must not directly index content handles",
  );
});

test("settings focus subscriptions and file filtering guard stale indexes", () => {
  const setupNavbarFocusSubscriptions = sliceBetween(
    settingsSource,
    "fn setup_navbar_focus_subscriptions(",
    "fn nav_entry_in_search_filter",
  );
  const filterMatchesToFile = sliceBetween(
    settingsSource,
    "fn filter_matches_to_file(&mut self) {",
    "fn filter_by_json_path",
  );

  assert.match(
    setupNavbarFocusSubscriptions,
    /for \(entry_index, entry\) in self\.navbar_entries\.iter\(\)\.enumerate\(\)/,
    "navbar focus subscriptions must iterate entries directly while retaining indexes",
  );
  assert.match(
    setupNavbarFocusSubscriptions,
    /let focus_handle = entry\.focus_handle\.clone\(\);/,
    "navbar focus subscriptions must clone focus handles from the iterated entry",
  );
  assert.doesNotMatch(
    setupNavbarFocusSubscriptions,
    /navbar_entries\[[^\]]+\]/,
    "navbar focus subscriptions must not direct-index navbar entries",
  );
  assert.doesNotMatch(
    setupNavbarFocusSubscriptions,
    /0\.\.self\.navbar_entries\.len\(\)/,
    "navbar focus subscriptions must not create indexes separate from entry references",
  );

  assert.match(
    filterMatchesToFile,
    /page_filter\.get_mut\(header_index\)/,
    "file filtering must guard stale section header indexes",
  );
  assert.ok(
    matchCount(filterMatchesToFile, /page_filter\.get_mut\(index\)/g) >= 2,
    "file filtering must guard stale setting and action row indexes",
  );
  assert.doesNotMatch(
    filterMatchesToFile,
    /page_filter\[[^\]]+\]\s*=/,
    "file filtering must not direct-index stale filter-table rows",
  );
});

test("settings dropdown and picker option lists use shared caps", () => {
  assert.match(componentsSource, /pub\(super\) const MAX_SETTINGS_PICKER_OPTIONS: usize = \d+;/);
  assert.match(componentsSource, /pub\(super\) const MAX_SETTINGS_PICKER_MATCHES: usize = \d+;/);
  assert.match(componentsSource, /fn bounded_picker_options\(/);
  assert.match(componentsSource, /fn bounded_picker_matches/);

  assert.match(dropdownSource, /const MAX_ENUM_DROPDOWN_VARIANTS: usize = \d+;/);
  assertBefore(
    dropdownSource,
    ".take(MAX_ENUM_DROPDOWN_VARIANTS)",
    "menu.toggleable_entry",
    "enum dropdown variants must be capped before menu row creation",
  );

  for (const [name, source] of [
    ["font picker", fontPickerSource],
    ["theme picker", themePickerSource],
    ["icon theme picker", iconThemePickerSource],
    ["ollama model picker", ollamaPickerSource],
  ] as const) {
    assert.match(
      source,
      /bounded_picker_options\(/,
      `${name} must cap option storage while preserving the current value`,
    );
    assert.match(
      source,
      /bounded_picker_matches\(/,
      `${name} must cap filtered row materialization`,
    );
  }
});

test("keymap editor declares caps for bindings, actions, matches, and conflicts", () => {
  assert.match(keymapSource, /const MAX_KEYMAP_BINDINGS: usize = \d+;/);
  assert.match(keymapSource, /const MAX_KEYMAP_ACTIONS: usize = \d+;/);
  assert.match(keymapSource, /const MAX_KEYMAP_MATCHES: usize = \d+;/);
  assert.match(keymapSource, /const MAX_KEYMAP_CONFLICT_SCAN: usize = \d+;/);
});

test("keymap binding/action indexing and search results are bounded", () => {
  const humanizedCache = sliceBetween(
    keymapSource,
    "impl HumanizedActionNameCache {",
    "struct KeyBinding {",
  );
  const updateMatches = sliceBetween(
    keymapSource,
    "async fn update_matches(",
    "fn get_conflict",
  );
  const processBindings = sliceBetween(
    keymapSource,
    "fn process_bindings(",
    "fn on_keymap_changed",
  );
  const onKeymapChanged = sliceBetween(
    keymapSource,
    "fn on_keymap_changed(",
    "fn scroll_to_item",
  );
  const modalNew = sliceBetween(
    keymapSource,
    "fn new(",
    "fn add_action_arguments_input",
  );

  assertBefore(
    humanizedCache,
    ".take(MAX_KEYMAP_ACTIONS)",
    "command_palette::humanize_action_name",
    "humanized action-name cache must be capped before label materialization",
  );
  assertBefore(
    updateMatches,
    "MAX_KEYMAP_MATCHES.min(keybind_count)",
    "&Default::default()",
    "keymap fuzzy search must receive a named result cap",
  );
  assertBefore(
    processBindings,
    ".take(MAX_KEYMAP_BINDINGS)",
    ".collect::<Vec<_>>()",
    "raw keybindings must be capped before vector materialization",
  );
  assertBefore(
    processBindings,
    ".take(MAX_KEYMAP_ACTIONS)",
    "HashSet::from_iter",
    "unmapped action discovery must be capped before HashSet materialization",
  );
  assertBefore(
    onKeymapChanged,
    ".take(MAX_KEYMAP_MATCHES)",
    ".collect()",
    "initial keymap rows must be capped before StringMatch collection",
  );
  assertBefore(
    modalNew,
    ".take(MAX_KEYMAP_ACTIONS)",
    "ActionCompletionProvider::new",
    "create-keybinding action completion inputs must be capped before provider construction",
  );
});

test("keymap fuzzy candidate ids are checked before binding materialization", () => {
  const updateMatches = sliceBetween(
    keymapSource,
    "async fn update_matches(",
    "fn get_conflict",
  );
  const previousEditScroll = sliceBetween(
    keymapSource,
    "PreviousEdit::Keybinding {",
    "if let Some(scroll_position) = scroll_position",
  );
  const selectedKeybindAndIndex = sliceBetween(
    keymapSource,
    "fn selected_keybind_and_index(&self) -> Option<(&ProcessedBinding, usize)> {",
    "fn selected_binding",
  );
  const renderRows = sliceBetween(
    keymapSource,
    'Table::new(COLS)',
    ".map_row(cx.processor(",
  );

  assert.match(
    updateMatches,
    /this\.keybindings\s*\.get\(candidate\.candidate_id\)\s*\.is_some_and\(\|binding\|/,
    "source filters must skip stale fuzzy candidate ids before reading bindings",
  );
  assert.match(
    updateMatches,
    /this\.keybindings\s*\.get\(item\.candidate_id\)\s*\.and_then\(\|binding\| binding\.keystrokes\(\)\)/,
    "keystroke search filters must skip stale fuzzy candidate ids before reading keystrokes",
  );
  assert.match(
    updateMatches,
    /this\.keybindings\s*\.get\(item\.candidate_id\)\s*\.is_some_and\(\|binding\| !binding\.is_no_action\(\)\)/,
    "no-action filtering must skip stale fuzzy candidate ids before reading bindings",
  );
  assert.match(
    updateMatches,
    /this\.keybindings\s*\.get\(item1\.candidate_id\)/,
    "empty-query sort must guard the first fuzzy candidate id",
  );
  assert.match(
    updateMatches,
    /this\.keybindings\s*\.get\(item2\.candidate_id\)/,
    "empty-query sort must guard the second fuzzy candidate id",
  );
  assert.doesNotMatch(
    updateMatches,
    /this\.keybindings\[[^\]]*candidate_id[^\]]*\]/,
    "keymap search materialization must not direct-index bindings by fuzzy candidate ids",
  );
  assert.match(
    previousEditScroll,
    /let binding = this\.keybindings\.get\(item\.candidate_id\)\?;/,
    "previous-edit scroll recovery must skip stale fuzzy candidate ids",
  );
  assert.doesNotMatch(
    previousEditScroll,
    /this\.keybindings\[[^\]]*candidate_id[^\]]*\]/,
    "previous-edit scroll recovery must not direct-index bindings by fuzzy candidate ids",
  );
  assert.match(
    selectedKeybindAndIndex,
    /self\.keybindings\s*\.get\(keybind_index\)\s*\.map\(\|binding\| \(binding, keybind_index\)\)/,
    "selected binding lookup must guard stale keybinding indexes",
  );
  assert.doesNotMatch(
    selectedKeybindAndIndex,
    /self\.keybindings\[[^\]]+\]/,
    "selected binding lookup must not direct-index keybindings",
  );
  assert.match(
    renderRows,
    /let binding = this\.keybindings\.get\(candidate_id\)\?;/,
    "rendered keymap rows must skip stale fuzzy candidate ids",
  );
  assert.doesNotMatch(
    renderRows,
    /this\.keybindings\[candidate_id\]/,
    "rendered keymap rows must not direct-index bindings by fuzzy candidate ids",
  );
});

test("keymap conflict rows and action completions use bounded result caps", () => {
  const conflictingIndices = sliceBetween(
    keymapSource,
    "fn conflicting_indices_for_mapping(",
    "fn conflict_for_idx",
  );
  assertBefore(
    conflictingIndices,
    ".take(MAX_KEYMAP_CONFLICT_SCAN)",
    ".count()",
    "conflict warning counts must be bounded before counting remaining rows",
  );

  assert.match(
    actionCompletionSource,
    /const MAX_ACTION_COMPLETION_CANDIDATES: usize = \d+;/,
  );
  assert.match(
    actionCompletionSource,
    /const MAX_ACTION_COMPLETION_MATCHES: usize = 50;/,
  );
  assertBefore(
    actionCompletionSource,
    ".take(MAX_ACTION_COMPLETION_CANDIDATES)",
    ".map(|(ix, &name)|",
    "action completion candidates must be capped before fuzzy label allocation",
  );
  assertBefore(
    actionCompletionSource,
    "MAX_ACTION_COMPLETION_MATCHES.min(candidates.len())",
    "&Default::default()",
    "action completion fuzzy search must receive a named result cap",
  );
  assertBefore(
    actionCompletionSource,
    ".take(MAX_ACTION_COMPLETION_MATCHES)",
    ".filter_map(|m|",
    "action completion rows must be capped before Completion materialization",
  );
});

test("action completion materialization skips stale fuzzy candidate ids", () => {
  assert.match(
    actionCompletionSource,
    /\.filter_map\(\|m\| {\s+let action_name = \*action_names\.get\(m\.candidate_id\)\?;/,
    "action completion rows must guard stale fuzzy candidate ids before reading action names",
  );
  assert.doesNotMatch(
    actionCompletionSource,
    /action_names\[[^\]]*candidate_id[^\]]*\]/,
    "action completion rows must not direct-index action names by fuzzy candidate ids",
  );
});
