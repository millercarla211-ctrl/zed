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

test("line rotation remaps selections through checked stale-safe lookups", () => {
  const source = read("crates/editor/src/editor.rs");
  const body = functionBody(source, "rotate_selections");

  assert.doesNotMatch(
    body,
    /row_to_index\s*\[\s*&point\.row\s*\]/,
    "line rotation must not panic on stale selection rows",
  );
  assert.doesNotMatch(
    body,
    /new_line_starts\s*\[\s*new_index\s*\]/,
    "line rotation must not panic on stale remapped line indexes",
  );
  assert.match(
    body,
    /let\s+Some\(&old_index\)\s*=\s*row_to_index\.get\(&point\.row\)\s*else\s*\{\s*return\s+selection\.clone\(\);\s*\};/s,
    "stale selection rows must preserve the original selection",
  );
  assert.match(
    body,
    /let\s+Some\(&new_line_start\)\s*=\s*new_line_starts\.get\(new_index\)\s*else\s*\{\s*return\s+selection\.clone\(\);\s*\};/s,
    "stale remapped line indexes must preserve the original selection",
  );
  assert.match(
    body,
    /MultiBufferOffset\(new_line_start \+ point\.column as usize\)/,
    "valid remaps must still use the rotated line start and original column",
  );
});
