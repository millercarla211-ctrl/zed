import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const source = readFileSync("crates/prompt_store/src/prompt_store.rs", "utf8")
  .replace(/\r\n/g, "\n");

function functionBody(name: string): string {
  const fnIndex = source.indexOf(`pub fn ${name}(`);
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

function sliceBetween(haystack: string, startNeedle: string, endNeedle: string): string {
  const start = haystack.indexOf(startNeedle);
  assert.notEqual(start, -1, `expected ${startNeedle}`);

  const end = haystack.indexOf(endNeedle, start + startNeedle.length);
  assert.ok(end > start, `expected ${endNeedle} after ${startNeedle}`);

  return haystack.slice(start, end);
}

test("prompt store search keeps empty-query behavior and default sorting", () => {
  const search = functionBody("search");

  assert.match(search, /if query\.is_empty\(\)\s*\{\s*cached_metadata\s*\}/);
  assert.match(
    search,
    /matches\.sort_by_key\(\|metadata\| Reverse\(metadata\.default\)\);/,
  );
});

test("prompt store fuzzy search does not index cached metadata by candidate id", () => {
  const search = functionBody("search");
  const fuzzyMapping = sliceBetween(
    search,
    "fuzzy::match_strings(",
    "matches.sort_by_key",
  );

  assert.doesNotMatch(
    fuzzyMapping,
    /cached_metadata\s*\[\s*mat\.candidate_id\s*\]/,
    "fuzzy result materialization must skip stale candidate ids instead of indexing cached metadata",
  );
});

test("prompt store fuzzy search uses checked cached metadata lookup", () => {
  const search = functionBody("search");
  const fuzzyMapping = sliceBetween(
    search,
    "fuzzy::match_strings(",
    "matches.sort_by_key",
  );

  assert.match(
    fuzzyMapping,
    /cached_metadata\s*\.get\(mat\.candidate_id\)\s*\.cloned\(\)/,
    "fuzzy result materialization must use cached_metadata.get(mat.candidate_id).cloned()",
  );
});
