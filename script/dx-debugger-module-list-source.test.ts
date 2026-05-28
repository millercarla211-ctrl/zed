import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/debugger_ui/src/session/running/module_list.rs";
const source = readFileSync(sourcePath, "utf8").replace(/\r\n/g, "\n");

function functionBody(name) {
  const start = source.search(new RegExp(`fn\\s+${name}\\b`));
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
}

test("module-list render_entry fails closed when uniform-list rows are stale", () => {
  const renderEntry = functionBody("render_entry");

  assert.match(
    renderEntry,
    /let Some\(module\)\s*=\s*self\.entries\.get\(ix\)\.cloned\(\)\s*else\s*\{/s,
  );
  assert.match(renderEntry, /return Empty\.into_any\(\);/);
  assert.doesNotMatch(
    renderEntry,
    /self\s*\.\s*entries\s*\[[^\]]+\]/,
    "render_entry must not direct-index module entries from stale uniform-list rows",
  );
});

test("module-list source guard stays scoped to worker-owned files", () => {
  assert.equal(sourcePath, "crates/debugger_ui/src/session/running/module_list.rs");
});
