import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/title_bar/src/application_menu.rs";
const source = readFileSync(sourcePath, "utf8").replace(/\r\n/g, "\n");

test("application menu activation checks stale entry indexes before handle use", () => {
  const activation = functionBody("navigate_menus_in_direction");

  assert.doesNotMatch(
    activation,
    /self\s*\.\s*entries\s*\[\s*current_index\s*\]/,
    "application menu activation must not direct-index the current entry",
  );
  assert.doesNotMatch(
    activation,
    /self\s*\.\s*entries\s*\[\s*next_index\s*\]/,
    "application menu activation must not direct-index the next entry",
  );
  assert.match(
    activation,
    /let\s+Some\(current_entry\)\s*=\s*self\s*\.\s*entries\s*\.\s*get\(\s*current_index\s*\)\s*else\s*\{\s*return;\s*\};/s,
    "application menu activation must check the current entry before hiding it",
  );
  assert.match(
    activation,
    /let\s+Some\(next_entry\)\s*=\s*self\s*\.\s*entries\s*\.\s*get\(\s*next_index\s*\)\s*else\s*\{\s*return;\s*\};/s,
    "application menu activation must check the next entry before cloning its handle",
  );
  assert.match(activation, /current_entry\.handle\.hide\(cx\);/);
  assert.match(
    activation,
    /let\s+next_handle\s*=\s*next_entry\.handle\.clone\(\);/,
  );
});

test("title-bar source guard stays scoped to worker-owned files", () => {
  assert.equal(sourcePath, "crates/title_bar/src/application_menu.rs");
  assert.doesNotMatch(sourcePath, /test/i);
});

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
