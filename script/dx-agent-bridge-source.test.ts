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
    "crates/agent_ui/src/dx_agent_bridge/local_file_labels.rs",
    "crates/agent_ui/src/dx_agent_bridge/local_files.rs",
    "crates/agent_ui/src/dx_agent_bridge/receipts.rs",
    "crates/agent_ui/src/dx_agent_bridge/receipts/receipt_strings.rs",
    "crates/agent_ui/src/dx_agent_bridge/runtime.rs",
  ];

  for (const module of expectedModules) {
    assert.ok(existsSync(module), `expected focused DX Agent bridge module ${module}`);
  }

  assert.match(parent, /^mod command_safety;$/m);
  assert.match(parent, /^mod commands;$/m);
  assert.match(parent, /^mod local_file_labels;$/m);
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
  const localFileLabels = read("crates/agent_ui/src/dx_agent_bridge/local_file_labels.rs");
  const localFiles = read("crates/agent_ui/src/dx_agent_bridge/local_files.rs");
  const receipts = read("crates/agent_ui/src/dx_agent_bridge/receipts.rs");
  const receiptStrings = read("crates/agent_ui/src/dx_agent_bridge/receipts/receipt_strings.rs");
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
  assert.match(localFiles, /receipt_file_label/);
  assert.match(localFiles, /MAX_RECEIPT_BYTES/);
  assert.match(localFileLabels, /pub\(crate\) fn receipt_file_label/);
  assert.match(localFileLabels, /eq_ignore_ascii_case\("json"\)/);
  assert.match(localFileLabels, /receipt_file_label_accepts_uppercase_json_extension/);
  assert.match(receipts, /pub\(super\) fn contract_summary/);
  assert.match(receipts, /pub\(super\) fn receipt_index_summary/);
  assert.match(receipts, /^mod receipt_strings;$/m);
  assert.match(receipts, /use self::receipt_strings::\{/);
  assert.match(receiptStrings, /pub\(super\) fn receipt_string_field/);
  assert.match(receiptStrings, /pub\(super\) fn receipt_string_array_field/);
  assert.match(receiptStrings, /pub\(super\) fn receipt_string_values_field/);
  assert.match(runtime, /pub\(super\) fn social_accounts/);
  assert.match(runtime, /pub\(super\) fn catalog_summary/);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/command_safety.rs") < 120);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/command_safety_tests.rs") < 130);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/commands.rs") < 330);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/local_file_labels.rs") < 110);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/local_files.rs") < 110);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/receipts.rs") < 560);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/receipts/receipt_strings.rs") < 75);
  assert.ok(lineCount("crates/agent_ui/src/dx_agent_bridge/runtime.rs") < 420);
});

test("DX Agent bridge local receipt reads reject post-metadata growth before parsing", () => {
  const localFiles = read("crates/agent_ui/src/dx_agent_bridge/local_files.rs");
  const readJsonStart = localFiles.indexOf("pub(super) fn read_json");
  const readJsonEnd = localFiles.indexOf("\npub(super) fn read_first_json");

  assert.ok(readJsonStart >= 0, "expected local read_json helper");
  assert.ok(readJsonEnd > readJsonStart, "expected read_json to stay focused");

  const readJson = localFiles.slice(readJsonStart, readJsonEnd);
  const growthLimitCheck =
    "u64::try_from(source.len()).unwrap_or(u64::MAX) > MAX_RECEIPT_BYTES";

  assert.match(readJson, /take\(MAX_RECEIPT_BYTES \+ 1\)/);
  assert.match(readJson, /read_to_end\(&mut source\)/);
  assert.match(readJson, new RegExp(growthLimitCheck.replace(/[().+]/g, "\\$&")));
  assert.match(readJson, /serde_json::from_slice\(&source\)/);
  assert.doesNotMatch(readJson, /read_to_string/);
  assert.ok(
    readJson.indexOf(growthLimitCheck) < readJson.indexOf("serde_json::from_slice"),
    "receipt buffers must be rejected over MAX_RECEIPT_BYTES before parsing",
  );
});

