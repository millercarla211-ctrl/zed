import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX Check panel stays split by reader and parser ownership", () => {
  const parent = read("crates/agent_ui/src/dx_check_panel.rs");
  const expectedModules = [
    "crates/agent_ui/src/dx_check_panel/parser.rs",
    "crates/agent_ui/src/dx_check_panel/reader.rs",
  ];

  for (const module of expectedModules) {
    assert.ok(existsSync(module), `expected focused DX Check panel module ${module}`);
  }

  assert.match(parent, /^mod parser;$/m);
  assert.match(parent, /^mod reader;$/m);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_check_panel.rs") < 450,
    "dx_check_panel.rs should stay a cache/type facade",
  );
});

test("DX Check panel delegates receipt IO and panel parsing", () => {
  const parent = read("crates/agent_ui/src/dx_check_panel.rs");
  const parser = read("crates/agent_ui/src/dx_check_panel/parser.rs");
  const reader = read("crates/agent_ui/src/dx_check_panel/reader.rs");

  assert.match(parent, /reader::read_latest_check_panel/);
  assert.doesNotMatch(parent, /fn read_check_receipt/);
  assert.doesNotMatch(parent, /fn panel_from_zed_value/);
  assert.match(reader, /pub\(super\) fn read_latest_check_panel/);
  assert.match(parser, /pub\(super\) fn panel_from_receipt_value/);
  assert.match(parser, /pub\(super\) fn missing_snapshot/);
  assert.match(parser, /pub\(super\) fn malformed_snapshot/);
  assert.match(reader, /use crate::dx_deploy_root_key::deploy_root_key;/);
  assert.match(reader, /let path_key = deploy_root_key\(&path\);/);
  assert.match(reader, /deploy_root_key\(existing\) == path_key/);
  assert.ok(lineCount("crates/agent_ui/src/dx_check_panel/reader.rs") < 140);
  assert.ok(lineCount("crates/agent_ui/src/dx_check_panel/parser.rs") < 620);
});

test("DX Check panel reader uses sentinel-byte bounded JSON reads", () => {
  const reader = read("crates/agent_ui/src/dx_check_panel/reader.rs");

  assert.match(reader, /File::open\(path\)/);
  assert.match(reader, /\.take\(MAX_RECEIPT_BYTES \+ 1\)\s*\.read_to_end\(&mut receipt\)/);
  assert.match(reader, /receipt\.len\(\) as u64 > MAX_RECEIPT_BYTES/);
  assert.match(reader, /serde_json::from_slice::<Value>\(&receipt\)/);
  assert.doesNotMatch(reader, /read_to_string/);
});

test("DX Check panel parser bounds user-controlled snapshot strings", () => {
  const parser = read("crates/agent_ui/src/dx_check_panel/parser.rs");

  assert.match(parser, /const MAX_PANEL_TEXT_CHARS: usize = \d+;/);
  assert.match(parser, /fn bounded_panel_text\(value: &str\) -> Option<String>/);
  assert.match(parser, /fn bounded_string_from\(value: Option<&Value>\) -> Option<String>/);
  assert.match(
    parser,
    /fn bounded_string_at<const N: usize>\(value: &Value, path: \[&str; N\]\) -> Option<String>/,
  );

  const boundedSnapshotFields = [
    /status: bounded_string_from\(zed\.get\("status"\)\)/,
    /weight_profile: bounded_string_from\(zed\.get\("weight_profile"\)\)/,
    /refresh_command: bounded_string_from\(zed\.get\("refresh_command"\)\)/,
    /detail_command: bounded_string_from\(zed\.get\("detail_command"\)\)/,
    /title: bounded_string_from\(view_model\.get\("title"\)\)/,
    /receipt_error: if status == "malformed" \{\s+bounded_string_from\(view_model\.get\("empty_state"\)\)/,
    /last_run_label\(\s+bounded_string_from\(view_model\.get\("last_run_label"\)\)/,
    /refresh_command: bounded_string_at\(view_model, \["primary_action", "command"\]\)/,
    /detail_command: bounded_string_at\(view_model, \["secondary_action", "command"\]\)/,
  ];

  for (const pattern of boundedSnapshotFields) {
    assert.match(parser, pattern);
  }

  const boundedRowPatterns = [
    /let title = bounded_string_from\(section\.get\("title"\)\)/,
    /status: bounded_string_from\(section\.get\("status"\)\)/,
    /let message = bounded_string_from\(notice\.get\("message"\)\)\?/,
    /code: bounded_string_from\(notice\.get\("code"\)\)/,
    /next_action: bounded_string_from\(notice\.get\("next_action"\)\)/,
    /let label = bounded_string_from\(fix\.get\("label"\)\)\?/,
    /let next_action = bounded_string_from\(fix\.get\("next_action"\)\)\?/,
    /let raw_command = string_from\(fix\.get\("command"\)\);/,
    /let command = raw_command\.and_then\(bounded_panel_text\);/,
    /risk_level: bounded_string_from\(fix\.get\("risk_level"\)\)/,
    /quick_fix_risk_level\(raw_command\)/,
    /quick_fix_requires_approval\(raw_command\)/,
    /quick_fix_writes_receipts\(raw_command\)/,
    /string_array\(value: Option<&Value>\).*bounded_panel_text/s,
  ];

  for (const pattern of boundedRowPatterns) {
    assert.match(parser, pattern);
  }
});
