import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const readProductionSource = (path: string) => {
  const source = readFileSync(path, "utf8");
  return source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;
};

const sources = {
  outlinePanel: readProductionSource("crates/outline_panel/src/outline_panel.rs"),
  outlineModal: readProductionSource("crates/outline/src/outline.rs"),
  languageOutline: readProductionSource("crates/language/src/outline.rs"),
  themeSelector: readProductionSource("crates/theme_selector/src/theme_selector.rs"),
  iconThemeSelector: readProductionSource(
    "crates/theme_selector/src/icon_theme_selector.rs",
  ),
  fileIcons: readProductionSource("crates/file_icons/src/file_icons.rs"),
};

function sliceBetween(haystack: string, start: string, end: string): string {
  const startIndex = haystack.indexOf(start);
  assert.notEqual(startIndex, -1, `expected ${start}`);
  const endIndex = haystack.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `expected ${end} after ${start}`);
  return haystack.slice(startIndex, endIndex);
}

function indexOfPattern(source: string, pattern: string | RegExp): number {
  if (typeof pattern === "string") {
    return source.indexOf(pattern);
  }

  return source.match(pattern)?.index ?? -1;
}

function assertBefore(
  haystack: string,
  before: string | RegExp,
  after: string | RegExp,
  message: string,
) {
  const beforeIndex = indexOfPattern(haystack, before);
  const afterIndex = indexOfPattern(haystack, after);
  assert.ok(beforeIndex >= 0, `expected ${before}`);
  assert.ok(afterIndex >= 0, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

function functionBody(source: string, name: string): string {
  const start = source.indexOf(`fn ${name}(`);
  assert.ok(start >= 0, `expected ${name}`);

  const bodyStart = source.indexOf("{", start);
  assert.ok(bodyStart > start, `expected body for ${name}`);

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
}

test("language outline caps symbol rows, candidate text, and search expansion", () => {
  const source = sources.languageOutline;
  assert.match(source, /pub const MAX_OUTLINE_ITEMS: usize = 20_000;/);
  assert.match(source, /const MAX_OUTLINE_PATH_TEXT_BYTES: usize = \d+ \* 1024;/);
  assert.match(source, /const MAX_OUTLINE_SYMBOL_TEXT_BYTES: usize = \d+ \* 1024;/);
  assert.match(source, /const MAX_OUTLINE_NAME_RANGES: usize = \d+;/);
  assert.match(source, /pub const MAX_OUTLINE_SEARCH_MATCHES: usize = 100;/);
  assert.match(source, /const MAX_OUTLINE_TREE_MATCHES: usize = [\d_]+;/);

  const newBody = functionBody(source, "new");
  assertBefore(
    newBody,
    "items.truncate(MAX_OUTLINE_ITEMS)",
    "for (id, item) in items.iter().enumerate()",
    "outline rows must be capped before candidate materialization",
  );
  assertBefore(
    newBody,
    "push_bounded_outline_text(&mut path_text, &item.text, MAX_OUTLINE_PATH_TEXT_BYTES)",
    "path_candidates.push(StringMatchCandidate::new(id, &path_text))",
    "path candidates must use bounded symbol path text",
  );
  assertBefore(
    newBody,
    ".take(MAX_OUTLINE_NAME_RANGES)",
    /push_bounded_outline_text\(\s*&mut candidate_text/,
    "symbol name ranges must be capped before candidate text materialization",
  );

  const searchBody = functionBody(source, "search");
  assertBefore(
    searchBody,
    "MAX_OUTLINE_SEARCH_MATCHES",
    "&Default::default()",
    "outline fuzzy search must pass a named result cap",
  );
  assertBefore(
    searchBody,
    "tree_matches.len() >= MAX_OUTLINE_TREE_MATCHES",
    "tree_matches.insert(",
    "ancestor insertion must check the tree-match cap before inserting rows",
  );
  assertBefore(
    searchBody,
    "tree_matches.len() >= MAX_OUTLINE_TREE_MATCHES",
    "tree_matches.push(string_match)",
    "search result rows must check the tree-match cap before pushing rows",
  );
});

test("outline modal caps fetched items and empty-query render rows", () => {
  const source = sources.outlineModal;
  assert.match(source, /const MAX_OUTLINE_VIEW_ITEMS: usize = language::MAX_OUTLINE_ITEMS;/);

  const outlineForEditor = functionBody(source, "outline_for_editor");
  assertBefore(
    outlineForEditor,
    ".take(MAX_OUTLINE_VIEW_ITEMS)",
    ".collect()",
    "outline modal must cap fetched symbol rows before collecting",
  );

  const updateMatches = functionBody(source, "update_matches");
  const emptyQuery = sliceBetween(
    updateMatches,
    "let matches = if is_query_empty {",
    "} else {",
  );
  assertBefore(
    emptyQuery,
    ".take(MAX_OUTLINE_VIEW_ITEMS)",
    ".collect()",
    "empty-query modal rows must be capped before collection",
  );
});

test("outline modal guards stale selection indexes and candidate ids", () => {
  const source = sources.outlineModal;
  const clampedMatchIndex = functionBody(source, "clamped_match_index");
  assert.match(clampedMatchIndex, /self\.matches\.len\(\)\.saturating_sub\(1\)/);

  const clampSelection = functionBody(source, "clamp_selected_index_to_matches");
  assert.match(
    clampSelection,
    /self\.selected_match_index = self\.clamped_match_index\(self\.selected_match_index\);/,
  );

  const setSelectedIndex = functionBody(source, "set_selected_index");
  assert.match(
    setSelectedIndex,
    /self\.selected_match_index = self\.clamped_match_index\(ix\);/,
  );
  assert.match(
    setSelectedIndex,
    /self\.matches\.get\(self\.selected_match_index\)/,
  );
  assert.match(
    setSelectedIndex,
    /self\.outline\.items\.get\(selected_match\.candidate_id\)/,
  );
  assert.doesNotMatch(
    setSelectedIndex,
    /outline\.items\s*\[\s*selected_match\.candidate_id\s*\]/,
  );

  const updateMatches = functionBody(source, "update_matches");
  assertBefore(
    updateMatches,
    "this.delegate.matches = matches;",
    "this.delegate.clamp_selected_index_to_matches();",
    "outline modal must clamp the selected index after replacing matches",
  );
  assertBefore(
    updateMatches,
    "this.delegate.clamp_selected_index_to_matches();",
    "let selected_index = if is_query_empty {",
    "outline modal must clamp stale selection before choosing the new selected match",
  );

  const emptyQuerySelection = sliceBetween(
    updateMatches,
    "let selected_index = if is_query_empty {",
    "} else {",
  );
  assert.match(
    emptyQuerySelection,
    /filter_map\(\|\(ix, m\)\|\s*\{\s*let item = this\.delegate\.outline\.items\.get\(m\.candidate_id\)\?;/,
  );
  assert.doesNotMatch(
    emptyQuerySelection,
    /outline\.items\s*\[\s*m\.candidate_id\s*\]/,
  );
});

test("outline panel bounds cached rows, match candidates, and filtering", () => {
  const source = sources.outlinePanel;
  assert.match(source, /const MAX_OUTLINE_PANEL_CACHED_ENTRIES: usize = 50_000;/);
  assert.match(
    source,
    /const MAX_OUTLINE_PANEL_MATCH_CANDIDATES: usize = MAX_OUTLINE_PANEL_CACHED_ENTRIES;/,
  );
  assert.match(source, /const MAX_OUTLINE_PANEL_FILTER_MATCHES: usize = 10_000;/);
  assert.match(source, /const MAX_OUTLINE_PANEL_SEARCH_MATCHES: usize = 20_000;/);
  assert.match(
    source,
    /const MAX_OUTLINE_PANEL_SEARCH_MATCHES_PER_BUFFER: usize = 10_000;/,
  );
  assert.match(
    source,
    /const MAX_OUTLINE_PANEL_OUTLINES_PER_EXCERPT: usize = language::MAX_OUTLINE_ITEMS;/,
  );
  assert.match(
    source,
    /const MAX_OUTLINE_PANEL_OUTLINE_LOCATION_ITEMS: usize = language::MAX_OUTLINE_ITEMS;/,
  );
  assert.match(source, /const MAX_OUTLINE_PANEL_EXCERPT_RANGES: usize = 4_096;/);

  const searchStateNew = sliceBetween(
    source,
    "impl SearchState {",
    "struct SearchData",
  );
  assertBefore(
    searchStateNew,
    ".take(MAX_OUTLINE_PANEL_SEARCH_MATCHES)",
    ".collect()",
    "outline panel search-state rows must be capped before collection",
  );

  const generateCachedEntries = functionBody(source, "generate_cached_entries");
  assertBefore(
    generateCachedEntries,
    "MAX_OUTLINE_PANEL_FILTER_MATCHES",
    "&AtomicBool::default()",
    "outline panel filter matching must pass a named result cap",
  );

  const pushEntry = functionBody(source, "push_entry");
  assertBefore(
    pushEntry,
    "state.entries.len() >= MAX_OUTLINE_PANEL_CACHED_ENTRIES",
    "state.entries.push(CachedEntry",
    "cached entry pushes must check the row cap first",
  );
  assertBefore(
    pushEntry,
    "state.match_candidates.len() < MAX_OUTLINE_PANEL_MATCH_CANDIDATES",
    "StringMatchCandidate::new",
    "match candidates must be guarded before allocation",
  );
});

test("outline panel caps outline flattening, location scans, and search rows", () => {
  const source = sources.outlinePanel;
  const addBufferEntries = functionBody(source, "add_buffer_entries");
  assert.match(addBufferEntries, /let all_outlines = capped_outline_refs\(buffer\.iter_outlines\(\)\);/);
  assertBefore(
    addBufferEntries,
    "visible_outlines.len() >= MAX_OUTLINE_PANEL_OUTLINES_PER_EXCERPT",
    "visible_outlines.push(outline)",
    "visible outline rows must check the cap before pushing rows",
  );

  const outlineLocation = functionBody(source, "outline_location");
  assertBefore(
    outlineLocation,
    ".take(MAX_OUTLINE_PANEL_OUTLINE_LOCATION_ITEMS + 1)",
    ".collect::<Vec<_>>()",
    "outline location rows must be capped before collection",
  );
  assertBefore(
    outlineLocation,
    "cap_outline_location_items(&mut excerpt_outlines)",
    "let mut matching_outline_indices = Vec::new()",
    "outline location rows must be truncated before child/index maps are built",
  );

  const addSearchEntries = functionBody(source, "add_search_entries");
  assertBefore(
    addSearchEntries,
    ".take(MAX_OUTLINE_PANEL_EXCERPT_RANGES)",
    ".collect::<Vec<_>>()",
    "excerpt ranges must be capped before Vec materialization",
  );
  assertBefore(
    addSearchEntries,
    "pushed_matches >= MAX_OUTLINE_PANEL_SEARCH_MATCHES_PER_BUFFER",
    "self.push_entry(",
    "per-buffer search rows must be capped after excerpt filtering and before panel row pushes",
  );
});

test("theme selector caps theme candidates and fuzzy results", () => {
  const source = sources.themeSelector;
  assert.match(source, /const MAX_THEME_SELECTOR_THEMES: usize = 4_096;/);
  assert.match(source, /const MAX_THEME_SELECTOR_MATCHES: usize = 100;/);

  const delegateNew = sliceBetween(
    source,
    "impl ThemeSelectorDelegate {",
    "fn show_selected_theme",
  );
  assertBefore(
    delegateNew,
    ".take(MAX_THEME_SELECTOR_THEMES + 1)",
    ".collect::<Vec<_>>()",
    "theme list must be bounded before materialization",
  );
  assertBefore(
    delegateNew,
    "cap_theme_selector_themes(&mut themes)",
    "themes.sort_unstable_by",
    "theme list must be truncated before sort/render setup",
  );

  const updateMatches = functionBody(source, "update_matches");
  assertBefore(
    updateMatches,
    ".take(MAX_THEME_SELECTOR_THEMES)",
    "StringMatchCandidate::new",
    "theme match candidates must be capped before allocation",
  );
  assertBefore(
    updateMatches,
    "MAX_THEME_SELECTOR_MATCHES",
    "&Default::default()",
    "theme fuzzy search must pass a named result cap",
  );
});

test("icon theme selector caps theme candidates and fuzzy results", () => {
  const source = sources.iconThemeSelector;
  assert.match(source, /const MAX_ICON_THEME_SELECTOR_THEMES: usize = 4_096;/);
  assert.match(source, /const MAX_ICON_THEME_SELECTOR_MATCHES: usize = 100;/);

  const delegateNew = sliceBetween(
    source,
    "impl IconThemeSelectorDelegate {",
    "fn show_selected_theme",
  );
  assertBefore(
    delegateNew,
    ".take(MAX_ICON_THEME_SELECTOR_THEMES + 1)",
    ".collect::<Vec<_>>()",
    "icon-theme list must be bounded before materialization",
  );
  assertBefore(
    delegateNew,
    "cap_icon_theme_selector_themes(&mut themes)",
    "themes.sort_unstable_by",
    "icon-theme list must be truncated before sort/render setup",
  );

  const updateMatches = functionBody(source, "update_matches");
  assertBefore(
    updateMatches,
    ".take(MAX_ICON_THEME_SELECTOR_THEMES)",
    "StringMatchCandidate::new",
    "icon-theme match candidates must be capped before allocation",
  );
  assertBefore(
    updateMatches,
    "MAX_ICON_THEME_SELECTOR_MATCHES",
    "&Default::default()",
    "icon-theme fuzzy search must pass a named result cap",
  );
});

test("file icons bound suffix bytes and dot-segment expansion", () => {
  const source = sources.fileIcons;
  assert.match(source, /const MAX_FILE_ICON_SUFFIX_BYTES: usize = 1_024;/);
  assert.match(source, /const MAX_FILE_ICON_SUFFIX_SEGMENTS: usize = 32;/);
  assert.match(source, /fn bounded_icon_suffix\(suffix: &str\) -> Option<&str>/);
  assert.match(source, /fn bounded_multiple_extensions\(path: &Path\) -> Option<String>/);

  const getIcon = functionBody(source, "get_icon");
  assert.match(getIcon, /for _ in 0\.\.MAX_FILE_ICON_SUFFIX_SEGMENTS/);
  assert.doesNotMatch(getIcon, /while let Some\(\(_, suffix\)\) = typ\.split_once\('\.'\)/);
  assert.match(getIcon, /bounded_multiple_extensions\(path\)/);
  assert.match(
    getIcon,
    /path\s*\.extension_or_hidden_file_name\(\)\s*\.and_then\(bounded_icon_suffix\)/,
  );
  assert.match(
    getIcon,
    /path\s*\.extension\(\)\s*\.and_then\(\|ext\| ext\.to_str\(\)\)\s*\.and_then\(bounded_icon_suffix\)/,
  );
});
