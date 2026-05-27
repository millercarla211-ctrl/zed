import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;
const normalizedPath = (path: string) => path.replaceAll("\\", "/");
const maxOwnedRustLines = 620;

const collectFiles = (root: string, extensions: Set<string>): string[] => {
  if (!existsSync(root)) return [];

  const stats = statSync(root);
  if (stats.isFile()) {
    return extensions.has(root.slice(root.lastIndexOf("."))) ? [root] : [];
  }

  return readdirSync(root, { withFileTypes: true })
    .flatMap((entry) => {
      const child = join(root, entry.name);
      if (entry.isDirectory()) return collectFiles(child, extensions);
      return extensions.has(entry.name.slice(entry.name.lastIndexOf(".")))
        ? [child]
        : [];
    })
    .sort();
};

test("DX Studio source is split into small owned modules", () => {
  const expectedModules = [
    "crates/web_preview/src/dx_studio/manifest.rs",
    "crates/web_preview/src/dx_studio/project.rs",
    "crates/web_preview/src/dx_studio/routes.rs",
    "crates/web_preview/src/dx_studio_source_edit/manifest.rs",
    "crates/web_preview/src/dx_studio_source_edit/manifest/selectors.rs",
    "crates/web_preview/src/dx_studio_source_edit/manifest/summaries.rs",
    "crates/web_preview/src/dx_studio_source_edit/manifest_ts.rs",
    "crates/web_preview/src/dx_studio_source_edit/operations.rs",
    "crates/web_preview/src/dx_studio_source_edit/paths.rs",
    "crates/web_preview/src/dx_studio_source_edit/plan.rs",
    "crates/web_preview/src/dx_studio_source_edit/receipt.rs",
    "crates/web_preview/src/dx_studio_source_edit/snapshot.rs",
    "crates/web_preview/src/dx_studio_source_edit/source_ranges.rs",
    "crates/web_preview/src/dx_studio_source_edit/values.rs",
  ];
  const rustFiles = [
    "crates/web_preview/src/dx_studio.rs",
    "crates/web_preview/src/dx_studio_bridge.rs",
    "crates/web_preview/src/dx_studio_source_edit.rs",
    ...collectFiles("crates/web_preview/src/dx_studio", new Set([".rs"])),
    ...collectFiles("crates/web_preview/src/dx_studio_source_edit", new Set([".rs"])),
  ];
  const normalizedRustFiles = rustFiles.map(normalizedPath);

  for (const module of expectedModules) {
    assert.ok(
      normalizedRustFiles.includes(module),
      `expected focused DX module ${module}`,
    );
  }

  assert.ok(rustFiles.length >= 15, "expected DX Studio to stay split by ownership");
  for (const file of rustFiles) {
    assert.ok(lineCount(file) < maxOwnedRustLines, `${file} is too large`);
  }
});

