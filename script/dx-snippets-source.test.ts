import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8").replace(/\r\n/g, "\n");

const snippetsUiPath = "crates/snippets_ui/src/snippets_ui.rs";
const snippetsUiSource = read(snippetsUiPath);

const sliceBetween = (haystack: string, start: string, end: string) => {
  const startIndex = haystack.indexOf(start);
  assert.notEqual(startIndex, -1, `expected ${start}`);
  const endIndex = haystack.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `expected ${end} after ${start}`);
  return haystack.slice(startIndex, endIndex);
};

const assertBefore = (
  haystack: string,
  before: string,
  after: string,
  message: string,
) => {
  const beforeIndex = haystack.indexOf(before);
  const afterIndex = haystack.indexOf(after);
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("snippet scope selector caps scope candidates before picker materialization", () => {
  assert.match(snippetsUiSource, /const MAX_SNIPPET_SCOPE_CANDIDATES: usize = 2_000;/);
  assert.match(snippetsUiSource, /const MAX_EXISTING_SNIPPET_SCOPE_FILES: usize = 4_000;/);

  const newDelegate = sliceBetween(
    snippetsUiSource,
    "impl ScopeSelectorDelegate {",
    "    fn scope_icon(",
  );

  assertBefore(
    newDelegate,
    ".take(MAX_SNIPPET_SCOPE_CANDIDATES.saturating_sub(1))",
    ".map(|(candidate_id, name)| StringMatchCandidate::new(candidate_id, name.as_ref()))",
    "language scope names must be capped before fuzzy candidate construction",
  );
  assertBefore(
    newDelegate,
    ".take(MAX_EXISTING_SNIPPET_SCOPE_FILES)",
    "existing_scopes\n                            .insert",
    "snippet scope files must be capped before existing-scope storage",
  );
});

test("snippet source guard stays in worker-owned files", () => {
  assert.equal(snippetsUiPath, "crates/snippets_ui/src/snippets_ui.rs");
  assert.doesNotMatch(snippetsUiPath, /test/i);
});
