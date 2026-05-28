import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const source = readFileSync("crates/agent_ui/src/agent_configuration.rs", "utf8");

test("agent configuration caps automatic server insertion before materializing text", () => {
  assert.match(
    source,
    /const MAX_AGENT_SERVER_SETTINGS_INSERTION_BYTES: usize = 512 \* 1024;/,
  );
  assert.match(
    source,
    /const MAX_AGENT_SERVER_SETTINGS_INSERTION_CHARS: usize = 512 \* 1024;/,
  );
  assert.match(
    source,
    /fn settings_editor_snapshot_exceeds_agent_server_insertion_limit/,
  );

  const functionStart = source.indexOf(
    "async fn open_new_agent_servers_entry_in_settings_editor",
  );
  const functionEnd = source.indexOf("\nfn find_text_in_buffer", functionStart);
  assert.notEqual(functionStart, -1, "expected agent-server insertion helper");
  assert.ok(functionEnd > functionStart, "expected helper to stay before find_text_in_buffer");

  const helper = source.slice(functionStart, functionEnd);
  const snapshotLine = "let snapshot = item.buffer().read(cx).snapshot(cx);";
  const sizeGuard =
    "if settings_editor_snapshot_exceeds_agent_server_insertion_limit(&snapshot) {";
  const textLine = "let text = snapshot.text();";
  const editsLine = ".edits_for_update(&text, |settings|";

  assert.match(helper, new RegExp(snapshotLine.replace(/[().]/g, "\\$&")));
  assert.match(helper, new RegExp(sizeGuard.replace(/[().]/g, "\\$&")));
  assert.match(helper, new RegExp(textLine.replace(/[().]/g, "\\$&")));
  assert.match(helper, new RegExp(editsLine.replace(/[().|]/g, "\\$&")));
  assert.doesNotMatch(helper, /snapshot\(cx\)\.text\(\)/);
  assert.ok(
    helper.indexOf(snapshotLine) < helper.indexOf(sizeGuard),
    "the snapshot must be available before the size guard",
  );
  assert.ok(
    helper.indexOf(sizeGuard) < helper.indexOf(textLine),
    "the size guard must run before building the settings String",
  );
  assert.ok(
    helper.indexOf(sizeGuard) < helper.indexOf(editsLine),
    "SettingsStore edits must only run after the size guard",
  );
});

test("oversized settings editor path reports a deferred visible status", () => {
  const limitStart = source.indexOf(
    "fn settings_editor_snapshot_exceeds_agent_server_insertion_limit",
  );
  const limitEnd = source.indexOf("\nfn show_deferred_agent_configuration_status", limitStart);
  const toastStart = source.indexOf("fn show_deferred_agent_configuration_status");
  const toastEnd = source.indexOf("\nfn find_text_in_buffer", toastStart);
  assert.ok(limitStart >= 0, "expected a focused snapshot size guard");
  assert.ok(limitEnd > limitStart, "expected the size guard before the status helper");
  assert.ok(toastEnd > toastStart, "expected deferred status helper before find_text_in_buffer");

  const limitHelper = source.slice(limitStart, limitEnd);
  const toastHelper = source.slice(toastStart, toastEnd);
  assert.match(limitHelper, /let summary = snapshot\.text_summary\(\);/);
  assert.match(limitHelper, /summary\.len\.0 > MAX_AGENT_SERVER_SETTINGS_INSERTION_BYTES/);
  assert.match(limitHelper, /summary\.chars > MAX_AGENT_SERVER_SETTINGS_INSERTION_CHARS/);
  assert.match(toastHelper, /cx\.defer\(move \|cx\|/);
  assert.match(toastHelper, /StatusToast::new\(message, cx/);
  assert.match(toastHelper, /workspace\.toggle_status_toast\(status_toast, cx\)/);
  assert.match(source, /show_deferred_agent_configuration_status\(\s*&workspace,/);
  assert.match(source, /settings file is too large/i);
});
