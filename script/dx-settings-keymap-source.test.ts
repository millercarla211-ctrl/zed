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
    /\.filter\(move \|\&\(item_index, _\)\| self\.filter_table\[page_idx\]\[item_index\]\)\s+\.take\(MAX_SETTINGS_VISIBLE_PAGE_ITEMS\)/,
    "visible page row iteration must be capped at the iterator boundary",
  );
});

test("settings search result paths are capped before applying filters", () => {
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
    ".map(|m|",
    "action completion rows must be capped before Completion materialization",
  );
});
