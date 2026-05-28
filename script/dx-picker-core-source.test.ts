import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8").replace(/\r\n/g, "\n");
const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const source = productionSource(read("crates/picker/src/picker.rs"));

const sliceBetween = (haystack: string, start: string, end: string) => {
  const startIndex = haystack.indexOf(start);
  assert.notEqual(startIndex, -1, `expected ${start}`);
  const endIndex = haystack.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `expected ${end} after ${start}`);
  return haystack.slice(startIndex, endIndex);
};

const assertBefore = (
  haystack: string,
  before: string,
  after: string,
  message: string,
) => {
  const beforeIndex = haystack.indexOf(before);
  const afterIndex = haystack.indexOf(after);
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("picker clamps out-of-range selection requests before selectable fallback loops", () => {
  const setter = sliceBetween(
    source,
    "pub fn set_selected_index(",
    "\n    pub fn select_next(",
  );

  assertBefore(
    setter,
    "if match_count == 0 {\n            return;\n        }",
    "ix = ix.min(match_count - 1);",
    "empty-list exits must happen before clamping with match_count - 1",
  );
  assertBefore(
    setter,
    "ix = ix.min(match_count - 1);",
    "if let Some(bias) = fallback_direction",
    "selection requests must be in range before directional fallback starts",
  );
  assertBefore(
    setter,
    "ix = ix.min(match_count - 1);",
    "while !self.delegate.can_select(curr_ix, window, cx)",
    "selectable fallback loops must not start from an out-of-range index",
  );
});

test("picker clamps stale delegate selection after match updates before reveal scrolling", () => {
  const clampHelper = sliceBetween(
    source,
    "fn clamp_selected_index_to_match_count(",
    "\n    fn matches_updated(",
  );
  const matchesUpdated = sliceBetween(
    source,
    "fn matches_updated(",
    "\n    pub fn query(",
  );

  assert.match(clampHelper, /if match_count == 0\s*\{\s*return;\s*\}/);
  assert.match(clampHelper, /let selected_index = self\.delegate\.selected_index\(\);/);
  assert.match(clampHelper, /if selected_index >= match_count\s*\{/);
  assert.match(
    clampHelper,
    /self\.set_selected_index\(match_count - 1, Some\(Direction::Up\), false, window, cx\);/,
  );
  assertBefore(
    matchesUpdated,
    "let match_count = self.delegate.match_count();",
    "self.clamp_selected_index_to_match_count(match_count, window, cx);",
    "matches_updated must know the new count before clamping stale selection",
  );
  assertBefore(
    matchesUpdated,
    "self.clamp_selected_index_to_match_count(match_count, window, cx);",
    "match &mut self.element_container",
    "selection must be clamped before list state reset or reveal scrolling",
  );
  assert.match(
    matchesUpdated,
    /if match_count > 0\s*\{\s*let index = self\.delegate\.selected_index\(\);\s*self\.scroll_to_item_index\(index\.min\(match_count - 1\)\);\s*\}/,
    "reveal scrolling must bound the target even if no selectable row accepted the clamp",
  );
});
