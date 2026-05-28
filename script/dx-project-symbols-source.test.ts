import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/project_symbols/src/project_symbols.rs";

const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const source = productionSource(readFileSync(sourcePath, "utf8"));

function indexOfPattern(source: string, pattern: string | RegExp): number {
  if (typeof pattern === "string") {
    return source.indexOf(pattern);
  }

  return source.match(pattern)?.index ?? -1;
}

function assertBefore(
  haystack: string,
  before: string | RegExp,
  after: string | RegExp,
  message: string,
) {
  const beforeIndex = indexOfPattern(haystack, before);
  const afterIndex = indexOfPattern(haystack, after);
  assert.ok(beforeIndex >= 0, `expected ${before}`);
  assert.ok(afterIndex >= 0, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

function functionBody(source: string, name: string): string {
  const start = source.indexOf(`fn ${name}(`);
  assert.ok(start >= 0, `expected ${name}`);

  const bodyStart = source.indexOf("{", start);
  assert.ok(bodyStart > start, `expected body for ${name}`);

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
}

test("project symbols declares a UTF-8-safe fuzzy highlight range helper", () => {
  assert.match(
    source,
    /fn fuzzy_highlight_range\(label: &str, position: usize\) -> Option<Range<usize>>/,
  );

  const helper = functionBody(source, "fuzzy_highlight_range");
  assert.match(helper, /position >= label\.len\(\)/);
  assert.match(helper, /!label\.is_char_boundary\(position\)/);
  assertBefore(
    helper,
    /label\[position\.\.\]\s*\.char_indices\(\)\s*\.nth\(1\)/,
    "Some(position..end)",
    "helper must derive the end from the next UTF-8 character boundary",
  );
  assert.match(helper, /unwrap_or\(label\.len\(\)\)/);
});

test("project symbols filters stale fuzzy positions before render highlights", () => {
  const renderMatch = functionBody(source, "render_match");

  assertBefore(
    renderMatch,
    ".filter_map(|pos| fuzzy_highlight_range(&label, *pos))",
    "gpui::combine_highlights",
    "render_match must filter invalid highlight positions before combining highlights",
  );
  assert.doesNotMatch(
    renderMatch,
    /ceil_char_boundary\(pos \+ 1\)/,
    "render_match should not trust fuzzy positions with direct ceil_char_boundary calls",
  );
});

test("project symbols source guard is focused on production source", () => {
  assert.equal(sourcePath, "crates/project_symbols/src/project_symbols.rs");
  assert.doesNotMatch(sourcePath, /test/i);
  assert.doesNotMatch(
    source,
    /#\[cfg\(test\)\]/,
    "source guard should only inspect production project symbols code",
  );
});
