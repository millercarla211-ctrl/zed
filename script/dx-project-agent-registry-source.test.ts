import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const sliceBetween = (source: string, startNeedle: string, endNeedle: string) => {
  const start = source.indexOf(startNeedle);
  assert.notEqual(start, -1, `missing ${startNeedle}`);
  const end = source.indexOf(endNeedle, start + startNeedle.length);
  assert.notEqual(end, -1, `missing ${endNeedle}`);
  return source.slice(start, end);
};

const assertOrdered = (
  body: string,
  firstNeedle: string,
  secondNeedle: string,
  message: string,
) => {
  const first = body.indexOf(firstNeedle);
  const second = body.indexOf(secondNeedle);

  assert.ok(first >= 0, `${message}: missing ${firstNeedle}`);
  assert.ok(second >= 0, `${message}: missing ${secondNeedle}`);
  assert.ok(second > first, message);
};

test("Project agent registry HTTP response bodies are sentinel-byte capped", () => {
  const source = read("crates/project/src/agent_registry_store.rs");
  const helper = sliceBetween(
    source,
    "async fn fetch_url_body(",
    "fn resolve_icon_url(",
  );

  assert.match(
    source,
    /const MAX_REGISTRY_RESPONSE_BODY_BYTES: u64 = \d+ \* 1024 \* 1024;/,
  );
  assert.match(
    helper,
    /\.take\(MAX_REGISTRY_RESPONSE_BODY_BYTES \+ 1\)\s+\.read_to_end\(&mut body\)\s+\.await/,
  );
  assert.match(
    helper,
    /body\.len\(\) as u64 > MAX_REGISTRY_RESPONSE_BODY_BYTES/,
  );
  assertOrdered(
    helper,
    ".read_to_end(&mut body)",
    "body.len() as u64 > MAX_REGISTRY_RESPONSE_BODY_BYTES",
    "registry response bodies must be checked after the sentinel read",
  );
  assertOrdered(
    helper,
    "body.len() as u64 > MAX_REGISTRY_RESPONSE_BODY_BYTES",
    "Ok((status, body))",
    "oversized registry response bodies must be rejected before materialization",
  );
  assert.doesNotMatch(
    helper,
    /body_mut\(\)\s*\.read_to_end\(&mut body\)/,
    "fetch_url_body must not directly read unbounded response bodies",
  );
  assert.doesNotMatch(helper, /read_to_string/);
});

test("Project agent registry caps error text before display", () => {
  const source = read("crates/project/src/agent_registry_store.rs");
  const displayHelper = sliceBetween(
    source,
    "fn format_registry_response_error_body(",
    "async fn fetch_url_body(",
  );
  const registryFetch = sliceBetween(
    source,
    "async fn fetch_registry_index(",
    "async fn build_registry_agents(",
  );
  const iconFetch = sliceBetween(
    source,
    "async fn download_icon(",
    "fn format_registry_response_error_body(",
  );

  assert.match(
    source,
    /const MAX_REGISTRY_RESPONSE_ERROR_DISPLAY_BYTES: usize = \d+ \* 1024;/,
  );
  assert.match(
    displayHelper,
    /body\.len\(\)\.min\(MAX_REGISTRY_RESPONSE_ERROR_DISPLAY_BYTES\)/,
  );
  assert.match(displayHelper, /String::from_utf8_lossy\(visible_body\)\.into_owned\(\)/);
  assert.match(displayHelper, /\[truncated\]/);
  assert.doesNotMatch(registryFetch, /String::from_utf8_lossy/);
  assert.doesNotMatch(iconFetch, /String::from_utf8_lossy/);
  assert.match(registryFetch, /format_registry_response_error_body\(&body\)/);
  assert.match(iconFetch, /format_registry_response_error_body\(&body\)/);
});

