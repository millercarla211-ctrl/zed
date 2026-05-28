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
  assert.match(packets, /\.take\(MAX_PACKET_BYTES \+ 1\)\s*\.read_to_end\(&mut buffer\)/s);
  assert.match(packets, /buffer\.len\(\) as u64 > MAX_PACKET_BYTES/);
  assert.match(packets, /serde_json::from_slice\(&buffer\)/);
  assert.doesNotMatch(packets, /read_to_string/);
  assert.match(review, /pub\(super\) fn redaction_requires_review/);
  assert.match(review, /exports_secret_values/);

  assert.ok(lineCount(parentPath) < 310, "dx_launch_contracts.rs should stay focused on snapshot assembly");
  assert.ok(lineCount(fieldsPath) < 80, "launch-contract field helper module should stay small");
  assert.ok(lineCount(packetsPath) < 60, "launch-contract packet IO module should stay small");
  assert.ok(lineCount(reviewPath) < 50, "launch-contract review module should stay small");
});

test("DX launch contracts compact display strings before snapshot use", () => {
  const parentPath = "crates/agent_ui/src/dx_launch_contracts.rs";
  const parent = read(parentPath);

  assert.match(parent, /const MAX_LAUNCH_CONTRACT_DISPLAY_CHARS: usize = 240;/);
  assert.match(parent, /fn compact_display_string\(value: &str\) -> Option<String>/);
  assert.match(parent, /split_whitespace\(\)\.collect::<Vec<_>>\(\)\.join\(" "\)/);
  assert.match(parent, /\.take\(MAX_LAUNCH_CONTRACT_DISPLAY_CHARS\.saturating_sub\(3\)\)/s);
  assert.match(parent, /bounded\.push_str\("\.\.\."\)/);
  assert.match(parent, /fn compact_display_string_or\(value: Option<&str>, fallback: &str\) -> String/);
  assert.match(parent, /fn compact_optional_display_string\(value: Option<&str>\) -> Option<String>/);
  assert.match(parent, /fn compact_display_strings\(values: Vec<String>\) -> Vec<String>/);

  assert.match(
    parent,
    /let first_packets = packets[\s\S]*compact_optional_display_string\(string_field\(packet, "command"\)\)/,
  );
  assert.match(
    parent,
    /let first_action = actions[\s\S]*compact_optional_display_string\(string_field\(action, "command"\)\)/,
  );
  assert.match(
    parent,
    /let startup_commands =\s*compact_display_strings\(pointer_string_array\([\s\S]*handoff_ref,[\s\S]*"\/polling\/startup_commands"[\s\S]*\)\);/,
  );
  assert.match(
    parent,
    /let detail_commands =\s*compact_display_strings\(pointer_string_array\([\s\S]*handoff_ref,[\s\S]*"\/polling\/detail_commands"[\s\S]*\)\);/,
  );
  assert.match(
    parent,
    /let diagnostics_commands =\s*compact_display_strings\(pointer_string_array\([\s\S]*handoff_ref,[\s\S]*"\/polling\/diagnostics_commands"[\s\S]*\)\);/,
  );
  assert.match(
    parent,
    /let refresh_command =\s*compact_optional_display_string\(pointer_string\([\s\S]*handoff_ref,[\s\S]*"\/polling\/foreground_refresh_command"[\s\S]*\)\);/,
  );
  assert.match(
    parent,
    /let cached_receipt_path =\s*compact_optional_display_string\(pointer_string\([\s\S]*handoff_ref,[\s\S]*"\/polling\/cached_receipt_path"[\s\S]*\)\);/,
  );
  assert.match(parent, /let last_error = errors[\s\S]*\.first\(\)[\s\S]*\.and_then\(\|error\| compact_display_string\(error\)\);/);
  assert.match(parent, /let status = if !manifest_present \|\| !handoff_present \{[\s\S]*compact_display_string_or\([\s\S]*string_field\(value, "status"\)[\s\S]*"ready"[\s\S]*\);/);
  assert.match(
    parent,
    /let operator_summary = compact_display_string_or\([\s\S]*string_field\(value, "operator_summary"\)[\s\S]*"Launch handoff packets are not available\."/,
  );
  assert.match(parent, /let next_action = if !errors\.is_empty\(\) \{[\s\S]*compact_display_string_or\([\s\S]*string_field\(value, "next_action"\)/);

  assert.doesNotMatch(parent, /string_field\(packet, "command"\)\.map\(ToString::to_string\)/);
  assert.doesNotMatch(parent, /string_field\(action, "command"\)\s*\)\s*\.map\(ToString::to_string\)/);
  assert.doesNotMatch(parent, /pointer_string\(handoff_ref, "[^"]+"\)\.map\(ToString::to_string\)/);
  assert.doesNotMatch(parent, /\.unwrap_or\("ready"\)\s*;/);
});
