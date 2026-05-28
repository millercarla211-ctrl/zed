import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/encoding_selector/src/encoding_selector.rs";

const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const source = productionSource(readFileSync(sourcePath, "utf8"));

function functionBody(sourceText: string, name: string): string {
  const fnIndex = sourceText.search(new RegExp(`fn\\s+${name}\\s*\\(`));
  assert.ok(fnIndex >= 0, `expected ${name}`);

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

  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("encoding selector declares focused picker materialization caps", () => {
  assert.match(source, /const MAX_ENCODING_SELECTOR_MATCHES: usize = 256;/);
  assert.match(source, /const MAX_ENCODING_SELECTOR_FUZZY_MATCHES: usize = 100;/);
});

test("encoding selector caps empty and fuzzy match materialization", () => {
  const emptyMatches = functionBody(source, "capped_empty_query_matches");
  const updateMatches = functionBody(source, "update_matches");

  assertBefore(
    emptyMatches,
    ".take(MAX_ENCODING_SELECTOR_MATCHES)",
    ".collect()",
    "empty query rows must be capped before vector materialization",
  );
  assert.match(emptyMatches, /candidate_id: candidate\.id/);
  assert.match(updateMatches, /capped_empty_query_matches\(candidates\.as_ref\(\)\)/);
  assert.match(updateMatches, /MAX_ENCODING_SELECTOR_FUZZY_MATCHES/);
  assert.doesNotMatch(updateMatches, /\n\s*100,\n\s*&Default::default\(\),/);
});

test("encoding selector clamps stale picker selection", () => {
  const setter = functionBody(source, "set_selected_index");
  const clampIndex = functionBody(source, "clamped_match_index");
  const clampSelected = functionBody(source, "clamp_selected_index_to_matches");
  const updateMatches = functionBody(source, "update_matches");

  assert.match(clampIndex, /self\.matches\s*\.len\(\)\s*\.checked_sub\(1\)/);
  assert.match(setter, /self\.selected_index = self\.clamped_match_index\(ix\);/);
  assert.match(clampSelected, /self\.selected_index = self\.clamped_match_index\(self\.selected_index\);/);
  assertBefore(
    updateMatches,
    "delegate.matches = matches;",
    "delegate.clamp_selected_index_to_matches();",
    "new matches must be installed before stale selection is reclamped",
  );
});

test("encoding selector confirms candidate IDs through guarded lookup", () => {
  const confirm = functionBody(source, "confirm");

  assert.match(confirm, /self\.encodings\.get\(mat\.candidate_id\)\.copied\(\)/);
  assert.doesNotMatch(
    confirm,
    /self\.encodings\[[^\]]+\]/,
    "confirm must not index encoding candidates from stale match IDs",
  );
});

test("encoding selector source guard stays scoped to worker-owned files", () => {
  assert.equal(sourcePath, "crates/encoding_selector/src/encoding_selector.rs");
  assert.doesNotMatch(sourcePath, /DX\.md|todo\.txt|changelog\.txt/);
  assert.doesNotMatch(
    source,
    /#\[cfg\(test\)\]/,
    "source guard should only inspect production encoding selector code",
  );
});
