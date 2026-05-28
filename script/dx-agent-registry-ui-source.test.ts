import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/agent_ui/src/agent_registry_ui.rs";
const source = readFileSync(sourcePath, "utf8");

function sliceBetween(startNeedle: string, endNeedle: string): string {
  const start = source.indexOf(startNeedle);
  assert.notEqual(start, -1, `expected ${startNeedle}`);
  const end = source.indexOf(endNeedle, start + startNeedle.length);
  assert.ok(end > start, `expected ${endNeedle} after ${startNeedle}`);
  return source.slice(start, end);
}

function assertBefore(haystack: string, before: string, after: string, message: string) {
  const beforeIndex = haystack.indexOf(before);
  const afterIndex = haystack.indexOf(after);
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("agent registry UI query text is bounded before search filtering", () => {
  const searchQuery = sliceBetween(
    "fn search_query(&self, cx: &mut App) -> Option<String> {",
    "fn filter_registry_agents(&mut self, cx: &mut Context<Self>) {",
  );
  const filterRegistryAgents = sliceBetween(
    "fn filter_registry_agents(&mut self, cx: &mut Context<Self>) {",
    "fn scroll_to_top(&mut self, cx: &mut Context<Self>) {",
  );

  assert.match(
    source,
    /const MAX_AGENT_REGISTRY_SEARCH_QUERY_CHARS: usize = \d+;/,
  );
  assert.match(searchQuery, /text_for_range\(MultiBufferOffset\(0\)\.\.snapshot\.len\(\)\)/);
  assert.match(searchQuery, /\.take\(MAX_AGENT_REGISTRY_SEARCH_QUERY_CHARS\)/);
  assert.match(searchQuery, /\.collect::<String>\(\)/);
  assertBefore(
    searchQuery,
    ".take(MAX_AGENT_REGISTRY_SEARCH_QUERY_CHARS)",
    ".collect::<String>()",
    "registry search query must be capped before string materialization",
  );
  assertBefore(
    searchQuery,
    ".collect::<String>()",
    ".to_lowercase()",
    "registry search query must be bounded before lowercasing",
  );
  assert.doesNotMatch(
    searchQuery,
    /\.text\(cx\)/,
    "registry search must not clone the full editor text before bounding",
  );
  assert.doesNotMatch(
    filterRegistryAgents,
    /search\.to_lowercase\(\)/,
    "filtering should consume the already-bounded lowercase query",
  );
});

test("agent registry UI caps filtered indices before storage and render counts", () => {
  const filterRegistryAgents = sliceBetween(
    "fn filter_registry_agents(&mut self, cx: &mut Context<Self>) {",
    "fn scroll_to_top(&mut self, cx: &mut Context<Self>) {",
  );
  const renderBody = sliceBetween(
    "impl Render for AgentRegistryPage {",
    "impl EventEmitter<ItemEvent> for AgentRegistryPage {}",
  );

  assert.match(
    source,
    /const MAX_FILTERED_AGENT_REGISTRY_RESULTS: usize = \d+;/,
  );
  assert.match(
    source,
    /const MAX_DISPLAYED_AGENT_REGISTRY_RESULTS: usize = MAX_FILTERED_AGENT_REGISTRY_RESULTS;/,
  );
  assertBefore(
    filterRegistryAgents,
    ".take(MAX_FILTERED_AGENT_REGISTRY_RESULTS)",
    ".collect()",
    "filtered registry indices must be capped before collection",
  );
  assertBefore(
    filterRegistryAgents,
    ".take(MAX_FILTERED_AGENT_REGISTRY_RESULTS)",
    "self.filtered_registry_indices = filtered_indices;",
    "filtered registry indices must be capped before storage",
  );
  assert.match(
    renderBody,
    /self\s*\.filtered_registry_indices\s*\.len\(\)\s*\.min\(MAX_DISPLAYED_AGENT_REGISTRY_RESULTS\)/,
  );
  assertBefore(
    renderBody,
    ".min(MAX_DISPLAYED_AGENT_REGISTRY_RESULTS)",
    'uniform_list("registry-entries", count',
    "registry render count must be capped before list rendering",
  );
});

test("agent registry UI guard is focused on production source", () => {
  assert.equal(sourcePath, "crates/agent_ui/src/agent_registry_ui.rs");
  assert.doesNotMatch(sourcePath, /test/i);
  assert.doesNotMatch(
    source,
    /#\[cfg\(test\)\]/,
    "agent registry UI source guard should cover production code, not test-only code",
  );
});
