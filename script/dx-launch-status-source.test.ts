import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX launch status keeps receipt IO, JSON helpers, review, and summaries focused", () => {
  const parentPath = "crates/agent_ui/src/dx_launch_status.rs";
  const fieldsPath = "crates/agent_ui/src/dx_launch_status/fields.rs";
  const receiptsPath = "crates/agent_ui/src/dx_launch_status/receipts.rs";
  const reviewPath = "crates/agent_ui/src/dx_launch_status/review.rs";
  const summariesPath = "crates/agent_ui/src/dx_launch_status/summaries.rs";

  assert.ok(existsSync(fieldsPath), "missing focused launch-status field module");
  assert.ok(existsSync(receiptsPath), "missing focused launch-status receipt IO module");
  assert.ok(existsSync(reviewPath), "missing focused launch-status review module");
  assert.ok(existsSync(summariesPath), "missing focused launch-status summary module");

  const parent = read(parentPath);
  const fields = read(fieldsPath);
  const receipts = read(receiptsPath);
  const review = read(reviewPath);
  const summaries = read(summariesPath);

  assert.match(parent, /^mod fields;$/m);
  assert.match(parent, /^mod receipts;$/m);
  assert.match(parent, /^mod review;$/m);
  assert.match(parent, /^mod summaries;$/m);
  assert.match(parent, /use self::fields::string_field;/);
  assert.match(parent, /use self::receipts::read_json_receipt;/);
  assert.match(parent, /use self::review::redaction_requires_review;/);
  assert.match(parent, /use self::summaries::\{agents_summary, discovery_summary, tokens_summary\};/);
  assert.doesNotMatch(parent, /fn read_json_receipt\(/);
  assert.doesNotMatch(parent, /fn redaction_requires_review\(/);
  assert.doesNotMatch(parent, /fn pointer_string\(/);
  assert.doesNotMatch(parent, /fn agents_summary\(/);
  assert.doesNotMatch(parent, /impl DxLaunchAgentsSummary/);
  assert.match(fields, /pub\(super\) fn string_field/);
  assert.match(fields, /pub\(super\) fn pointer_usize/);
  assert.match(receipts, /pub\(super\) fn read_json_receipt/);
  assert.match(receipts, /MAX_RECEIPT_BYTES/);
  assert.match(receipts, /take\(MAX_RECEIPT_BYTES \+ 1\)/);
  assert.match(receipts, /if buffer\.len\(\) as u64 > MAX_RECEIPT_BYTES/);
  assert.match(receipts, /serde_json::from_slice\(&buffer\)/);
  assert.doesNotMatch(receipts, /read_to_string/);
  assert.match(review, /pub\(super\) fn redaction_requires_review/);
  assert.match(review, /exports_secret_values/);
  assert.match(summaries, /pub\(super\) fn agents_summary/);
  assert.match(summaries, /impl DxLaunchTokensSummary/);

  assert.ok(lineCount(parentPath) < 250, "dx_launch_status.rs should stay focused on snapshot assembly");
  assert.ok(lineCount(fieldsPath) < 60, "launch-status field helper module should stay small");
  assert.ok(lineCount(receiptsPath) < 55, "launch-status receipt IO module should stay small");
  assert.ok(lineCount(reviewPath) < 50, "launch-status review module should stay small");
  assert.ok(lineCount(summariesPath) < 130, "launch-status summary module should stay small");
});
