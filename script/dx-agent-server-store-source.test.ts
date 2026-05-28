import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/project/src/agent_server_store.rs";
const source = readFileSync(sourcePath, "utf8");

function sliceBetween(startNeedle: string, endNeedle: string): string {
  const start = source.indexOf(startNeedle);
  assert.notEqual(start, -1, `expected ${startNeedle}`);
  const end = source.indexOf(endNeedle, start);
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

test("agent server store bounds discovered server and archive-cache candidates before processing", () => {
  assert.match(
    source,
    /const MAX_AGENT_SERVER_REGISTRY_DISCOVERY_ENTRIES: usize = 1024;/,
  );
  assert.match(
    source,
    /const MAX_AGENT_SERVER_SETTINGS_DISCOVERY_ENTRIES: usize = 512;/,
  );
  assert.match(
    source,
    /const MAX_AGENT_SERVER_REMOTE_DISCOVERY_ENTRIES: usize = 512;/,
  );
  assert.match(
    source,
    /const MAX_VERSIONED_ARCHIVE_CACHE_SCAN_ENTRIES: usize = 512;/,
  );

  const registryCandidates = sliceBetween(
    "let registry_agents_by_id = registry_store",
    "// Drain the existing versioned agents",
  );
  assertBefore(
    registryCandidates,
    ".take(MAX_AGENT_SERVER_REGISTRY_DISCOVERY_ENTRIES)",
    ".collect::<HashMap<_, _>>()",
    "registry candidates must be capped before collecting the lookup map",
  );

  const settingsCandidates = sliceBetween(
    "for (name, settings) in new_settings",
    "// For each rebuilt versioned agent",
  );
  assertBefore(
    settingsCandidates,
    ".take(MAX_AGENT_SERVER_SETTINGS_DISCOVERY_ENTRIES)",
    "match settings",
    "settings candidates must be capped before registering agents",
  );

  const settingsImport = sliceBetween(
    "impl settings::Settings for AllAgentServersSettings",
    "#[cfg(test)]",
  );
  assertBefore(
    settingsImport,
    ".take(MAX_AGENT_SERVER_SETTINGS_DISCOVERY_ENTRIES)",
    ".collect(),",
    "settings-file agent server entries must be capped before collecting",
  );

  const remoteCandidates = sliceBetween(
    "this.external_agents = envelope",
    "cx.emit(AgentServersUpdated);",
  );
  assertBefore(
    remoteCandidates,
    ".take(MAX_AGENT_SERVER_REMOTE_DISCOVERY_ENTRIES)",
    ".collect();",
    "remote server candidates must be capped before collecting entries",
  );

  const archiveCleanup = sliceBetween(
    "async fn remove_stale_versioned_archive_cache_dirs",
    "struct LocalRegistryArchiveAgent",
  );
  assertBefore(
    archiveCleanup,
    "while scanned_entries < MAX_VERSIONED_ARCHIVE_CACHE_SCAN_ENTRIES",
    "let Some(entry) = entries.next().await",
    "archive cache scans must stop before reading beyond the cap",
  );
  assertBefore(
    archiveCleanup,
    "scanned_entries += 1;",
    "fs.metadata(&entry)",
    "archive cache entries must be counted before metadata processing",
  );

  const archiveTestCollection = sliceBetween(
    "let mut remaining = fs",
    "remaining.sort();",
  );
  assertBefore(
    archiveTestCollection,
    ".take(MAX_VERSIONED_ARCHIVE_CACHE_SCAN_ENTRIES)",
    ".collect::<Vec<_>>()",
    "base_dir read_dir test collection must be capped before collecting and sorting",
  );
});
