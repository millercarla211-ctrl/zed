import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const extensionsSourcePath = "crates/extensions_ui/src/extensions_ui.rs";
const versionSelectorSourcePath = "crates/extensions_ui/src/extension_version_selector.rs";
const suggestSourcePath = "crates/extensions_ui/src/extension_suggest.rs";

const extensionsSource = read(extensionsSourcePath);
const versionSelectorSource = read(versionSelectorSourcePath);
const suggestSource = read(suggestSourcePath);

function functionBody(source: string, name: string): string {
  const fnIndex = source.indexOf(`fn ${name}(`);
  assert.notEqual(fnIndex, -1, `expected ${name}`);

  const bodyStart = source.indexOf("{", fnIndex);
  assert.ok(bodyStart > fnIndex, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(fnIndex, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
}

function sliceBetween(source: string, startNeedle: string, endNeedle: string): string {
  const start = source.indexOf(startNeedle);
  assert.notEqual(start, -1, `expected ${startNeedle}`);
  const end = source.indexOf(endNeedle, start + startNeedle.length);
  assert.ok(end > start, `expected ${endNeedle} after ${startNeedle}`);
  return source.slice(start, end);
}

function assertBefore(
  haystack: string,
  before: string | RegExp,
  after: string | RegExp,
  message: string,
) {
  const beforeIndex =
    typeof before === "string" ? haystack.indexOf(before) : haystack.match(before)?.index ?? -1;
  const afterIndex =
    typeof after === "string" ? haystack.indexOf(after) : haystack.match(after)?.index ?? -1;
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("extensions page caps search text and list materialization", () => {
  assert.match(extensionsSource, /const MAX_EXTENSION_SEARCH_QUERY_CHARS: usize = 256;/);
  assert.match(extensionsSource, /const MAX_EXTENSION_LIST_ENTRIES: usize = 2_000;/);
  assert.match(extensionsSource, /const MAX_EXTENSION_FUZZY_CANDIDATES: usize = 1_000;/);
  assert.match(extensionsSource, /const MAX_FILTERED_EXTENSION_RESULTS: usize = 1_000;/);
  assert.match(
    extensionsSource,
    /const MAX_DISPLAYED_EXTENSION_RESULTS: usize = MAX_FILTERED_EXTENSION_RESULTS;/,
  );

  const searchQuery = functionBody(extensionsSource, "search_query");
  assert.match(searchQuery, /text_for_range\(MultiBufferOffset\(0\)\.\.snapshot\.len\(\)\)/);
  assertBefore(
    searchQuery,
    ".take(MAX_EXTENSION_SEARCH_QUERY_CHARS)",
    ".collect::<String>()",
    "extension search query must be capped before string materialization",
  );
  assert.doesNotMatch(
    searchQuery,
    /\.text\(cx\)/,
    "extension search query must not clone the full editor text before bounding",
  );

  const fetchExtensions = functionBody(extensionsSource, "fetch_extensions");
  assertBefore(
    fetchExtensions,
    ".take(MAX_EXTENSION_LIST_ENTRIES)",
    ".collect::<Vec<_>>()",
    "dev extension entries must be capped before collection",
  );
  assertBefore(
    fetchExtensions,
    ".take(MAX_EXTENSION_FUZZY_CANDIDATES)",
    "StringMatchCandidate::new",
    "dev extension fuzzy candidates must be capped before candidate creation",
  );
  assert.match(fetchExtensions, /MAX_FILTERED_EXTENSION_RESULTS/);
  assert.match(fetchExtensions, /installed_extension_ids/);
  assert.match(fetchExtensions, /bounded_remote_extension_entries\(extensions, &installed_extension_ids\)/);

  const filterEntries = functionBody(extensionsSource, "filter_extension_entries");
  assert.match(filterEntries, /let dev_limit = if self\.filter\.include_dev_extensions\(\)/);
  assert.match(filterEntries, /\.take\(dev_limit\)/);
  assertBefore(
    filterEntries,
    ".take(dev_limit)",
    /self\s*\.filtered_dev_extension_indices\s*\.extend/,
    "dev extension filtered indices must be capped before storage",
  );
  assert.match(filterEntries, /MAX_FILTERED_EXTENSION_RESULTS\.saturating_sub/);
  assertBefore(
    filterEntries,
    ".take(remaining)",
    /self\s*\.filtered_remote_extension_indices\s*\.extend/,
    "remote extension filtered indices must be capped before storage",
  );

  const renderBody = sliceBetween(
    extensionsSource,
    "impl Render for ExtensionsPage {",
    "impl EventEmitter<ItemEvent> for ExtensionsPage {}",
  );
  assertBefore(
    renderBody,
    ".min(MAX_DISPLAYED_EXTENSION_RESULTS)",
    'uniform_list("entries", count',
    "extension render count must be capped before uniform list materialization",
  );
});

test("extension card text and chips use bounded display helpers", () => {
  assert.match(extensionsSource, /const MAX_EXTENSION_NAME_CHARS: usize = 128;/);
  assert.match(extensionsSource, /const MAX_EXTENSION_DESCRIPTION_CHARS: usize = 512;/);
  assert.match(extensionsSource, /const MAX_EXTENSION_AUTHORS_TO_RENDER: usize = 16;/);
  assert.match(extensionsSource, /const MAX_EXTENSION_AUTHOR_LABEL_CHARS: usize = 512;/);
  assert.match(extensionsSource, /const MAX_EXTENSION_VERSION_LABEL_CHARS: usize = 64;/);
  assert.match(extensionsSource, /const MAX_EXTENSION_PROVIDES_CHIPS: usize = 8;/);
  assert.match(
    functionBody(extensionsSource, "bounded_extension_ui_text"),
    /char_indices\(\)\.nth\(max_chars\)/,
  );
  assert.match(
    functionBody(extensionsSource, "bounded_extension_authors_text"),
    /\.iter\(\)\s*\.take\(MAX_EXTENSION_AUTHORS_TO_RENDER\)/,
  );

  const devCard = functionBody(extensionsSource, "render_dev_extension");
  assert.match(
    devCard,
    /bounded_extension_ui_text\(\s*&extension\.name,\s*MAX_EXTENSION_NAME_CHARS,\s*\)/,
  );
  assert.match(devCard, /bounded_extension_version_label\(&extension\.version\)/);
  assert.match(devCard, /bounded_extension_authors_text\(&extension\.authors\)/);
  assert.match(
    devCard,
    /bounded_extension_ui_text\(\s*description,\s*MAX_EXTENSION_DESCRIPTION_CHARS,\s*\)/,
  );

  const remoteCard = functionBody(extensionsSource, "render_remote_extension");
  assert.match(
    remoteCard,
    /bounded_extension_ui_text\(\s*&extension\.manifest\.name,\s*MAX_EXTENSION_NAME_CHARS,\s*\)/,
  );
  assert.match(remoteCard, /bounded_extension_version_label\(&extension\.manifest\.version\)/);
  assert.match(remoteCard, /\.take\(MAX_EXTENSION_PROVIDES_CHIPS\)/);
  assert.match(
    remoteCard,
    /bounded_extension_ui_text\(\s*description,\s*MAX_EXTENSION_DESCRIPTION_CHARS,\s*\)/,
  );
  assert.match(
    remoteCard,
    /bounded_extension_authors_text\(\s*&extension\.manifest\.authors,\s*\)/,
  );

  const menu = functionBody(extensionsSource, "render_remote_extension_context_menu");
  assert.match(menu, /bounded_extension_authors_text\(&authors\)/);
});

test("extension version selector bounds rows, labels, query, and fuzzy matches", () => {
  assert.match(
    versionSelectorSource,
    /const MAX_EXTENSION_VERSION_SELECTOR_ROWS: usize = 256;/,
  );
  assert.match(
    versionSelectorSource,
    /const MAX_EXTENSION_VERSION_SELECTOR_QUERY_CHARS: usize = 128;/,
  );
  assert.match(
    versionSelectorSource,
    /const MAX_EXTENSION_VERSION_SELECTOR_FUZZY_MATCHES: usize = 100;/,
  );
  assert.match(
    versionSelectorSource,
    /const MAX_EXTENSION_VERSION_SELECTOR_LABEL_CHARS: usize = 64;/,
  );

  const newDelegate = sliceBetween(
    versionSelectorSource,
    "impl ExtensionVersionSelectorDelegate {",
    "impl PickerDelegate for ExtensionVersionSelectorDelegate {",
  );
  assertBefore(
    newDelegate,
    "extension_versions.sort_unstable_by",
    "cap_extension_version_selector_rows(&mut extension_versions)",
    "version selector rows must be capped after newest-version sorting",
  );
  assert.match(newDelegate, /bounded_extension_version_selector_label\(&extension\.manifest\.version\)/);

  const updateMatches = functionBody(versionSelectorSource, "update_matches");
  assertBefore(
    updateMatches,
    "bounded_extension_version_selector_query(query)",
    "StringMatchCandidate::new",
    "version selector query must be capped before fuzzy candidate matching",
  );
  assertBefore(
    updateMatches,
    ".take(MAX_EXTENSION_VERSION_SELECTOR_ROWS)",
    "StringMatchCandidate::new",
    "version selector candidates must be capped before candidate creation",
  );
  assert.match(updateMatches, /MAX_EXTENSION_VERSION_SELECTOR_FUZZY_MATCHES/);

  assert.match(
    versionSelectorSource,
    /fn clamp_extension_version_selector_index\(selected_index: usize, match_count: usize\) -> usize/,
  );
  const clampSelectedIndex = functionBody(
    versionSelectorSource,
    "clamp_extension_version_selector_index",
  );
  assert.match(
    clampSelectedIndex,
    /selected_index\.min\(match_count\.saturating_sub\(1\)\)/,
    "version selector selected index clamp must tolerate empty match lists",
  );

  const setSelectedIndex = functionBody(versionSelectorSource, "set_selected_index");
  assert.match(
    setSelectedIndex,
    /self\.selected_index\s*=\s*clamp_extension_version_selector_index\(ix,\s*self\.matches\.len\(\)\)/,
    "set_selected_index must clamp to the current match count",
  );
  assert.doesNotMatch(
    setSelectedIndex,
    /self\.selected_index\s*=\s*ix\s*;/,
    "set_selected_index must not store a stale index directly",
  );

  assert.match(
    updateMatches,
    /this\.delegate\.selected_index\s*=\s*clamp_extension_version_selector_index\(\s*this\.delegate\.selected_index,\s*this\.delegate\.matches\.len\(\),?\s*\)/,
    "async match replacement must clamp stale selected indexes",
  );

  const confirm = functionBody(versionSelectorSource, "confirm");
  assert.match(
    confirm,
    /self\.matches\.get\(self\.selected_index\)/,
    "confirm must guard stale selected indexes",
  );
  assert.match(
    confirm,
    /self\.extension_versions\.get\(candidate_id\)/,
    "confirm must guard stale fuzzy candidate ids",
  );
  assert.doesNotMatch(
    confirm,
    /self\.matches\[[^\]]+\]/,
    "confirm must not directly index matches",
  );
  assert.doesNotMatch(
    confirm,
    /self\.extension_versions\[[^\]]+\]/,
    "confirm must not directly index extension versions",
  );
});

test("extension suggestions bound notification display labels", () => {
  assert.match(suggestSource, /const MAX_EXTENSION_SUGGESTION_LABEL_CHARS: usize = 128;/);
  assert.match(
    functionBody(suggestSource, "bounded_extension_suggestion_label"),
    /char_indices\(\)\s*\.nth\(MAX_EXTENSION_SUGGESTION_LABEL_CHARS\)/,
  );

  const suggest = functionBody(suggestSource, "suggest");
  assertBefore(
    suggest,
    /let extension_id_label\s*=\s*bounded_extension_suggestion_label\(&extension_id\);/,
    "MessageNotification::new",
    "suggestion extension label must be bounded before notification text materialization",
  );
  assertBefore(
    suggest,
    /let file_name_or_extension_label\s*=\s*bounded_extension_suggestion_label\(&file_name_or_extension\);/,
    "MessageNotification::new",
    "suggestion file label must be bounded before notification text materialization",
  );
});

test("extensions UI source guard stays focused on worker-owned production source", () => {
  assert.equal(extensionsSourcePath, "crates/extensions_ui/src/extensions_ui.rs");
  assert.equal(versionSelectorSourcePath, "crates/extensions_ui/src/extension_version_selector.rs");
  assert.equal(suggestSourcePath, "crates/extensions_ui/src/extension_suggest.rs");
  assert.doesNotMatch(extensionsSourcePath, /test/i);
  assert.doesNotMatch(versionSelectorSourcePath, /test/i);
  assert.doesNotMatch(suggestSourcePath, /test/i);
});
