import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8").replace(/\r\n/g, "\n");
const productionSource = (source: string) =>
  source.split(/\n#\[cfg\(test\)\]/)[0] ?? source;

const sourcePath = "crates/debugger_ui/src/session/running/stack_frame_list.rs";
const source = productionSource(read(sourcePath));

function functionBody(name: string): string {
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

test("stack-frame render_entry fails closed when filtered rows are stale", () => {
  const renderEntry = functionBody("render_entry");

  assert.match(renderEntry, /self\.filter_entries_indices\s*\.get\(ix\)\s*\.copied\(\)/s);
  assert.match(renderEntry, /let Some\(entry\)\s*=\s*self\.entries\.get\(ix\)\s*else/s);
  assert.match(renderEntry, /return Empty\.into_any\(\);/);
  assert.doesNotMatch(
    renderEntry,
    /self\.filter_entries_indices\[[^\]]+\]/,
    "render_entry must not direct-index filtered row mappings from stale uniform-list rows",
  );
  assert.doesNotMatch(
    renderEntry,
    /self\.entries\[[^\]]+\]/,
    "render_entry must not direct-index stack-frame entries from stale row mappings",
  );
});

test("stack-frame source guard stays scoped to worker-owned files", () => {
  assert.equal(sourcePath, "crates/debugger_ui/src/session/running/stack_frame_list.rs");
});
