import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/agent_ui/src/entry_view_state.rs";
const source = readFileSync(sourcePath, "utf8");
const productionSource =
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

function sliceBetween(startNeedle: string, endNeedle: string): string {
  const start = productionSource.indexOf(startNeedle);
  assert.notEqual(start, -1, `expected ${startNeedle}`);
  const end = productionSource.indexOf(endNeedle, start + startNeedle.length);
  assert.ok(end > start, `expected ${endNeedle} after ${startNeedle}`);
  return productionSource.slice(start, end);
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

function assertPatternBefore(
  haystack: string,
  before: RegExp,
  after: RegExp,
  message: string,
) {
  const beforeIndex = haystack.match(before)?.index ?? -1;
  const afterIndex = haystack.match(after)?.index ?? -1;
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("entry view caps tool-call terminal and diff rows before display materialization", () => {
  assert.match(
    productionSource,
    /const MAX_ENTRY_VIEW_TOOL_CALL_TERMINALS: usize = 64;/,
  );
  assert.match(
    productionSource,
    /const MAX_ENTRY_VIEW_TOOL_CALL_DIFFS: usize = 128;/,
  );

  const toolCallBranch = sliceBetween(
    "AgentThreadEntry::ToolCall(tool_call) => {",
    "AgentThreadEntry::AssistantMessage(message) => {",
  );
  const terminalCollection = toolCallBranch.slice(
    toolCallBranch.indexOf("let terminals ="),
    toolCallBranch.indexOf("let diffs ="),
  );
  const diffCollection = toolCallBranch.slice(
    toolCallBranch.indexOf("let diffs ="),
    toolCallBranch.indexOf("let views ="),
  );

  assertPatternBefore(
    terminalCollection,
    /\.take\(MAX_ENTRY_VIEW_TOOL_CALL_TERMINALS\)/,
    /\.cloned\(\)\s+\.collect::<Vec<_>>\(\)/,
    "terminal display rows must be capped before cloning into a Vec",
  );
  assertPatternBefore(
    diffCollection,
    /\.take\(MAX_ENTRY_VIEW_TOOL_CALL_DIFFS\)/,
    /\.cloned\(\)\s+\.collect::<Vec<_>>\(\)/,
    "diff display rows must be capped before cloning into a Vec",
  );
  assert.doesNotMatch(
    toolCallBranch,
    /tool_call\.terminals\(\)\.cloned\(\)\.collect::<Vec<_>>\(\)/,
    "terminals must not collect all display rows before applying a cap",
  );
  assert.doesNotMatch(
    toolCallBranch,
    /tool_call\.diffs\(\)\.cloned\(\)\.collect::<Vec<_>>\(\)/,
    "diffs must not collect all display rows before applying a cap",
  );
});

test("entry view prunes cached tool-call views to the capped display ids before rendering rows", () => {
  const toolCallBranch = sliceBetween(
    "AgentThreadEntry::ToolCall(tool_call) => {",
    "AgentThreadEntry::AssistantMessage(message) => {",
  );

  assert.match(
    toolCallBranch,
    /let displayed_content_ids = terminals\s+\.iter\(\)\s+\.chain\(diffs\.iter\(\)\)/,
  );
  assert.match(toolCallBranch, /views\.retain\(\|entity_id, _\|/);
  assertBefore(
    toolCallBranch,
    "let displayed_content_ids = terminals",
    "views.retain(|entity_id, _| displayed_content_ids.contains(entity_id));",
    "cached entry-view entities must be reduced to the capped display ids",
  );
  assertBefore(
    toolCallBranch,
    "views.retain(|entity_id, _| displayed_content_ids.contains(entity_id));",
    "for terminal in terminals",
    "stale cached terminal/diff views must be pruned before terminal rows render",
  );
  assertBefore(
    toolCallBranch,
    "for terminal in terminals",
    "for diff in diffs",
    "entry view should preserve the existing terminal-before-diff display order",
  );
});

test("entry view state guard stays focused on production source", () => {
  assert.equal(sourcePath, "crates/agent_ui/src/entry_view_state.rs");
  assert.doesNotMatch(sourcePath, /test/i);
  const toolCallBranch = sliceBetween(
    "AgentThreadEntry::ToolCall(tool_call) => {",
    "AgentThreadEntry::AssistantMessage(message) => {",
  );
  assert.doesNotMatch(toolCallBranch, /#\[cfg\(test\)\]/);
});
