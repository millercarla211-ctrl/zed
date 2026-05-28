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

const functionBody = (name: string) => {
  const start = snippetsUiSource.search(new RegExp(`fn\\s+${name}\\b`));
  assert.ok(start >= 0, `expected ${name}`);

  const bodyStart = snippetsUiSource.indexOf("{", start);
  assert.ok(bodyStart > start, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < snippetsUiSource.length; index += 1) {
    const char = snippetsUiSource[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return snippetsUiSource.slice(start, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
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

test("snippet scope selector guards stale fuzzy candidate ids before confirm and render lookup", () => {
  const confirm = functionBody("confirm");
  const renderMatch = functionBody("render_match");

  assert.match(
    confirm,
    /self\.candidates\s*\.get\(mat\.candidate_id\)/,
    "confirm must check stale fuzzy candidate ids before opening a scope file",
  );
  assertBefore(
    confirm,
    "self.candidates.get(mat.candidate_id)",
    "let scope_name = candidate.string.clone();",
    "confirm must resolve the candidate before reading its scope name",
  );
  assert.doesNotMatch(
    confirm,
    /self\.candidates\s*\[[^\]]+\]/,
    "confirm must not direct-index candidates from stale fuzzy matches",
  );

  assert.match(
    renderMatch,
    /self\.candidates\s*\.get\(mat\.candidate_id\)\?/,
    "render_match must return None for stale fuzzy candidate ids",
  );
  assertBefore(
    renderMatch,
    "self.candidates.get(mat.candidate_id)?",
    "LanguageName::new(&candidate.string).lsp_id()",
    "render_match must resolve the candidate before deriving the scope file label",
  );
  assert.doesNotMatch(
    renderMatch,
    /self\.candidates\s*\[[^\]]+\]/,
    "render_match must not direct-index candidates from stale fuzzy matches",
  );
});

test("snippet source guard stays in worker-owned files", () => {
  assert.equal(snippetsUiPath, "crates/snippets_ui/src/snippets_ui.rs");
  assert.doesNotMatch(snippetsUiPath, /test/i);
});