test("DX Studio bridge is assembled from focused browser-script fragments", () => {
  const bridgeSource = read("crates/web_preview/src/dx_studio_bridge.rs");
  const fragments = [
    "crates/web_preview/src/dx_studio_bridge/preamble.ts",
    "crates/web_preview/src/dx_studio_bridge/selection.ts",
    "crates/web_preview/src/dx_studio_bridge/overlay.ts",
    "crates/web_preview/src/dx_studio_bridge/capture.ts",
    "crates/web_preview/src/dx_studio_bridge/source_edit.ts",
    "crates/web_preview/src/dx_studio_bridge/api.ts",
  ];
  const discoveredFragments = collectFiles(
    "crates/web_preview/src/dx_studio_bridge",
    new Set([".ts"]),
  );

  assert.match(bridgeSource, /concat!\(/);
  assert.match(bridgeSource, /include_str!\("dx_studio_bridge\/preamble\.ts"\)/);
  assert.doesNotMatch(bridgeSource, /r#"/);
  assert.deepEqual(discoveredFragments.map(normalizedPath), [...fragments].sort());

  for (const fragment of fragments) {
    assert.ok(lineCount(fragment) < 380, `${fragment} is too large`);
  }

  const combinedScript = fragments.map((fragment) => read(fragment)).join("");
  assert.doesNotThrow(() => new Function(combinedScript));
});

test("DX Studio bridge refuses blank operation picker answers", () => {
  const capture = read("crates/web_preview/src/dx_studio_bridge/capture.ts");

  assert.match(capture, /const rawOperationAnswer = answer\.trim\(\);/);
  assert.match(
    capture,
    /if \(!rawOperationAnswer\) \{\s+restoreBridgeStateAfterPromptCancel\(\);\s+return;\s+\}/,
  );
  assert.ok(capture.includes("if (!/^\\d+$/.test(rawOperationAnswer)) {"));
  assert.match(capture, /drawSelection\(selection, "operation refused"\);/);
  assert.match(capture, /const index = Number\.parseInt\(rawOperationAnswer, 10\);/);
  assert.match(capture, /!Number\.isSafeInteger\(index\)/);
  assert.doesNotMatch(capture, /Number\.parseInt\(answer \|\| "0", 10\)/);
});

test("DX Studio bridge refuses blank target picker answers", () => {
  const selection = read("crates/web_preview/src/dx_studio_bridge/selection.ts");

  assert.match(selection, /const rawTargetAnswer = answer\.trim\(\);/);
  assert.match(
    selection,
    /if \(!rawTargetAnswer\) return null;/,
  );
  assert.ok(selection.includes("if (!/^\\d+$/.test(rawTargetAnswer)) return null;"));
  assert.match(selection, /const index = Number\.parseInt\(rawTargetAnswer, 10\);/);
  assert.match(selection, /!Number\.isSafeInteger\(index\)/);
  assert.doesNotMatch(selection, /Number\.parseInt\(answer \|\| "0", 10\)/);
});

test("DX Studio session surfaces invalid manifest candidates", () => {
  const session = read("crates/web_preview/src/dx_studio_session.rs");

  assert.match(session, /fn manifest_candidate_snapshot\(path: &Path\) -> Value/);
  assert.match(session, /"candidate_status":/);
  assert.match(session, /"read_status":/);
  assert.match(session, /"parse_status":/);
  assert.match(session, /"invalid_candidate_count":/);
  assert.match(session, /"invalid_candidates":/);
  assert.match(session, /"skipped_candidates":/);
  assert.match(session, /"malformed_json"/);
  assert.match(session, /"unreadable"/);
  assert.match(session, /"missing_edit_contract"/);
  assert.match(session, /"loaded_edit_contract"/);
});

test("DX Studio session summary can load TypeScript edit contracts", () => {
  const manifest = read("crates/web_preview/src/dx_studio/manifest.rs");
  const manifestTs = read("crates/web_preview/src/dx_studio_source_edit/manifest_ts.rs");
  const paths = read("crates/web_preview/src/dx_studio_source_edit/paths.rs");

  assert.match(manifest, /edit_contract_from_typescript/);
  assert.match(manifest, /Some\("ts" \| "tsx"\)/);
  assert.doesNotMatch(
    manifest,
    /!= Some\("json"\)\s*\{\s*continue;\s*\}/,
  );
  assert.match(
    manifestTs,
    /pub\(crate\) fn edit_contract_from_typescript\(contents: &str\) -> Option<Value>/,
  );
  assert.match(manifestTs, /fn assigned_delimiter_range\(/);
  assert.match(manifestTs, /identifier_is_non_value_declaration/);
  assert.match(manifestTs, /find_assignment_after_identifier/);
  assert.match(manifestTs, /fn parses_typed_exported_contract_arrays_after_assignment\(\)/);
  assert.match(manifestTs, /"allowGeneratedEdits"/);
  assert.match(paths, /edit_contract_from_typescript/);
  assert.match(paths, /Some\("ts" \| "tsx"\) => edit_contract_from_typescript/);
});

test("DX Studio source edits require content and selection-bound snapshots", () => {
  const root = read("crates/web_preview/src/dx_studio_source_edit.rs");
  const plan = read("crates/web_preview/src/dx_studio_source_edit/plan.rs");
  const snapshot = read("crates/web_preview/src/dx_studio_source_edit/snapshot.rs");
  const receipt = read("crates/web_preview/src/dx_studio_source_edit/receipt.rs");

  assert.match(snapshot, /"content_digest": content_digest/);
  assert.match(snapshot, /"selection_identity": selection_snapshot_identity\(selection\)/);
  assert.match(snapshot, /fn validate_expected_selection_identity\(/);
  assert.match(snapshot, /fn content_digest\(contents: &\[u8\]\) -> String/);
  assert.match(snapshot, /0xcbf29ce484222325u64/);
  assert.match(snapshot, /is_strong_selection_identity_key/);
  assert.match(root, /validate_expected_source_contents\(&source, &original, payload\)\?/);
  assert.match(plan, /source_file_snapshot\(source, selection\)/);
  assert.match(receipt, /source snapshot selection identity/);
  assert.match(receipt, /source snapshot content identity/);
});

test("DX Studio source writes stay inside bounded file and edit sizes", () => {
  const root = read("crates/web_preview/src/dx_studio_source_edit.rs");

  assert.match(root, /const DX_STUDIO_MAX_SOURCE_FILE_BYTES: u64 = 2_000_000;/);
  assert.match(root, /const DX_STUDIO_MAX_SOURCE_EDIT_DELTA_BYTES: i64 = 200_000;/);
  assert.match(root, /ensure_source_file_size_allows_edit\(&source, metadata\.len\(\)\)\?/);
  assert.match(root, /ensure_source_write_bounds\(&source, edit\.updated\.len\(\), edit\.changed_bytes\)\?/);
});
