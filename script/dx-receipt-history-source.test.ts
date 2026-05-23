import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX receipt history keeps bucket scanning, receipt IO, Forge summaries, and fields focused", () => {
  const parentPath = "crates/agent_ui/src/dx_receipt_history.rs";
  const bucketsPath = "crates/agent_ui/src/dx_receipt_history/buckets.rs";
  const fieldsPath = "crates/agent_ui/src/dx_receipt_history/fields.rs";
  const forgePath = "crates/agent_ui/src/dx_receipt_history/forge_history.rs";
  const forgeFieldsPath = "crates/agent_ui/src/dx_receipt_history/forge_receipt_fields.rs";
  const receiptFilesPath = "crates/agent_ui/src/dx_receipt_history/receipt_files.rs";
  const receiptIoPath = "crates/agent_ui/src/dx_receipt_history/receipt_io.rs";

  assert.ok(existsSync(bucketsPath), "missing focused receipt-history bucket scanner");
  assert.ok(existsSync(fieldsPath), "missing focused receipt-history field helpers");
  assert.ok(existsSync(forgePath), "missing focused receipt-history Forge parser");
  assert.ok(existsSync(forgeFieldsPath), "missing focused receipt-history Forge field parser");
  assert.ok(existsSync(receiptFilesPath), "missing focused receipt-history file walker");
  assert.ok(existsSync(receiptIoPath), "missing focused receipt-history receipt IO");

  const parent = read(parentPath);
  const buckets = read(bucketsPath);
  const fields = read(fieldsPath);
  const forge = read(forgePath);
  const forgeFields = read(forgeFieldsPath);
  const receiptFiles = read(receiptFilesPath);
  const receiptIo = read(receiptIoPath);

  assert.match(parent, /^mod buckets;$/m);
  assert.match(parent, /^mod fields;$/m);
  assert.match(parent, /^mod forge_history;$/m);
  assert.match(parent, /^mod forge_receipt_fields;$/m);
  assert.match(parent, /^mod receipt_files;$/m);
  assert.match(parent, /^mod receipt_io;$/m);
  assert.match(parent, /use self::buckets::scan_tool_history;/);
  assert.doesNotMatch(parent, /fn scan_bucket\(/);
  assert.doesNotMatch(parent, /fn count_receipt_files\(/);
  assert.doesNotMatch(parent, /fn push_latest_receipts\(/);
  assert.doesNotMatch(parent, /fn forge_receipt_summary\(/);
  assert.doesNotMatch(parent, /fn read_json\(/);
  assert.doesNotMatch(parent, /fn value_at</);
  assert.match(buckets, /pub\(super\) fn scan_tool_history/);
  assert.match(buckets, /fn scan_bucket/);
  assert.match(buckets, /Forge History/);
  assert.match(fields, /pub\(super\) fn string_field/);
  assert.match(fields, /pub\(super\) fn bool_field/);
  assert.match(fields, /pub\(super\) fn usize_field/);
  assert.match(forge, /pub\(super\) fn forge_receipt_summary/);
  assert.match(forge, /forge_history_kind/);
  assert.doesNotMatch(forge, /fn forge_history_kind/);
  assert.doesNotMatch(forge, /fn forge_history_target_path/);
  assert.match(forgeFields, /pub\(super\) fn forge_history_kind/);
  assert.match(forgeFields, /pub\(super\) fn forge_history_target_path/);
  assert.match(forgeFields, /restore_target_plan/);
  assert.match(receiptFiles, /pub\(super\) fn count_receipt_files/);
  assert.match(receiptFiles, /pub\(super\) fn push_latest_receipts/);
  assert.match(receiptFiles, /pub\(super\) fn root_label/);
  assert.match(receiptFiles, /fn is_receipt_file/);
  assert.match(receiptIo, /pub\(super\) fn read_json/);
  assert.match(receiptIo, /MAX_RECEIPT_BYTES/);

  assert.ok(lineCount(parentPath) < 95, "dx_receipt_history.rs should stay focused on cache and public snapshot types");
  assert.ok(lineCount(bucketsPath) < 95, "receipt-history bucket scanner should stay small");
  assert.ok(lineCount(fieldsPath) < 35, "receipt-history field helpers should stay small");
  assert.ok(lineCount(forgePath) < 85, "receipt-history Forge summary parser should stay small");
  assert.ok(lineCount(forgeFieldsPath) < 120, "receipt-history Forge field parser should stay small");
  assert.ok(lineCount(receiptFilesPath) < 125, "receipt-history file walker should stay small");
  assert.ok(lineCount(receiptIoPath) < 25, "receipt-history receipt IO should stay small");
});
