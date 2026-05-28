import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const indexOfPattern = (source: string, pattern: string | RegExp) => {
  if (typeof pattern === "string") {
    return source.indexOf(pattern);
  }

  return source.match(pattern)?.index ?? -1;
};

const functionBody = (source: string, name: string) => {
  const start = source.search(new RegExp(`fn\\s+${name}\\s*\\(`));
  assert.ok(start >= 0, `expected ${name}`);

  const bodyStart = source.indexOf("{", start);
  assert.ok(bodyStart > start, `expected ${name} body`);

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
};

const assertBefore = ({
  body,
  before,
  after,
  message,
}: {
  body: string;
  before: string | RegExp;
  after: string | RegExp;
  message: string;
}) => {
  const beforeIndex = indexOfPattern(body, before);
  const afterIndex = indexOfPattern(body, after);

  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("editor search match activation checks stale indexes before selecting a match", () => {
  const source = read("crates/editor/src/items.rs");
  const activateMatch = functionBody(source, "activate_match");

  assert.doesNotMatch(
    activateMatch,
    /matches\s*\[\s*index\s*\]/,
    "activate_match must not direct-index matches with a caller-provided index",
  );
  assert.match(
    activateMatch,
    /matches\.get\(\s*index\s*\)/,
    "activate_match must check the requested match index",
  );
  assertBefore({
    body: activateMatch,
    before: /matches\.get\(\s*index\s*\)/,
    after: "unfold_ranges",
    message: "activate_match must check the match index before unfolding ranges",
  });
  assertBefore({
    body: activateMatch,
    before: /matches\.get\(\s*index\s*\)/,
    after: "range_for_match",
    message: "activate_match must check the match index before selecting it",
  });
});

test("editor search match navigation checks the current match before deriving fallback cursor", () => {
  const source = read("crates/editor/src/items.rs");
  const matchIndexForDirection = functionBody(source, "match_index_for_direction");

  assert.doesNotMatch(
    matchIndexForDirection,
    /matches\s*\[\s*current_match_index\s*\]\s*\.start/,
    "match_index_for_direction must not direct-index the current match cursor",
  );
  assert.match(
    matchIndexForDirection,
    /matches\.get\(\s*current_match_index\s*\)/,
    "match_index_for_direction must check the bounded current match index",
  );
  assert.match(
    matchIndexForDirection,
    /let Some\(current_match\) = matches\.get\(current_match_index\) else \{\s*return current_match_index;\s*\};\s*current_match\.start/s,
    "match_index_for_direction must fail closed if the current match is stale before cursor derivation",
  );
});
