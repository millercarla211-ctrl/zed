import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX launch receipts keep IO, paths, fields, freshness, and summaries focused", () => {
  const parentPath = "crates/agent_ui/src/dx_launch_receipts.rs";
  const fieldsPath = "crates/agent_ui/src/dx_launch_receipts/fields.rs";
  const freshnessPath = "crates/agent_ui/src/dx_launch_receipts/freshness.rs";
  const pathsPath = "crates/agent_ui/src/dx_launch_receipts/paths.rs";
  const receiptIoPath = "crates/agent_ui/src/dx_launch_receipts/receipt_io.rs";
  const summaryPath = "crates/agent_ui/src/dx_launch_receipts/summary.rs";

  assert.ok(existsSync(fieldsPath), "missing focused launch-receipts field module");
  assert.ok(existsSync(freshnessPath), "missing focused launch-receipts freshness module");
  assert.ok(existsSync(pathsPath), "missing focused launch-receipts path module");
  assert.ok(existsSync(receiptIoPath), "missing focused launch-receipts IO module");
  assert.ok(existsSync(summaryPath), "missing focused launch-receipts summary module");

  const parent = read(parentPath);
  const fields = read(fieldsPath);
  const freshness = read(freshnessPath);
  const paths = read(pathsPath);
  const receiptIo = read(receiptIoPath);
  const summary = read(summaryPath);

  assert.match(parent, /^mod fields;$/m);
  assert.match(parent, /^mod freshness;$/m);
  assert.match(parent, /^mod paths;$/m);
  assert.match(parent, /^mod receipt_io;$/m);
  assert.match(parent, /^mod summary;$/m);
  assert.match(parent, /use self::freshness::launch_receipt_operator_summary;/);
  assert.match(parent, /use self::paths::\{launch_snapshot_paths, now_ms\};/);
  assert.doesNotMatch(parent, /fn read_json_receipt\(/);
  assert.doesNotMatch(parent, /fn launch_snapshot_paths\(/);
  assert.doesNotMatch(parent, /fn freshness_state\(/);
  assert.doesNotMatch(parent, /fn optional_string_field\(/);
  assert.doesNotMatch(parent, /impl DxLaunchReceiptSummary/);
  assert.match(fields, /pub\(super\) fn optional_string_field/);
  assert.match(fields, /fn render_safe_string/);
  assert.match(freshness, /pub\(super\) fn freshness_state/);
  assert.match(freshness, /pub\(super\) fn launch_receipt_operator_summary/);
  assert.match(paths, /pub\(super\) fn launch_snapshot_paths/);
  assert.match(paths, /pub\(super\) fn now_ms/);
  assert.match(receiptIo, /pub\(super\) fn read_json_receipt/);
  assert.match(receiptIo, /MAX_RECEIPT_BYTES/);
  assert.match(summary, /impl DxLaunchReceiptSummary/);
  assert.match(summary, /pub\(super\) fn from_path/);

  assert.ok(lineCount(parentPath) < 250, "dx_launch_receipts.rs should stay focused on snapshot assembly");
  assert.ok(lineCount(fieldsPath) < 45, "launch-receipts field module should stay small");
  assert.ok(lineCount(freshnessPath) < 60, "launch-receipts freshness module should stay small");
  assert.ok(lineCount(pathsPath) < 70, "launch-receipts path module should stay small");
  assert.ok(lineCount(receiptIoPath) < 55, "launch-receipts IO module should stay small");
  assert.ok(lineCount(summaryPath) < 95, "launch-receipts summary module should stay small");
});
