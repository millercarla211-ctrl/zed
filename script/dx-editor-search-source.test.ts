import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const functionBody = (source: string, name: string) => {
  const start = source.search(new RegExp(`fn\\s+${name}(?:<[^>]+>)?\\s*\\(`));
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
  const beforeIndex =
    typeof before === "string" ? body.indexOf(before) : body.match(before)?.index ?? -1;
  const afterIndex =
    typeof after === "string" ? body.indexOf(after) : body.match(after)?.index ?? -1;
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("editor search match navigation clamps stale match indexes before deriving a cursor", () => {
  const source = read("crates/editor/src/items.rs");
  const matchIndexForDirection = functionBody(source, "match_index_for_direction");

  assert.doesNotMatch(
    matchIndexForDirection,
    /matches\s*\[\s*current_index\s*\]/,
    "stale search indexes must not directly index matches",
  );
  assert.match(
    matchIndexForDirection,
    /let Some\(last_match_index\) = matches\.len\(\)\.checked_sub\(1\) else \{\s*return 0;\s*\};/s,
    "empty match lists must be handled before cursor derivation",
  );
  assert.match(
    matchIndexForDirection,
    /let current_match_index = current_index\.min\(last_match_index\);/,
    "stale current indexes must clamp to a valid match index",
  );
  assertBefore({
    body: matchIndexForDirection,
    before: "let current_match_index = current_index.min(last_match_index);",
    after: "let cursor =",
    message: "the current match index must be bounded before deriving the cursor",
  });
});
