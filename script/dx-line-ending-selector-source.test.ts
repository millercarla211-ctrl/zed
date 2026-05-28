import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8").replace(/\r\n/g, "\n");
const productionSource = (source: string) =>
  source.split(/\n#\[cfg\(test\)\]\nmod tests\s*\{/)[0] ?? source;

const sourcePath = "crates/line_ending_selector/src/line_ending_selector.rs";
const source = productionSource(read(sourcePath));

function functionBody(sourceText: string, name: string): string {
  const fnIndex = sourceText.indexOf(`fn ${name}`);
  assert.notEqual(fnIndex, -1, `expected ${name}`);

  const bodyStart = sourceText.indexOf("{", fnIndex);
  assert.ok(bodyStart > fnIndex, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < sourceText.length; index += 1) {
    const char = sourceText[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return sourceText.slice(fnIndex, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
}

function assertBefore(
  haystack: string,
  before: string,
  after: string,
  message: string,
) {
  const beforeIndex = haystack.indexOf(before);
  const afterIndex = haystack.indexOf(after);
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("line ending selector preserves the two production choices", () => {
  assert.match(
    source,
    /matches:\s*vec!\[\s*LineEnding::Unix,\s*LineEnding::Windows,?\s*\]/,
  );
});

test("line ending selector clamps stale selection before setter and confirm use", () => {
  const clampIndex = functionBody(source, "clamped_match_index");
  const clampSelection = functionBody(source, "clamp_selected_index_to_matches");
  const confirm = functionBody(source, "confirm");
  const setter = functionBody(source, "set_selected_index");

  assert.match(clampIndex, /self\.matches\.len\(\)\.saturating_sub\(1\)/);
  assert.match(clampIndex, /ix\.min\(/);
  assert.match(
    clampSelection,
    /self\.selected_index = self\.clamped_match_index\(self\.selected_index\);/,
  );
  assert.match(setter, /self\.selected_index = self\.clamped_match_index\(ix\);/);
  assert.doesNotMatch(setter, /self\.selected_index = ix;/);
  assertBefore(
    confirm,
    "self.clamp_selected_index_to_matches();",
    "self.matches.get(self.selected_index)",
    "confirm must clamp stale selection before materializing the line ending",
  );
  assertBefore(
    confirm,
    "self.matches.get(self.selected_index)",
    "this.set_line_ending(*line_ending, cx);",
    "confirm must materialize a bounded line ending before applying it",
  );
});

test("line ending selector source guard stays scoped to worker-owned files", () => {
  assert.equal(sourcePath, "crates/line_ending_selector/src/line_ending_selector.rs");
  assert.doesNotMatch(sourcePath, /DX\.md|todo\.txt|changelog\.txt/);
});
