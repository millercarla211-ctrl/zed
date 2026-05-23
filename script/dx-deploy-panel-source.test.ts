import test from "node:test";
import assert from "node:assert/strict";

import {
  deployGuardFiles,
  deploySourceDir,
  deploySourceFiles,
  lineCount,
  read,
} from "./dx-deploy-source-guard.ts";

test("agent_ui registers the focused deploy modules", () => {
  const source = read("crates/agent_ui/src/agent_ui.rs");
  const modules = [
    "dx_deploy_capabilities",
    "dx_deploy_check_roots",
    "dx_deploy_gate_rail",
    "dx_deploy_hub_roots",
    "dx_deploy_launch_actions",
    "dx_deploy_launch_action_labels",
    "dx_deploy_launch_actions_rail",
    "dx_deploy_launch_approval_evidence",
    "dx_deploy_launch_buckets",
    "dx_deploy_launch_evidence",
    "dx_deploy_launch_evidence_rail",
    "dx_deploy_launch_gate",
    "dx_deploy_launch_gate_rail",
    "dx_deploy_launch_notices",
    "dx_deploy_launch_outcome",
    "dx_deploy_launch_prompts",
    "dx_deploy_launch_score",
    "dx_deploy_launch_scope",
    "dx_deploy_local_files",
    "dx_deploy_matrix_rail",
    "dx_deploy_provider_gate_summary",
    "dx_deploy_prompts",
    "dx_deploy_rail",
    "dx_deploy_rail_ui",
    "dx_deploy_receipt_buckets",
    "dx_deploy_receipt_extract",
    "dx_deploy_receipt_fields",
    "dx_deploy_receipt_files",
    "dx_deploy_receipt_rank",
    "dx_deploy_receipt_roots",
    "dx_deploy_receipt_summary",
    "dx_deploy_root_key",
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
  const source = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(source, /source\/runtime\/launch approval/);
  assert.match(source, /no live deploy should be inferred from dry-run receipts/);
  assert.match(source, /dry-run receipts are not live deploy approval/);
  assert.match(source, /fn deploy_launch_gate_prompt\(/);
});

test("deploy launch prompt details stay in a focused module", () => {
  const prompts = read("crates/agent_ui/src/dx_deploy_prompts.rs");
  const launchPrompts = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(
    prompts,
    /pub\(crate\) use crate::dx_deploy_launch_prompts::deploy_launch_gate_prompt;/,
  );
  assert.match(
    prompts,
    /use crate::dx_deploy_launch_prompts::deploy_launch_evidence_prompt;/,
  );
  assert.match(launchPrompts, /pub\(crate\) fn deploy_launch_gate_prompt/);
  assert.match(launchPrompts, /pub\(crate\) fn deploy_launch_evidence_prompt/);
  assert.match(launchPrompts, /fn launch_actions_prompt/);
  assert.match(launchPrompts, /fn approval_state_label/);
  assert.match(launchPrompts, /status_score=/);
  assert.match(launchPrompts, /launch_actions=/);
  assert.doesNotMatch(prompts, /fn\s+deploy_launch_gate_prompt/);
  assert.doesNotMatch(prompts, /fn\s+launch_actions_prompt/);
  assert.doesNotMatch(prompts, /fn\s+launch_evidence_prompt/);
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
    const count = lineCount(`${deploySourceDir}/${fileName}`);
    assert.ok(count < 300, `${fileName} has ${count} lines`);
  }
});

test("deploy source guard tests stay split by ownership", () => {
  const guardFiles = deployGuardFiles();

  for (const fileName of [
    "dx-deploy-receipts-source.test.ts",
    "dx-deploy-launch-gate-source.test.ts",
    "dx-deploy-launch-evidence-source.test.ts",
  ]) {
    assert.ok(
      guardFiles.includes(fileName),
      `${fileName} should own its focused deploy source checks`,
    );
  }

  for (const fileName of guardFiles) {
    const count = lineCount(`script/${fileName}`);
    assert.ok(count < 260, `${fileName} has ${count} lines`);
  }
});

test("deploy status docs name the repeatable source guard", () => {
  const docs = [read("DX.md"), read("todo.txt"), read("changelog.txt")].join("\n");

  assert.match(docs, /DX Deploy panel source guard/);
  assert.match(docs, /DX Deploy launch evidence-source rows/);
  assert.match(docs, /five-source launch evidence/);
  assert.match(docs, /stable evidence id/);
  assert.match(docs, /receipt freshness/);
  assert.match(docs, /100-point deploy status/);
  assert.match(docs, /49\/100/);
  assert.match(docs, /49\/100 estimated/);
  assert.match(docs, /launch quick actions/);
  assert.match(docs, /quick-action identities/);
  assert.match(docs, /source blocker evidence paths/);
  assert.match(docs, /warning evidence paths/);
  assert.match(docs, /evidence-source blocker text/);
  assert.match(docs, /launch outcome counts/);
  assert.match(docs, /skipped expensive checks/);
  assert.match(docs, /launch scope and checked paths/);
  assert.match(docs, /scoring profile/);
  assert.match(docs, /bucket-score breakdown/);
  assert.match(docs, /launch buckets/);
  assert.match(docs, /approval evidence/);
  assert.match(docs, /source runtime and launch evidence/);
  assert.match(docs, /deploy receipt roots/);
  assert.match(docs, /dx_deploy_hub_roots\.rs/);
  assert.match(docs, /dx_deploy_check_roots\.rs/);
  assert.match(docs, /launch-check receipt roots/);
  assert.match(docs, /DX_HOME/);
  assert.match(docs, /D:\\Dx/);
  assert.match(docs, /workspace plus DX hub\/cli\/www receipt roots/);
  assert.match(docs, /receipt-write/);
  assert.match(docs, /script\/dx-deploy-panel-source\.test\.ts/);
  assert.match(docs, /script\/dx-deploy-receipts-source\.test\.ts/);
  assert.match(docs, /script\/dx-deploy-launch-gate-source\.test\.ts/);
  assert.match(docs, /script\/dx-deploy-launch-evidence-source\.test\.ts/);
});
