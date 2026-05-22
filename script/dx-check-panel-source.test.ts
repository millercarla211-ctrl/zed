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
  assert.ok(lineCount("crates/agent_ui/src/dx_check_panel/reader.rs") < 140);
  assert.ok(lineCount("crates/agent_ui/src/dx_check_panel/parser.rs") < 560);
});
