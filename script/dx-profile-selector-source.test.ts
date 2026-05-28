import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/agent_ui/src/profile_selector.rs";

const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const source = productionSource(readFileSync(sourcePath, "utf8"));

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

test("profile selector declares named source caps", () => {
  assert.match(source, /const MAX_PROFILE_SELECTOR_CANDIDATES: usize = \d+;/);
  assert.match(
    source,
    /const MAX_PROFILE_SELECTOR_MATCH_CANDIDATES: usize = MAX_PROFILE_SELECTOR_CANDIDATES;/,
  );
  assert.match(source, /const MAX_PROFILE_SELECTOR_MATCHES: usize = 100;/);
  assert.match(
    source,
    /const MAX_PROFILE_SELECTOR_RENDER_ENTRIES: usize = MAX_PROFILE_SELECTOR_CANDIDATES \+ 1;/,
  );
});

test("profile candidates are capped before selector vector materialization", () => {
  const candidatesFrom = sliceBetween(
    source,
    "fn candidates_from(profiles: AvailableProfiles) -> Vec<ProfileCandidate> {",
    "fn string_candidates(candidates: &[ProfileCandidate]) -> Vec<StringMatchCandidate> {",
  );

  assertBefore(
    candidatesFrom,
    ".take(MAX_PROFILE_SELECTOR_CANDIDATES)",
    ".map(|(id, name)| ProfileCandidate",
    "profile candidates must be capped before candidate rows are allocated",
  );
  assertBefore(
    candidatesFrom,
    ".take(MAX_PROFILE_SELECTOR_CANDIDATES)",
    ".collect()",
    "profile candidates must be capped before Vec materialization",
  );
});

test("string-match candidates and fuzzy results are bounded before search vectors", () => {
  const stringCandidates = sliceBetween(
    source,
    "fn string_candidates(candidates: &[ProfileCandidate]) -> Vec<StringMatchCandidate> {",
    "fn documentation(candidate: &ProfileCandidate) -> Option<&'static str> {",
  );
  const searchBlocking = sliceBetween(
    source,
    "fn search_blocking(&self, query: &str) -> Vec<StringMatch> {",
    "impl PickerDelegate for ProfilePickerDelegate",
  );
  const updateMatches = sliceBetween(
    source,
    "fn update_matches(",
    "fn confirm(",
  );

  assertBefore(
    stringCandidates,
    ".take(MAX_PROFILE_SELECTOR_MATCH_CANDIDATES)",
    "StringMatchCandidate::new",
    "string-match candidates must be capped before candidate allocation",
  );
  assertBefore(
    stringCandidates,
    ".take(MAX_PROFILE_SELECTOR_MATCH_CANDIDATES)",
    ".collect()",
    "string-match candidates must be capped before Vec materialization",
  );
  assertBefore(
    searchBlocking,
    "MAX_PROFILE_SELECTOR_MATCHES",
    "&cancel_flag",
    "blocking fuzzy search must pass a named result cap to match_strings",
  );
  assertBefore(
    updateMatches,
    "MAX_PROFILE_SELECTOR_MATCHES",
    "cancel_for_future.as_ref()",
    "async fuzzy search must pass a named result cap to match_strings",
  );
});

test("picker entries are capped before filtered entry assignment", () => {
  const refreshProfiles = sliceBetween(
    source,
    "fn refresh_profiles(",
    "fn candidates_from(profiles: AvailableProfiles) -> Vec<ProfileCandidate> {",
  );
  const updateMatches = sliceBetween(
    source,
    "fn update_matches(",
    "fn confirm(",
  );
  const entriesFromCandidates = sliceBetween(
    source,
    "fn entries_from_candidates(candidates: &[ProfileCandidate]) -> Vec<ProfilePickerEntry> {",
    "fn entries_from_matches(&self, matches: Vec<StringMatch>) -> Vec<ProfilePickerEntry> {",
  );
  const entriesFromMatches = sliceBetween(
    source,
    "fn entries_from_matches(&self, matches: Vec<StringMatch>) -> Vec<ProfilePickerEntry> {",
    "fn first_selectable_index(&self) -> Option<usize> {",
  );

  assertBefore(
    refreshProfiles,
    "Self::entries_from_candidates(&self.candidates)",
    "self.filtered_entries =",
    "refresh without a query must build capped picker entries before assignment",
  );
  assertBefore(
    refreshProfiles,
    "self.entries_from_matches(matches)",
    "self.filtered_entries =",
    "refresh with a query must build capped match entries before assignment",
  );
  assertBefore(
    updateMatches,
    "Self::entries_from_candidates(&self.candidates)",
    "self.filtered_entries =",
    "query clearing must build capped picker entries before assignment",
  );
  assertBefore(
    updateMatches,
    "this.delegate.entries_from_matches(matches)",
    "this.delegate.filtered_entries =",
    "async search must build capped match entries before assignment",
  );
  assertBefore(
    entriesFromCandidates,
    "entries.len() >= MAX_PROFILE_SELECTOR_RENDER_ENTRIES",
    "entries.push(entry)",
    "candidate entries must check the render cap before pushing rows",
  );
  assertBefore(
    entriesFromMatches,
    ".take(MAX_PROFILE_SELECTOR_MATCHES)",
    ".collect()",
    "match entries must be capped before Vec materialization",
  );
});

test("entries_from_candidates and entries_from_matches avoid unbounded row growth", () => {
  const entriesFromCandidates = sliceBetween(
    source,
    "fn entries_from_candidates(candidates: &[ProfileCandidate]) -> Vec<ProfilePickerEntry> {",
    "fn entries_from_matches(&self, matches: Vec<StringMatch>) -> Vec<ProfilePickerEntry> {",
  );
  const entriesFromMatches = sliceBetween(
    source,
    "fn entries_from_matches(&self, matches: Vec<StringMatch>) -> Vec<ProfilePickerEntry> {",
    "fn first_selectable_index(&self) -> Option<usize> {",
  );

  assert.match(entriesFromCandidates, /fn push_picker_entry/);
  assert.match(entriesFromCandidates, /return entries;/);
  assert.doesNotMatch(
    entriesFromCandidates.replace(/fn push_picker_entry[\s\S]*?\n    \}/, ""),
    /entries\.push\(/,
    "entries_from_candidates should route row pushes through the capped helper",
  );
  assert.doesNotMatch(
    entriesFromMatches,
    /for\s+\w+\s+in\s+matches\s*\{/,
    "entries_from_matches should not iterate every match without a cap",
  );
});

test("profile selector source guard is focused on production source", () => {
  assert.equal(sourcePath, "crates/agent_ui/src/profile_selector.rs");
  assert.doesNotMatch(sourcePath, /test/i);
  assert.doesNotMatch(
    source,
    /#\[cfg\(test\)\]/,
    "source guard should only inspect production selector code",
  );
});
