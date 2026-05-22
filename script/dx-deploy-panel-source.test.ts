import { readFileSync, readdirSync } from "node:fs";
import test from "node:test";
import assert from "node:assert/strict";

const deploySourceDir = "crates/agent_ui/src";
const read = (path) => readFileSync(path, "utf8");

const deploySourceFiles = () =>
  readdirSync(deploySourceDir)
    .filter((name) => name.startsWith("dx_deploy") && name.endsWith(".rs"))
    .sort();

test("agent_ui registers the focused deploy modules", () => {
  const source = read("crates/agent_ui/src/agent_ui.rs");
  const modules = [
    "dx_deploy_capabilities",
    "dx_deploy_gate_rail",
    "dx_deploy_launch_evidence",
    "dx_deploy_launch_evidence_rail",
    "dx_deploy_launch_gate",
    "dx_deploy_launch_gate_rail",
    "dx_deploy_local_files",
    "dx_deploy_matrix_rail",
    "dx_deploy_prompts",
    "dx_deploy_rail",
    "dx_deploy_rail_ui",
    "dx_deploy_receipt_buckets",
    "dx_deploy_receipt_extract",
    "dx_deploy_receipt_files",
    "dx_deploy_receipt_rank",
    "dx_deploy_receipt_summary",
    "dx_deploy_target_detection",
    "dx_deploy_targets",
  ];

  for (const moduleName of modules) {
    assert.match(
      source,
      new RegExp(`^mod ${moduleName};$`, "m"),
      `${moduleName} should be registered from agent_ui.rs`,
    );
  }
});

test("deploy prompt ownership stays out of the launch prompt module", () => {
  const agentPanel = read("crates/agent_ui/src/agent_panel.rs");
  const launchPrompts = read("crates/agent_ui/src/dx_launch_prompts.rs");

  assert.match(
    agentPanel,
    /use crate::dx_deploy_prompts::deploy_readiness_prompt;/,
  );
  assert.match(
    launchPrompts,
    /use crate::dx_deploy_prompts::\{deploy_launch_gate_prompt, deploy_receipt_bucket_prompt\};/,
  );
  assert.doesNotMatch(launchPrompts, /fn\s+deploy_readiness_prompt/);
  assert.doesNotMatch(launchPrompts, /fn\s+deploy_capability_matrix_prompt/);
});

test("deploy prompts keep dry-run and launch approval language explicit", () => {
  const source = read("crates/agent_ui/src/dx_deploy_prompts.rs");

  assert.match(source, /source\/runtime\/launch approval/);
  assert.match(source, /no live deploy should be inferred from dry-run receipts/);
  assert.match(source, /dry-run receipts are not live deploy approval/);
  assert.match(source, /fn deploy_launch_gate_prompt\(/);
});

test("launch gate reader prefers launch-specific check receipts", () => {
  const source = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");

  assert.match(source, /DX_HUB_CHECK_RECEIPT_ROOT/);
  assert.match(source, /DX_WWW_CHECK_RECEIPT_ROOT/);
  assert.match(source, /\["check-launch-latest\.json", "check-latest\.json"\]/);
  assert.match(source, /file_rank/);
  assert.match(source, /root_rank/);
});

test("launch gate exposes dx-check evidence-source rows", () => {
  const parser = read("crates/agent_ui/src/dx_deploy_launch_evidence.rs");
  const reader = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");
  const rail = read("crates/agent_ui/src/dx_deploy_launch_gate_rail.rs");
  const evidenceRail = read("crates/agent_ui/src/dx_deploy_launch_evidence_rail.rs");
  const prompts = read("crates/agent_ui/src/dx_deploy_prompts.rs");

  assert.match(parser, /DxDeployLaunchEvidenceSource/);
  assert.match(parser, /DxDeployLaunchChain/);
  assert.match(parser, /launch_evidence_sources/);
  assert.match(parser, /launch_chain/);
  assert.match(parser, /required_source_count/);
  assert.match(parser, /blocker_count/);
  assert.match(reader, /launch_evidence_sources\(&receipt\)/);
  assert.match(reader, /launch_chain\(&receipt\)/);
  assert.match(rail, /deploy_launch_evidence_state/);
  assert.match(evidenceRail, /launch_evidence_summary/);
  assert.match(evidenceRail, /launch_evidence_row/);
  assert.match(evidenceRail, /launch_chain_summary/);
  assert.match(prompts, /launch_evidence_prompt/);
  assert.match(prompts, /evidence_sources=/);
});

test("deploy rail renders launch approval before provider dry-run rows", () => {
  const source = read("crates/agent_ui/src/dx_deploy_rail.rs");
  const launchGateIndex = source.indexOf(
    ".child(deploy_launch_gate_state(&snapshot.launch_gate, cx))",
  );
  const matrixIndex = source.indexOf(".child(deploy_capability_matrix_state(");

  assert.notEqual(launchGateIndex, -1);
  assert.notEqual(matrixIndex, -1);
  assert.ok(
    launchGateIndex < matrixIndex,
    "launch approval should be visible before plan/status/provider rows",
  );
});

test("deploy source files stay small enough for maintenance", () => {
  const files = deploySourceFiles();

  assert.ok(files.length >= 12, "expected the deploy lane to stay split by ownership");
  for (const fileName of files) {
    const lineCount = read(`${deploySourceDir}/${fileName}`).split(/\r?\n/).length;
    assert.ok(lineCount < 300, `${fileName} has ${lineCount} lines`);
  }
});

test("deploy status docs name the repeatable source guard", () => {
  const docs = [read("DX.md"), read("todo.txt"), read("changelog.txt")].join("\n");

  assert.match(docs, /DX Deploy panel source guard/);
  assert.match(docs, /DX Deploy launch evidence-source rows/);
  assert.match(docs, /script\/dx-deploy-panel-source\.test\.ts/);
});
