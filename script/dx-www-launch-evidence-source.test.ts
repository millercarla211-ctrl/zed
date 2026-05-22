import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX-WWW launch evidence keeps artifact catalog out of scanner", () => {
  const parentPath = "crates/agent_ui/src/dx_www_launch_evidence.rs";
  const catalogPath = "crates/agent_ui/src/dx_www_launch_evidence/expected_artifacts.rs";
  const labelsPath = "crates/agent_ui/src/dx_www_launch_evidence/evidence_labels.rs";
  const statusPath = "crates/agent_ui/src/dx_www_launch_evidence/evidence_status.rs";
  assert.ok(existsSync(catalogPath), "expected focused DX-WWW launch evidence catalog module");
  assert.ok(existsSync(labelsPath), "expected focused DX-WWW launch evidence label module");
  assert.ok(existsSync(statusPath), "expected focused DX-WWW launch evidence status module");

  const parent = read(parentPath);
  const catalog = read(catalogPath);
  const labels = read(labelsPath);
  const status = read(statusPath);

  assert.match(parent, /^mod evidence_labels;$/m);
  assert.match(parent, /^mod evidence_status;$/m);
  assert.match(parent, /^mod expected_artifacts;$/m);
  assert.match(parent, /use evidence_labels::evidence_score_label/);
  assert.match(parent, /use evidence_status::evidence_status_label/);
  assert.match(parent, /use expected_artifacts::\{/);
  assert.doesNotMatch(parent, /const EXPECTED_EVIDENCE_ARTIFACTS/);
  assert.doesNotMatch(parent, /struct ExpectedWwwEvidenceArtifact/);
  assert.doesNotMatch(parent, /enum EvidenceFormat/);
  assert.doesNotMatch(parent, /format!\("\{score\}\/100"\)/);
  assert.doesNotMatch(parent, /let status = match passed/);
  assert.match(parent, /pub\(crate\) fn www_launch_evidence_snapshot/);
  assert.match(parent, /fn scan_www_launch_evidence/);
  assert.match(parent, /fn inspect_expected_artifact/);
  assert.ok(lineCount(parentPath) < 430, "scanner should stay below the catalog-free line budget");

  assert.match(catalog, /pub\(super\) struct ExpectedWwwEvidenceArtifact/);
  assert.match(catalog, /pub\(super\) enum EvidenceFormat/);
  assert.match(catalog, /pub\(super\) const EXPECTED_EVIDENCE_ARTIFACTS/);
  assert.match(catalog, /launch-evidence-friday-baton\.md/);
  assert.match(catalog, /launch-evidence-acceptance-digest\.json/);
  assert.ok(lineCount(catalogPath) < 260, "artifact catalog should remain compact data ownership");

  assert.match(labels, /pub\(crate\) fn evidence_score_label/);
  assert.match(labels, /score_label_rejects_scores_above_100/);
  assert.match(labels, /score_label_trims_blank_schema/);
  assert.ok(lineCount(labelsPath) < 80, "evidence labels should stay small and pure");

  assert.match(status, /pub\(crate\) fn evidence_status_label/);
  assert.match(status, /status_label_blocks_failed_packets/);
  assert.match(status, /status_label_warns_when_passed_packet_has_findings/);
  assert.match(status, /status_label_warns_when_findings_exist_without_passed_flag/);
  assert.match(status, /status_label_defaults_ready_for_empty_metadata/);
  assert.ok(lineCount(statusPath) < 90, "evidence status labels should stay small and pure");
});
