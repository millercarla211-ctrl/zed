import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/component_preview/src/component_preview.rs";

const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const source = productionSource(
  readFileSync(sourcePath, "utf8").replace(/\r\n/g, "\n"),
);

const functionBody = (haystack: string, name: string) => {
  const start = haystack.search(new RegExp(`fn\\s+${name}\\s*\\(`));
  assert.ok(start >= 0, `expected ${name}`);

  const bodyStart = haystack.indexOf("{", start);
  assert.ok(bodyStart > start, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < haystack.length; index += 1) {
    const char = haystack[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return haystack.slice(start, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
};

test("component list rows use explicit get guards before rendering entries", () => {
  const renderAllComponents = functionBody(source, "render_all_components");

  assert.match(
    renderAllComponents,
    /let Some\(entry\) = this\.entries\.get\(ix\) else\s*\{\s*return div\(\)\.w_full\(\)\.h_0\(\)\.into_any_element\(\);\s*\};/,
    "list row rendering should fail closed when the row index is stale",
  );
  assert.doesNotMatch(
    renderAllComponents,
    /this\.entries\s*\[\s*ix\s*\]/,
    "list row rendering must not directly index entries by row index",
  );
});

test("component sidebar rows use filter_map plus get guards", () => {
  const render = functionBody(source, "render");

  assert.match(
    render,
    /\.filter_map\(\|ix\| \{\s*sidebar_entries\s*\.get\(ix\)\s*\.map\(\|entry\| this\.render_sidebar_entry\(ix, entry, cx\)\)\s*\}\)/,
    "sidebar row rendering should skip stale row indexes with filter_map",
  );
  assert.doesNotMatch(
    render,
    /sidebar_entries\s*\[\s*ix\s*\]/,
    "sidebar row rendering must not directly index entries by row index",
  );
});

test("Component Preview source guard stays focused on production code", () => {
  assert.equal(sourcePath, "crates/component_preview/src/component_preview.rs");
  assert.doesNotMatch(sourcePath, /test/i);
  assert.doesNotMatch(
    source,
    /#\[cfg\(test\)\]/,
    "source guard should only inspect production Component Preview code",
  );
});
