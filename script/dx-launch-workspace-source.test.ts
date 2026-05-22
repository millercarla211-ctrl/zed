import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX launch workspace UI stays split by rail ownership", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const expectedModules = [
    "crates/agent_ui/src/dx_launch_workspace/agents.rs",
    "crates/agent_ui/src/dx_launch_workspace/check.rs",
    "crates/agent_ui/src/dx_launch_workspace/check_labels.rs",
    "crates/agent_ui/src/dx_launch_workspace/sources.rs",
  ];

  for (const module of expectedModules) {
    assert.ok(existsSync(module), `expected focused launch workspace module ${module}`);
  }

  assert.match(parent, /^mod agents;$/m);
  assert.match(parent, /^mod check;$/m);
  assert.match(parent, /^mod check_labels;$/m);
  assert.match(parent, /^mod sources;$/m);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace.rs") < 1800,
    "dx_launch_workspace.rs should stay a coordinator instead of owning every rail",
  );
});

test("DX launch workspace delegates agents and source rails", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const agents = read("crates/agent_ui/src/dx_launch_workspace/agents.rs");
  const sources = read("crates/agent_ui/src/dx_launch_workspace/sources.rs");

  assert.match(parent, /agents::dx_agent_bridge_state/);
  assert.match(parent, /sources::source_set_stack/);
  assert.doesNotMatch(parent, /fn dx_agent_bridge_state/);
  assert.doesNotMatch(parent, /fn source_set_stack/);
  assert.match(agents, /pub\(super\) fn dx_agent_bridge_state/);
  assert.match(agents, /pub\(super\) fn dx_agent_provider_state/);
  assert.match(sources, /pub\(super\) fn source_set_stack/);
  assert.match(sources, /pub\(super\) fn receipt_source_state/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents.rs") < 1100);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/sources.rs") < 420);
});

test("DX launch workspace delegates Check rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const check = read("crates/agent_ui/src/dx_launch_workspace/check.rs");
  const labels = read("crates/agent_ui/src/dx_launch_workspace/check_labels.rs");

  assert.match(parent, /check::check_score_state/);
  assert.doesNotMatch(parent, /fn check_score_state/);
  assert.doesNotMatch(parent, /fn check_outcome_label/);
  assert.match(check, /use super::check_labels::\{/);
  assert.match(check, /pub\(super\) fn check_score_state/);
  assert.doesNotMatch(check, /fn check_outcome_label/);
  assert.doesNotMatch(check, /fn checked_paths_label/);
  assert.doesNotMatch(check, /fn skipped_checks_label/);
  assert.match(labels, /pub\(crate\) fn check_outcome_label/);
  assert.match(labels, /pub\(crate\) fn checked_paths_label/);
  assert.match(labels, /pub\(crate\) fn skipped_checks_label/);
  assert.match(labels, /pub\(crate\) fn last_run_label_with_generated_at/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/check.rs") < 190);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/check_labels.rs") < 140);
});
