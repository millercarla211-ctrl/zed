import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX launch readiness keeps packet IO, JSON helpers, and review helpers focused", () => {
  const parentPath = "crates/agent_ui/src/dx_launch_readiness.rs";
  const fieldsPath = "crates/agent_ui/src/dx_launch_readiness/packet_fields.rs";
  const packetsPath = "crates/agent_ui/src/dx_launch_readiness/packets.rs";
  const reviewPath = "crates/agent_ui/src/dx_launch_readiness/review.rs";
  const examplesPath = "crates/agent_ui/src/dx_launch_readiness/examples.rs";
  const statusCountsPath = "crates/agent_ui/src/dx_launch_readiness/status_counts.rs";

  assert.ok(existsSync(fieldsPath), "missing focused launch-readiness packet field module");
  assert.ok(existsSync(packetsPath), "missing focused launch-readiness packet IO module");
  assert.ok(existsSync(reviewPath), "missing focused launch-readiness review policy module");
  assert.ok(existsSync(examplesPath), "missing focused launch-readiness example helper module");
  assert.ok(existsSync(statusCountsPath), "missing focused launch-readiness status count module");

  const parent = read(parentPath);
  const fields = read(fieldsPath);
  const packets = read(packetsPath);
  const review = read(reviewPath);
  const examples = read(examplesPath);
  const statusCounts = read(statusCountsPath);

  assert.match(parent, /^mod examples;$/m);
  assert.match(parent, /^mod packet_fields;$/m);
  assert.match(parent, /^mod packets;$/m);
  assert.match(parent, /^mod review;$/m);
  assert.match(parent, /^mod status_counts;$/m);
  assert.match(parent, /use self::examples::\{balanced_examples, push_recovery_commands, push_unique\};/);
  assert.match(parent, /use self::packet_fields::\{\s*bool_field, packet_status, pointer_string, pointer_usize, string_field, usize_field,\s*\};/);
  assert.match(parent, /use self::packets::read_checked_packet;/);
  assert.match(parent, /use self::review::\{command_fanout_count, redaction_requires_review\};/);
  assert.doesNotMatch(parent, /fn read_checked_packet\(/);
  assert.doesNotMatch(parent, /fn read_json_packet\(/);
  assert.doesNotMatch(parent, /fn packet_status\(/);
  assert.doesNotMatch(parent, /fn push_recovery_commands\(/);
  assert.doesNotMatch(parent, /fn command_fanout_count\(/);
  assert.match(fields, /pub\(super\) fn packet_status/);
  assert.match(fields, /pub\(super\) fn pointer_usize/);
  assert.match(packets, /pub\(super\) fn read_checked_packet/);
  assert.match(packets, /MAX_PACKET_BYTES/);
  assert.match(review, /pub\(super\) fn command_fanout_count/);
  assert.match(review, /exports_secret_values/);
  assert.match(examples, /pub\(super\) fn balanced_examples/);
  assert.match(examples, /pub\(super\) fn push_recovery_commands/);
  assert.match(statusCounts, /impl DxLaunchReadinessStatusCounts/);
  assert.match(statusCounts, /pub\(super\) fn record/);

  assert.ok(lineCount(parentPath) < 310, "dx_launch_readiness.rs should stay focused on snapshot assembly");
  assert.ok(lineCount(fieldsPath) < 75, "launch-readiness packet field module should stay small");
  assert.ok(lineCount(packetsPath) < 65, "launch-readiness packet IO module should stay small");
  assert.ok(lineCount(reviewPath) < 70, "launch-readiness review policy module should stay small");
  assert.ok(lineCount(examplesPath) < 80, "launch-readiness example helper module should stay small");
  assert.ok(lineCount(statusCountsPath) < 35, "launch-readiness status count module should stay small");
});
