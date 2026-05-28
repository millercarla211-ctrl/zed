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
