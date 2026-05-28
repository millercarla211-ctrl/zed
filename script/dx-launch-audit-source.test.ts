import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX launch audit keeps packet IO, JSON helpers, and review policy focused", () => {
  const parentPath = "crates/agent_ui/src/dx_launch_audit.rs";
  const fieldsPath = "crates/agent_ui/src/dx_launch_audit/packet_fields.rs";
  const packetsPath = "crates/agent_ui/src/dx_launch_audit/packets.rs";
  const reviewPath = "crates/agent_ui/src/dx_launch_audit/review.rs";
  const statusPath = "crates/agent_ui/src/dx_launch_audit/status_summaries.rs";

  assert.ok(existsSync(fieldsPath), "missing focused launch-audit packet field module");
  assert.ok(existsSync(packetsPath), "missing focused launch-audit packet IO module");
  assert.ok(existsSync(reviewPath), "missing focused launch-audit review policy module");
  assert.ok(existsSync(statusPath), "missing focused launch-audit status summary module");

  const parent = read(parentPath);
  const fields = read(fieldsPath);
  const packets = read(packetsPath);
  const review = read(reviewPath);
  const status = read(statusPath);

  assert.match(parent, /^mod packet_fields;$/m);
  assert.match(parent, /^mod packets;$/m);
  assert.match(parent, /^mod review;$/m);
  assert.match(parent, /^mod status_summaries;$/m);
  assert.match(parent, /use self::packet_fields::\{array_len, bool_field, bool_label, string_field, usize_field\};/);
  assert.match(parent, /use self::packets::read_checked_packet;/);
  assert.match(parent, /use self::review::\{command_fanout_count, redaction_requires_review\};/);
  assert.match(parent, /use self::status_summaries::\{/);
  assert.match(parent, /status_agent_summary, status_discovery_summary, status_token_summary,/);
  assert.doesNotMatch(parent, /fn read_checked_packet\(/);
  assert.doesNotMatch(parent, /fn read_json_packet\(/);
  assert.doesNotMatch(parent, /fn string_field</);
  assert.doesNotMatch(parent, /fn command_fanout_count\(/);
  assert.doesNotMatch(parent, /fn redaction_requires_review\(/);
  assert.doesNotMatch(parent, /fn status_agent_summary\(/);
  assert.doesNotMatch(parent, /fn status_token_summary\(/);
  assert.match(fields, /pub\(super\) fn string_field/);
  assert.match(fields, /pub\(super\) fn bool_label/);
  assert.match(packets, /pub\(super\) fn read_checked_packet/);
  assert.match(packets, /MAX_PACKET_BYTES/);
  assert.match(packets, /\.take\(MAX_PACKET_BYTES \+ 1\)/);
  assert.match(packets, /read_to_end\(&mut buffer\)/);
  assert.match(packets, /buffer\.len\(\) as u64 > MAX_PACKET_BYTES/);
  assert.match(packets, /serde_json::from_slice\(&buffer\)/);
  assert.doesNotMatch(packets, /read_to_string/);
  assert.doesNotMatch(packets, /serde_json::from_str/);
  assert.match(review, /pub\(super\) fn command_fanout_count/);
  assert.match(review, /exports_secret_values/);
  assert.match(status, /pub\(super\) fn status_agent_summary/);
  assert.match(status, /pub\(super\) fn status_discovery_summary/);

  assert.ok(lineCount(parentPath) < 340, "dx_launch_audit.rs should stay focused on snapshot assembly");
  assert.ok(lineCount(fieldsPath) < 65, "launch-audit packet field module should stay small");
  assert.ok(lineCount(packetsPath) < 65, "launch-audit packet IO module should stay small");
  assert.ok(lineCount(reviewPath) < 70, "launch-audit review policy module should stay small");
  assert.ok(lineCount(statusPath) < 75, "launch-audit status summary module should stay small");
});
