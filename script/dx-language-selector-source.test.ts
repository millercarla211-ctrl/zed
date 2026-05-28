import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8").replace(/\r\n/g, "\n");
const productionSource = (source: string) =>
  source.split(/\n#\[cfg\(test\)\]\nmod tests\s*\{/)[0] ?? source;

const sourcePath = "crates/language_selector/src/language_selector.rs";
const source = productionSource(read(sourcePath));

function functionBody(sourceText: string, name: string): string {
  const fnIndex = sourceText.indexOf(`fn ${name}`);
  assert.notEqual(fnIndex, -1, `expected ${name}`);

  const bodyStart = sourceText.indexOf("{", fnIndex);
  assert.ok(bodyStart > fnIndex, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < sourceText.length; index += 1) {
    const char = sourceText[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return sourceText.slice(fnIndex, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
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

test("language selector declares focused picker materialization caps", () => {
  assert.match(source, /const MAX_LANGUAGE_SELECTOR_CANDIDATES: usize = \d+;/);
  assert.match(source, /const MAX_LANGUAGE_SELECTOR_MATCHES: usize = MAX_LANGUAGE_SELECTOR_CANDIDATES;/);
  assert.match(source, /const MAX_LANGUAGE_SELECTOR_FUZZY_MATCHES: usize = \d+;/);
});

test("language selector caps candidates while preserving the current language", () => {
  const delegateImplStart = source.indexOf("impl LanguageSelectorDelegate {");
  assert.notEqual(delegateImplStart, -1, "expected LanguageSelectorDelegate impl");
  const constructor = functionBody(source.slice(delegateImplStart), "new");
  const candidateHelper = functionBody(source, "capped_language_selector_candidates");

  assert.match(
    constructor,
    /capped_language_selector_candidates\(\s*&language_registry,\s*current_language_name\.as_deref\(\),\s*\)/,
  );
  assertBefore(
    candidateHelper,
    "candidates.len() < MAX_LANGUAGE_SELECTOR_CANDIDATES",
    "StringMatchCandidate::new(candidate_id, name.as_ref())",
    "language candidates must be capped before picker candidate materialization",
  );
  assert.match(candidateHelper, /current_language_name\s*\.is_some_and/);
  assert.match(candidateHelper, /candidates\.pop\(\);/);
  assert.match(candidateHelper, /candidates\.push\(current_language\);/);
});

test("language selector caps empty and fuzzy matches before picker rows", () => {
  const emptyMatches = functionBody(source, "capped_empty_query_matches");
  const updateMatches = functionBody(source, "update_matches");

  assertBefore(
    emptyMatches,
    ".take(MAX_LANGUAGE_SELECTOR_MATCHES)",
    ".collect()",
    "empty query matches must be capped before vector materialization",
  );
  assert.match(updateMatches, /capped_empty_query_matches\(candidates\)/);
  assert.match(updateMatches, /MAX_LANGUAGE_SELECTOR_FUZZY_MATCHES/);
  assert.doesNotMatch(updateMatches, /\n\s*100,\n\s*&Default::default\(\),/);
});

test("language selector clamps stale selection after match replacement", () => {
  const setter = functionBody(source, "set_selected_index");
  const clampHelper = functionBody(source, "clamp_selected_index_to_matches");
  const updateMatches = functionBody(source, "update_matches");

  assert.match(setter, /self\.selected_index = self\.clamped_match_index\(ix\);/);
  assert.match(clampHelper, /self\.selected_index = self\.clamped_match_index\(self\.selected_index\);/);
  assertBefore(
    updateMatches,
    "this.delegate.matches = matches;",
    "this.delegate.clamp_selected_index_to_matches();",
    "new matches must be stored before clamping stale selection",
  );
  assertBefore(
    updateMatches,
    "this.delegate.clamp_selected_index_to_matches();",
    "this.set_selected_index(selected_index, None, false, window, cx);",
    "selection must be clamped before picker selection state is updated",
  );
});

test("language selector source guard stays scoped to worker-owned files", () => {
  assert.equal(sourcePath, "crates/language_selector/src/language_selector.rs");
  assert.doesNotMatch(sourcePath, /DX\.md|todo\.txt|changelog\.txt/);
});
