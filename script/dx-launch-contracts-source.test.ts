import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX launch contracts keep packet IO, JSON helpers, and review helpers focused", () => {
  const parentPath = "crates/agent_ui/src/dx_launch_contracts.rs";
  const fieldsPath = "crates/agent_ui/src/dx_launch_contracts/fields.rs";
  const packetsPath = "crates/agent_ui/src/dx_launch_contracts/packets.rs";
  const reviewPath = "crates/agent_ui/src/dx_launch_contracts/review.rs";

  assert.ok(existsSync(fieldsPath), "missing focused launch-contract field module");
  assert.ok(existsSync(packetsPath), "missing focused launch-contract packet IO module");
  assert.ok(existsSync(reviewPath), "missing focused launch-contract review module");

  const parent = read(parentPath);
  const fields = read(fieldsPath);
  const packets = read(packetsPath);
  const review = read(reviewPath);

  assert.match(parent, /^mod fields;$/m);
  assert.match(parent, /^mod packets;$/m);
  assert.match(parent, /^mod review;$/m);
  assert.match(parent, /use self::fields::\{/);
  assert.match(parent, /array_len, bool_field, pointer_string, pointer_string_array, string_field, usize_field,/);
  assert.match(parent, /use self::packets::read_json_packet;/);
  assert.match(parent, /use self::review::redaction_requires_review;/);
  assert.doesNotMatch(parent, /fn read_json_packet\(/);
  assert.doesNotMatch(parent, /fn string_field</);
  assert.doesNotMatch(parent, /fn pointer_string_array\(/);
  assert.doesNotMatch(parent, /fn redaction_requires_review\(/);
  assert.match(fields, /pub\(super\) fn string_field/);
  assert.match(fields, /pub\(super\) fn pointer_string_array/);
  assert.match(packets, /pub\(super\) fn read_json_packet/);
  assert.match(packets, /MAX_PACKET_BYTES/);
  assert.match(review, /pub\(super\) fn redaction_requires_review/);
  assert.match(review, /exports_secret_values/);

  assert.ok(lineCount(parentPath) < 260, "dx_launch_contracts.rs should stay focused on snapshot assembly");
  assert.ok(lineCount(fieldsPath) < 80, "launch-contract field helper module should stay small");
  assert.ok(lineCount(packetsPath) < 60, "launch-contract packet IO module should stay small");
  assert.ok(lineCount(reviewPath) < 50, "launch-contract review module should stay small");
});