test("Project agent registry bounds response bytes before JSON parse and cache payload use", () => {
  const source = read("crates/project/src/agent_registry_store.rs");
  const registryFetch = sliceBetween(
    source,
    "async fn fetch_registry_index(",
    "async fn build_registry_agents(",
  );

  assertOrdered(
    registryFetch,
    "fetch_url_body(http_client, REGISTRY_URL, REGISTRY_FETCH_TIMEOUT, executor)",
    "status.is_client_error()",
    "registry status handling must happen after bounded response body reads",
  );
  assertOrdered(
    registryFetch,
    "fetch_url_body(http_client, REGISTRY_URL, REGISTRY_FETCH_TIMEOUT, executor)",
    "serde_json::from_slice(&body)",
    "registry JSON parsing must happen after bounded response body reads",
  );
  assertOrdered(
    registryFetch,
    "serde_json::from_slice(&body)",
    "raw_body: body",
    "registry raw payload should only be materialized after bounded parse input",
  );
});

test("Project agent registry caps entries before icon fanout and agent build", () => {
  const source = read("crates/project/src/agent_registry_store.rs");
  const buildAgents = sliceBetween(
    source,
    "async fn build_registry_agents(",
    "async fn resolve_icon_paths(",
  );
  const iconResolver = sliceBetween(
    source,
    "async fn resolve_icon_paths(",
    "async fn resolve_icon_path(",
  );

  assert.match(source, /const MAX_REGISTRY_INDEX_AGENTS: usize = \d+;/);
  assert.match(source, /const MAX_REGISTRY_ICON_FETCHES: usize = \d+;/);
  assert.match(
    source,
    /fn capped_registry_entries\(entries: Vec<RegistryEntry>\) -> Vec<RegistryEntry>[\s\S]+\.take\(MAX_REGISTRY_INDEX_AGENTS\)/,
  );
  assertOrdered(
    buildAgents,
    "let registry_entries = capped_registry_entries(index.agents);",
    "resolve_icon_paths(",
    "registry entries must be capped before icon path resolution",
  );
  assertOrdered(
    buildAgents,
    "let registry_entries = capped_registry_entries(index.agents);",
    "let mut agents = Vec::with_capacity(registry_entries.len());",
    "registry entries must be capped before RegistryAgent vector materialization",
  );
  assert.doesNotMatch(
    buildAgents,
    /resolve_icon_paths\(\s*&index\.agents/,
    "icon resolution must not fan out over the raw registry index",
  );
  assert.doesNotMatch(
    buildAgents,
    /index\.agents\.into_iter\(\)/,
    "agent build loop must not iterate the raw registry index",
  );
  assert.match(
    iconResolver,
    /entries\.iter\(\)\.take\(MAX_REGISTRY_ICON_FETCHES\)\.map\(\|entry\|/,
  );
  assert.match(iconResolver, /icon_paths\.resize\(entries\.len\(\), None\)/);
  assertOrdered(
    iconResolver,
    ".take(MAX_REGISTRY_ICON_FETCHES)",
    "resolve_icon_path(entry",
    "icon fetch fanout must be capped before spawning per-entry futures",
  );
});

test("Project agent registry caps binary targets before insertion", () => {
  const source = read("crates/project/src/agent_registry_store.rs");
  const buildAgents = sliceBetween(
    source,
    "async fn build_registry_agents(",
    "async fn resolve_icon_paths(",
  );

  assert.match(source, /const MAX_REGISTRY_BINARY_TARGETS: usize = \d+;/);
  assert.match(
    buildAgents,
    /current_platform_target\s+\.into_iter\(\)\s+\.chain\(/,
  );
  assert.match(
    buildAgents,
    /\.take\(MAX_REGISTRY_BINARY_TARGETS\)\s+\{\s+targets\.insert\(/,
  );
  assertOrdered(
    buildAgents,
    ".take(MAX_REGISTRY_BINARY_TARGETS)",
    "targets.insert(",
    "binary target maps must be capped before target insertion",
  );
  assert.doesNotMatch(
    buildAgents,
    /for \(platform, target\) in binary\.iter\(\) \{\s+targets\.insert\(/,
    "binary target insertion must not loop over the unbounded distribution map",
  );
});