test("DX Agent bridge failed command stderr is compacted before error display", () => {
  const commands = read("crates/agent_ui/src/dx_agent_bridge/commands.rs");
  const runStart = commands.indexOf("fn run_bridge_command");
  const runEnd = commands.indexOf("\nfn write_json_receipt");
  const helperStart = commands.indexOf("fn failed_command_stderr_display");
  const helperEnd = commands.indexOf("\nfn write_json_receipt");

  assert.ok(runStart >= 0, "expected run_bridge_command helper");
  assert.ok(runEnd > runStart, "expected run_bridge_command to stay before receipt writer");
  assert.ok(helperStart > runStart, "expected focused failed-command stderr display helper");
  assert.ok(helperEnd > helperStart, "expected helper before receipt writer");

  const runBridgeCommand = commands.slice(runStart, runEnd);
  const stderrHelper = commands.slice(helperStart, helperEnd);

  assert.match(commands, /const MAX_FAILED_COMMAND_STDERR_BYTES: usize = 2048;/);
  assert.match(commands, /const MAX_FAILED_COMMAND_STDERR_CHARS: usize = 500;/);
  assert.match(runBridgeCommand, /is_secret_like_arg\(arg\)/);
  assert.match(
    runBridgeCommand,
    /let stderr = failed_command_stderr_display\(&output\.stderr\);/,
  );
  assert.match(runBridgeCommand, /anyhow!\(\s*"`\{\}` failed: \{\}"/);
  const stderrDisplayCall = runBridgeCommand.indexOf(
    "failed_command_stderr_display(&output.stderr)",
  );
  const failedCommandAnyhow = runBridgeCommand.indexOf("`{}` failed: {}");
  assert.ok(failedCommandAnyhow > stderrDisplayCall, "expected failed-command anyhow");
  assert.ok(
    stderrDisplayCall < failedCommandAnyhow,
    "stderr must be compacted before inclusion in anyhow",
  );
  assert.doesNotMatch(commands, /String::from_utf8_lossy\(&output\.stderr\)/);
  assert.match(stderrHelper, /stderr\.len\(\) > MAX_FAILED_COMMAND_STDERR_BYTES/);
  assert.match(stderrHelper, /&stderr\[..visible_len\]/);
  assert.match(stderrHelper, /String::from_utf8_lossy\(&stderr\[..visible_len\]\)/);
  assert.match(stderrHelper, /split_whitespace\(\)\.collect::<Vec<_>>\(\)\.join\(" "\)/);
  assert.match(stderrHelper, /take\(MAX_FAILED_COMMAND_STDERR_CHARS\.saturating_sub\(3\)\)/);
  assert.match(stderrHelper, /display\.push_str\("\.\.\."\)/);
});

test("DX Agent bridge checks serialized receipt bytes before writing", () => {
  const commands = read("crates/agent_ui/src/dx_agent_bridge/commands.rs");
  const writeJsonStart = commands.indexOf("fn write_json_receipt");
  const writeActionErrorStart = commands.indexOf("fn write_action_error_receipt");
  const clearActionErrorStart = commands.indexOf("\nfn clear_action_error_receipt");
  const serializerStart = commands.indexOf("fn serialized_pretty_receipt");
  const limitStart = commands.indexOf("fn ensure_serialized_receipt_bytes");

  assert.ok(writeJsonStart >= 0, "expected metadata receipt writer");
  assert.ok(writeActionErrorStart > writeJsonStart, "expected action-error receipt writer");
  assert.ok(clearActionErrorStart > writeActionErrorStart, "expected clear helper after writes");
  assert.ok(serializerStart > writeActionErrorStart, "expected shared serializer helper");
  assert.ok(limitStart > serializerStart, "expected serialized-byte limit helper");

  const writeJson = commands.slice(writeJsonStart, writeActionErrorStart);
  const writeActionError = commands.slice(writeActionErrorStart, clearActionErrorStart);
  const serializer = commands.slice(serializerStart, limitStart);
  const limit = commands.slice(limitStart, clearActionErrorStart);

  assert.match(commands, /const MAX_ACTION_ERROR_DISPLAY_CHARS: usize = 500;/);
  assert.match(writeJson, /let bytes = serialized_pretty_receipt\(&value, "metadata"\)\?;/);
  assert.match(
    writeActionError,
    /"command": action_error_display_field\(command\)/,
    "action-error command display must be bounded before serialization",
  );
  assert.match(
    writeActionError,
    /"error": action_error_display_field\(&error\.to_string\(\)\)/,
    "action-error error display must be bounded before serialization",
  );
  assert.match(
    writeActionError,
    /let bytes = serialized_pretty_receipt\(&value, "action error"\)\?;/,
  );
  assert.match(serializer, /serde_json::to_vec_pretty\(value\)/);
  assert.match(serializer, /bytes\.push\(b'\\n'\);/);
  assert.match(serializer, /ensure_serialized_receipt_bytes\(receipt_kind, &bytes\)\?;/);
  assert.ok(
    serializer.indexOf("bytes.push(b'\\n');") <
      serializer.indexOf("ensure_serialized_receipt_bytes(receipt_kind, &bytes)?"),
    "serialized receipt size check must include trailing newline",
  );
  assert.match(
    limit,
    /u64::try_from\(bytes\.len\(\)\)\.unwrap_or\(u64::MAX\) > MAX_RECEIPT_BYTES/,
  );
  assert.ok(
    writeJson.indexOf("serialized_pretty_receipt") < writeJson.indexOf("fs::write"),
    "metadata receipts must be serialized and bounded before file write",
  );
  assert.ok(
    writeActionError.indexOf("serialized_pretty_receipt") < writeActionError.indexOf("fs::write"),
    "action-error receipts must be serialized and bounded before file write",
  );
});

test("DX Agent receipt display strings are redacted and bounded at parser boundaries", () => {
  const receipts = read("crates/agent_ui/src/dx_agent_bridge/receipts.rs");
  const receiptStrings = read("crates/agent_ui/src/dx_agent_bridge/receipts/receipt_strings.rs");

  assert.match(receiptStrings, /const MAX_RECEIPT_DISPLAY_CHARS: usize = 180;/);
  assert.match(receiptStrings, /fn receipt_string_field/);
  assert.match(receiptStrings, /safe_string_field\(value, path\)\.and_then\(bound_receipt_string\)/);
  assert.match(receiptStrings, /fn receipt_string_array_field/);
  assert.match(receiptStrings, /fn receipt_string_values_field/);
  assert.match(receiptStrings, /take\(MAX_RECEIPT_STRING_VALUES\)/);
  assert.match(receiptStrings, /split_whitespace\(\)\.collect::<Vec<_>>\(\)\.join\(" "\)/);
  assert.match(receiptStrings, /take\(MAX_RECEIPT_DISPLAY_CHARS\.saturating_sub\(3\)\)/);
  assert.match(receiptStrings, /bounded\.push_str\("\.\.\."\)/);
  assert.doesNotMatch(receipts, /(?<!receipt_|safe_)string_field\(/);
  assert.doesNotMatch(receipts, /(?<!receipt_)string_array_field\(/);
  assert.doesNotMatch(receipts, /(?<!receipt_)string_values_field\(/);

  const criticalBoundaries = [
    "safe_regeneration_command",
    "next_action",
    "operator_summary",
    "warning_reasons",
    "blocking_reasons",
    "recovery_commands",
    "last_error",
    "command",
    "status",
  ];

  for (const boundary of criticalBoundaries) {
    assert.match(
      receipts,
      new RegExp(`receipt_string_(field|array_field|values_field)\\(value, &\\["${boundary}"\\]`),
      `expected ${boundary} to use the redacted bounded receipt string helper`,
    );
  }

  assert.match(
    receipts,
    /let label = receipt_string_field\(row, &\["label"\]\)\?/,
    "release-gate acceptance row labels must be redacted and bounded",
  );
  assert.match(
    receipts,
    /let status =\s*receipt_string_field\(row, &\["status"\]\)/,
    "release-gate acceptance row statuses must be redacted and bounded",
  );
});
