import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/command_palette/src/command_palette.rs";
const source = readFileSync(sourcePath, "utf8");
const productionSource = source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const functionBody = (name: string) => {
  const start = productionSource.search(new RegExp(`fn\\s+${name}\\s*\\(`));
  assert.ok(start >= 0, `expected ${name}`);

  const bodyStart = productionSource.indexOf("{", start);
  assert.ok(bodyStart > start, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < productionSource.length; index += 1) {
    const char = productionSource[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return productionSource.slice(start, index + 1);
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
  before: string;
  after: string;
  message: string;
}) => {
  const beforeIndex = body.indexOf(before);
  const afterIndex = body.indexOf(after);
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("command palette clamps stale selection before command materialization", () => {
  const clampSelectedIndex = functionBody("clamp_selected_index");
  const matchesUpdated = functionBody("matches_updated");
  const setSelectedIndex = functionBody("set_selected_index");
  const confirm = functionBody("confirm");

  assert.match(clampSelectedIndex, /self\.matches\.len\(\)\.checked_sub\(1\)/);
  assert.match(clampSelectedIndex, /cmp::min\(self\.selected_ix, max_ix\)/);
  assert.match(clampSelectedIndex, /self\.selected_ix = 0;/);

  assertBefore({
    body: matchesUpdated,
    before: "self.matches = new_matches;",
    after: "self.clamp_selected_index();",
    message: "match updates must clamp stale selected indexes after replacing results",
  });
  assertBefore({
    body: setSelectedIndex,
    before: "self.selected_ix = ix;",
    after: "self.clamp_selected_index();",
    message: "picker selection changes must clamp before later command materialization",
  });

  assert.match(
    confirm,
    /let Some\(matching_command\) = self\.matches\.get\(self\.selected_ix\) else \{\s+return;\s+\};/,
  );
  assert.doesNotMatch(
    confirm,
    /self\.matches\[self\.selected_ix\]/,
    "confirm must not index matches directly with a potentially stale selected_ix",
  );
});
