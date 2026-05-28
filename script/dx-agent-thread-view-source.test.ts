import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/agent_ui/src/conversation_view/thread_view.rs";
const source = readFileSync(sourcePath, "utf8");

const cycleThinkingEffort = sliceBetween(
  "fn cycle_thinking_effort(",
  "\n    fn toggle_thinking_effort_menu(",
);

test("thread view thinking effort cycling checks stale next indexes before reading the effort", () => {
  assert.doesNotMatch(
    cycleThinkingEffort,
    /effort_levels\s*\[\s*next_index\s*\]/,
    "thinking effort cycling must not directly index effort_levels with next_index",
  );
  assert.match(
    cycleThinkingEffort,
    /let\s+Some\(\w+\)\s*=\s*effort_levels\.get\(next_index\)\s*else\s*\{\s*return;\s*\};/s,
    "thinking effort cycling must use a checked lookup and return early for stale next indexes",
  );
  assertBefore(
    cycleThinkingEffort,
    "effort_levels.get(next_index)",
    "thread.update(cx, |thread, cx| {",
    "the checked effort lookup must happen before mutating thread/settings state",
  );
});

test("thread view source guard stays scoped to production thread view code", () => {
  assert.equal(sourcePath, "crates/agent_ui/src/conversation_view/thread_view.rs");
  assert.doesNotMatch(sourcePath, /test/i);
  assert.doesNotMatch(cycleThinkingEffort, /#\[cfg\(test\)\]/);
});

function sliceBetween(start: string, end: string): string {
  const startIndex = source.indexOf(start);
  assert.notEqual(startIndex, -1, `expected ${start}`);

  const endIndex = source.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `expected ${end} after ${start}`);

  return source.slice(startIndex, endIndex);
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
