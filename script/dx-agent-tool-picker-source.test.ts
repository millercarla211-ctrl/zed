import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/agent_ui/src/agent_configuration/tool_picker.rs";
const source = readFileSync(sourcePath, "utf8");

function sliceBetween(startNeedle: string, endNeedle: string): string {
  const start = source.indexOf(startNeedle);
  assert.notEqual(start, -1, `expected ${startNeedle}`);

  const end = source.indexOf(endNeedle, start + startNeedle.length);
  assert.ok(end > start, `expected ${endNeedle} after ${startNeedle}`);

  return source.slice(start, end);
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

test("tool picker bounds visible tool rows while preserving context server headers", () => {
  const updateMatches = sliceBetween("fn update_matches(", "\n    fn confirm(");

  assert.match(source, /const MAX_TOOL_PICKER_TOOL_ROWS: usize = \d+;/);
  assert.match(updateMatches, /let mut tool_row_count = 0;/);
  assertBefore(
    updateMatches,
    "if tool_row_count >= MAX_TOOL_PICKER_TOOL_ROWS",
    "tools_by_provider.entry(server_id).or_default().push(name);",
    "tool rows must be capped before collecting visible rows",
  );
  assertBefore(
    updateMatches,
    "tools_by_provider.entry(server_id).or_default().push(name);",
    "tool_row_count += 1;",
    "the bounded visible row count must advance after collecting a tool row",
  );
  assertBefore(
    updateMatches,
    "items.push(PickerItem::ContextServer { server_id });",
    "items.push(PickerItem::Tool",
    "MCP context server headers must remain before their visible tool rows",
  );
});

test("tool picker clamps stale selections and avoids direct filtered item indexing", () => {
  const delegateImpl = sliceBetween("impl ToolPickerDelegate {", "\nimpl PickerDelegate");
  const setSelectedIndex = sliceBetween("fn set_selected_index(", "\n    fn can_select(");
  const canSelect = sliceBetween("fn can_select(", "\n    fn placeholder_text(");
  const updateMatches = sliceBetween("fn update_matches(", "\n    fn confirm(");
  const confirm = sliceBetween("fn confirm(", "\n    fn dismissed(");

  assert.match(
    delegateImpl,
    /fn clamp_selected_index\(&mut self\)\s*\{\s*self\.selected_index = self\s*\.selected_index\s*\.min\(self\.filtered_items\.len\(\)\.saturating_sub\(1\)\);\s*\}/,
  );
  assertBefore(
    setSelectedIndex,
    "self.selected_index = ix;",
    "self.clamp_selected_index();",
    "explicit selection changes must be clamped to the current filtered rows",
  );
  assertBefore(
    updateMatches,
    "this.delegate.filtered_items = filtered_items;",
    "this.delegate.clamp_selected_index();",
    "async match replacement must clamp stale selection before the picker is used again",
  );
  assert.match(canSelect, /self\.filtered_items\.get\(ix\)/);
  assert.doesNotMatch(canSelect, /self\.filtered_items\[ix\]/);
  assert.match(confirm, /self\.filtered_items\.get\(self\.selected_index\)/);
  assert.doesNotMatch(confirm, /self\.filtered_items\[self\.selected_index\]/);
});

test("tool picker guard is focused on production source", () => {
  assert.equal(sourcePath, "crates/agent_ui/src/agent_configuration/tool_picker.rs");
  assert.doesNotMatch(sourcePath, /test/i);
  assert.doesNotMatch(
    source,
    /#\[cfg\(test\)\]/,
    "tool picker source guard should cover production code, not test-only code",
  );
});
