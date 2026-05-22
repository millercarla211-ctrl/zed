import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX Agent bridge stays split by command, runtime, and receipt ownership", () => {
  const parent = read("crates/agent_ui/src/dx_agent_bridge.rs");
  const expectedModules = [
    "crates/agent_ui/src/dx_agent_bridge/command_safety.rs",
    "crates/agent_ui/src/dx_agent_bridge/command_safety_tests.rs",
    "crates/agent_ui/src/dx_agent_bridge/commands.rs",
    "crates/agent_ui/src/dx_agent_bridge/local_files.rs",
    "crates/agent_ui/src/dx_agent_bridge/receipts.rs",
    "crates/agent_ui/src/dx_agent_bridge/runtime.rs",
  ];

  for (const module of expectedModules) {
    assert.ok(existsSync(module), `expected focused DX Agent bridge module ${module}`);
  }

  assert.match(parent, /^mod command_safety;$/m);
  assert.match(parent, /^mod commands;$/m);
  assert.match(parent, /^mod local_files;$/m);
  assert.match(parent, /^mod receipts;$/m);
  assert.match(parent, /^mod runtime;$/m);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_agent_bridge.rs") < 820,
    "dx_agent_bridge.rs should stay a coordinator and type boundary",
  );
});

test("DX Agent bridge delegates bridge commands and receipt parsing", () => {
  const parent = read("crates/agent_ui/src/dx_agent_bridge.rs");
  const safety = read("crates/agent_ui/src/dx_agent_bridge/command_safety.rs");
  const safetyTests = read("crates/agent_ui/src/dx_agent_bridge/command_safety_tests.rs");
  const commands = read("crates/agent_ui/src/dx_agent_bridge/commands.rs");
  const localFiles = read("crates/agent_ui/src/dx_agent_bridge/local_files.rs");
  const receipts = read("crates/agent_ui/src/dx_agent_bridge/receipts.rs");
  const runtime = read("crates/agent_ui/src/dx_agent_bridge/runtime.rs");

  assert.doesNotMatch(parent, /fn run_bridge_command/);
  assert.doesNotMatch(parent, /fn contract_summary/);
  assert.doesNotMatch(parent, /fn social_accounts/);
  assert.doesNotMatch(parent, /fn is_secret_like_arg/);
  assert.doesNotMatch(parent, /fn public_command_for_runtime/);
  assert.match(parent, /use self::command_safety::\{/);
  assert.match(safety, /pub\(crate\) fn is_secret_like_arg/);
  assert.match(safety, /pub\(crate\) fn redact_action_scalar/);
  assert.match(safety, /pub\(crate\) fn public_command_for_runtime/);
  assert.match(safety, /pub\(crate\) fn is_safe_platform_arg/);
  assert.match(safety, /pub\(crate\) fn bridge_command_label/);
  assert.match(safety, /#\[path = "command_safety_tests\.rs"\]/);
  assert.match(safety, /normalized\.contains\(marker\)/);
  assert.match(safety, /let mut redact_next = false/);
  assert.match(safety, /redact_next = is_secret_flag_arg\(arg\)/);
  assert.match(safety, /fn is_secret_flag_arg/);
  assert.match(safetyTests, /dx_agent_secret_marker_guard_covers_bridge_receipt_scalars/);
  assert.match(safetyTests, /public_command_for_runtime_maps_legacy_dx_agents_commands/);
  assert.match(safetyTests, /bridge_command_label_redacts_secret_like_args/);
  assert.match(safetyTests, /bridge_command_label_redacts_secret_key_value_args/);
  assert.match(commands, /pub\(crate\) fn run_dx_agent_public_command/);
  assert.match(commands, /pub\(crate\) enum DxAgentPublicCommand/);
  assert.match(localFiles, /pub\(super\) fn read_json/);
  assert.match(localFiles, /pub\(super\) fn read_first_json/);
  assert.match(localFiles, /pub\(super\) fn latest_receipts/);
  assert.match(localFiles, /pub\(super\) fn dx_home_from_receipt_root/);
  assert.match(localFiles, /MAX_RECEIPT_BYTES/);
  assert.match(receipts, /pub\(super\) fn contract_summary/);
  assert.match(receipts, /pub\(super\) fn receipt_index_summary/);
  assert.match(runtime, /pub\(super\) fn social_accounts/);
  assert.match(runtime, /pub\(super\) fn catalog_summary/);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/command_safety.rs") < 120);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/command_safety_tests.rs") < 130);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/commands.rs") < 330);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/local_files.rs") < 110);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/receipts.rs") < 560);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/runtime.rs") < 420);
});
