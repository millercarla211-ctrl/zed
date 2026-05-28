import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

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

test("documentation cache key promotion checks stale cache indexes", () => {
  const source = read("crates/editor/src/code_context_menus.rs");
  const getOrCreateMarkdown = functionBody(source, "get_or_create_markdown");

  assert.doesNotMatch(
    getOrCreateMarkdown,
    /markdown_cache\s*\[\s*cache_index\s*\]\s*\.0\s*=/,
    "completion documentation cache key promotion must not direct-index by a stale cache_index",
  );
  assert.match(
    getOrCreateMarkdown,
    /markdown_cache\.get_mut\(cache_index\)/,
    "completion documentation cache key promotion must use a checked cache entry mutation",
  );
});
