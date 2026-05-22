import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX launch workspace UI stays split by rail ownership", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const expectedModules = [
    "crates/agent_ui/src/dx_launch_workspace/agents.rs",
    "crates/agent_ui/src/dx_launch_workspace/sources.rs",
  ];

  for (const module of expectedModules) {
    assert.ok(existsSync(module), `expected focused launch workspace module ${module}`);
  }

  assert.match(parent, /^mod agents;$/m);
  assert.match(parent, /^mod sources;$/m);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace.rs") < 2300,
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
