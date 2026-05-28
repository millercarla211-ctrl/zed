import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX source sets keep receipt IO and JSON field helpers in focused modules", () => {
  const parentPath = "crates/agent_ui/src/dx_source_sets.rs";
  const formattingPath = "crates/agent_ui/src/dx_source_sets/formatting.rs";
  const receiptsPath = "crates/agent_ui/src/dx_source_sets/receipts.rs";
  const fieldsPath = "crates/agent_ui/src/dx_source_sets/receipt_fields.rs";
  const restorePath = "crates/agent_ui/src/dx_source_sets/restore.rs";

  assert.ok(existsSync(formattingPath), "missing focused source-set formatting module");
  assert.ok(existsSync(receiptsPath), "missing focused source-set receipt IO module");
  assert.ok(existsSync(fieldsPath), "missing focused source-set JSON field helper module");
  assert.ok(existsSync(restorePath), "missing focused source-set restore warning module");

  const parent = read(parentPath);
  const formatting = read(formattingPath);
  const receipts = read(receiptsPath);
  const fields = read(fieldsPath);
  const restore = read(restorePath);

  assert.match(parent, /^mod formatting;$/m);
  assert.match(parent, /^mod receipt_fields;$/m);
  assert.match(parent, /^mod receipts;$/m);
  assert.match(parent, /^mod restore;$/m);
  assert.match(
    parent,
    /use self::formatting::\{display_name, format_bytes, short_hash, source_set_status\};/,
  );
  assert.match(parent, /use self::receipt_fields::\{/);
  assert.match(parent, /use self::receipts::\{ReceiptCandidate, latest_receipts, read_receipt_json\};/);
  assert.match(parent, /use self::restore::forge_restore_warnings;/);
  assert.doesNotMatch(parent, /fn latest_receipts\(/);
  assert.doesNotMatch(parent, /fn read_receipt_json\(/);
  assert.doesNotMatch(parent, /fn value_at</);
  assert.doesNotMatch(parent, /fn format_bytes\(/);
  assert.doesNotMatch(parent, /fn source_set_status\(/);
  assert.doesNotMatch(parent, /fn forge_restore_warnings\(/);
  assert.match(formatting, /pub\(super\) fn display_name/);
  assert.match(formatting, /pub\(super\) fn format_bytes/);
  assert.match(formatting, /pub\(super\) fn source_set_status/);
  assert.match(receipts, /pub\(super\) struct ReceiptCandidate/);
  assert.match(receipts, /pub\(super\) fn latest_receipts/);
  assert.match(receipts, /pub\(super\) fn read_receipt_json/);
  assert.match(fields, /pub\(super\) fn string_at/);
  assert.match(fields, /pub\(super\) fn array_strings_at/);
  assert.match(restore, /pub\(super\) fn forge_restore_warnings/);
  assert.match(restore, /target_mutation_applied/);

  assert.ok(lineCount(parentPath) < 420, "dx_source_sets.rs should stay a coordinator");
  assert.ok(lineCount(formattingPath) < 55, "source-set formatting module should stay small");
  assert.ok(lineCount(receiptsPath) < 90, "source-set receipt IO module should stay small");
  assert.ok(lineCount(fieldsPath) < 70, "source-set field helper module should stay small");
  assert.ok(lineCount(restorePath) < 50, "source-set restore warning module should stay small");
});

test("DX source-set bounded readers reject files larger than their parse limits", () => {
  const receiptsPath = "crates/agent_ui/src/dx_source_sets/receipts.rs";
  const toolchainPath = "crates/agent_ui/src/dx_source_sets/dx_editor_toolchain.rs";

  const receipts = read(receiptsPath);
  const toolchain = read(toolchainPath);

  assert.match(receipts, /\.take\(MAX_RECEIPT_BYTES \+ 1\)/);
  assert.match(receipts, /buffer\.len\(\) as u64 > MAX_RECEIPT_BYTES/);
  assert.doesNotMatch(receipts, /\.take\(MAX_RECEIPT_BYTES\)/);

  assert.match(toolchain, /\.take\(MAX_DX_CONFIG_BYTES \+ 1\)/);
  assert.match(toolchain, /buffer\.len\(\) as u64 > MAX_DX_CONFIG_BYTES/);
  assert.match(toolchain, /let config = read_bounded_utf8\(&config_path\)\?;/);
  assert.doesNotMatch(toolchain, /\.take\(MAX_DX_CONFIG_BYTES\)/);
  assert.doesNotMatch(toolchain, /read_bounded_utf8\(&config_path\)\.unwrap_or_default\(\)/);
});
