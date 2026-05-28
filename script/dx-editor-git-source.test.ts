import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const indexOfPattern = (source: string, pattern: string | RegExp) => {
  if (typeof pattern === "string") {
    return source.indexOf(pattern);
  }

  return source.match(pattern)?.index ?? -1;
};

const functionBody = (source: string, name: string) => {
  const start = source.search(
    new RegExp(`(?:pub\\s+)?fn\\s+${name}(?:<[^>]+>)?\\s*\\(`),
  );
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

test("diff review comment submission checks stale overlay indexes before reading comment state", () => {
  const source = productionSource(read("crates/editor/src/git.rs"));
  const body = functionBody(source, "submit_diff_review_comment");

  assert.doesNotMatch(
    body,
    /diff_review_overlays\s*\[\s*overlay_index\s*\]/,
    "diff review comment submission must not direct-index stale overlay indexes",
  );
  assert.match(
    body,
    /let\s+Some\(overlay\)\s*=\s*self\s*\.\s*diff_review_overlays\s*\.\s*get\(overlay_index\)\s*else\s*\{\s*return;\s*\};/,
    "diff review comment submission must fail closed with a checked overlay lookup",
  );
  assertBefore({
    body,
    before: /diff_review_overlays\s*\.\s*get\(overlay_index\)/,
    after: /prompt_editor\s*\.\s*read\(cx\)/,
    message: "focused overlay lookup must be checked before reading comment text",
  });
  assertBefore({
    body,
    before: /diff_review_overlays\s*\.\s*get\(overlay_index\)/,
    after: "anchor_range.clone()",
    message: "focused overlay lookup must be checked before cloning anchor state",
  });
  assertBefore({
    body,
    before: /diff_review_overlays\s*\.\s*get\(overlay_index\)/,
    after: "hunk_key.clone()",
    message: "focused overlay lookup must be checked before cloning hunk state",
  });
});
