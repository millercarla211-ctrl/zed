import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX runtime proof status keeps receipt IO and JSON helpers focused", () => {
  const parentPath = "crates/agent_ui/src/dx_runtime_proof_status.rs";
  const receiptsPath = "crates/agent_ui/src/dx_runtime_proof_status/receipts.rs";
  const fieldsPath = "crates/agent_ui/src/dx_runtime_proof_status/fields.rs";
  const summariesPath = "crates/agent_ui/src/dx_runtime_proof_status/summaries.rs";

  assert.ok(existsSync(receiptsPath), "missing focused runtime-proof receipt IO module");
  assert.ok(existsSync(fieldsPath), "missing focused runtime-proof JSON field module");
  assert.ok(existsSync(summariesPath), "missing focused runtime-proof summary parser module");

  const parent = read(parentPath);
  const receipts = read(receiptsPath);
  const fields = read(fieldsPath);
  const summaries = read(summariesPath);

  assert.match(parent, /^mod fields;$/m);
  assert.match(parent, /^mod receipts;$/m);
  assert.match(parent, /^mod summaries;$/m);
  assert.match(parent, /use self::receipts::\{count_receipt_files, latest_receipt_paths\};/);
  assert.match(parent, /use self::summaries::\{parse_import_summary, parse_plan_summary, parse_status_summary\};/);
  assert.doesNotMatch(parent, /fn count_receipt_files\(/);
  assert.doesNotMatch(parent, /fn latest_receipt_paths\(/);
  assert.doesNotMatch(parent, /fn read_json\(/);
  assert.doesNotMatch(parent, /fn string_at\(/);
  assert.doesNotMatch(parent, /fn compact_text\(/);
  assert.doesNotMatch(parent, /fn parse_plan_summary\(/);
  assert.doesNotMatch(parent, /fn parse_import_summary\(/);
  assert.doesNotMatch(parent, /fn parse_status_summary\(/);
  assert.match(receipts, /pub\(super\) fn count_receipt_files/);
  assert.match(receipts, /pub\(super\) fn latest_receipt_paths/);
  assert.match(receipts, /pub\(super\) fn read_json/);
  assert.match(receipts, /const MAX_RECEIPT_BYTES: u64 = 128 \* 1024;/);
  assert.match(receipts, /\.take\(MAX_RECEIPT_BYTES \+ 1\)/);
  assert.match(receipts, /if buffer\.len\(\) > MAX_RECEIPT_BYTES as usize \{\s*return None;\s*\}/);
  assert.doesNotMatch(receipts, /\.take\(MAX_RECEIPT_BYTES\)/);
  assert.match(fields, /pub\(super\) fn string_at/);
  assert.match(fields, /pub\(super\) fn compact_string_array_at/);
  assert.match(fields, /fn compact_text/);
  assert.match(summaries, /use super::fields::\{/);
  assert.match(summaries, /use super::receipts::read_json;/);
  assert.match(summaries, /pub\(super\) fn parse_plan_summary/);
  assert.match(summaries, /pub\(super\) fn parse_import_summary/);
  assert.match(summaries, /pub\(super\) fn parse_status_summary/);
  assert.doesNotMatch(
    summaries,
    /(^|[^\w])string_at\(/,
    "runtime-proof summaries should not use raw string extraction for display and prompt fields",
  );
  assert.match(summaries, /status: compact_string_at\(status, "status"\)/);
  assert.match(
    summaries,
    /expected_final_command: compact_string_at\(request, "expected_final_command"\)/,
  );
  assert.match(summaries, /next_action: compact_string_at\(plan, "next_action"\)/);
  assert.match(
    summaries,
    /operator_status: compact_string_at\(request, "operator_status"\)/,
  );
  assert.match(
    summaries,
    /validation_status: compact_string_at\(validation, "status"\)/,
  );
  assert.match(
    summaries,
    /headline: compact_string_at\(operator_status_copy, "headline"\)/,
  );
  assert.match(summaries, /headline: compact_string_at\(status_copy, "headline"\)/);

  assert.ok(lineCount(parentPath) < 280, "dx_runtime_proof_status.rs should stay focused on snapshot assembly");
  assert.ok(lineCount(receiptsPath) < 95, "runtime-proof receipt IO module should stay small");
  assert.ok(lineCount(fieldsPath) < 95, "runtime-proof JSON field module should stay small");
  assert.ok(lineCount(summariesPath) < 130, "runtime-proof summary parser module should stay small");
});
