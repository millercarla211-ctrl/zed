import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const source = () => read("crates/web_preview/src/web_preview_view.rs");

const sliceBetween = (contents: string, startNeedle: string, endNeedle: string) => {
  const start = contents.indexOf(startNeedle);
  assert.notEqual(start, -1, `missing ${startNeedle}`);
  const end = contents.indexOf(endNeedle, start + startNeedle.length);
  assert.notEqual(end, -1, `missing ${endNeedle}`);
  return contents.slice(start, end);
};

test("Web Preview clipboard payload import caps text before parsing or wrapping", () => {
  const webPreview = source();
  const clipboardImport = sliceBetween(
    webPreview,
    "fn import_agent_browser_action_payload_from_clipboard",
    "fn import_agent_browser_action_payload_from_managed_queue",
  );

  assert.match(
    webPreview,
    /const MAX_AGENT_BROWSER_ACTION_PAYLOAD_IMPORT_BYTES: u64 = 256 \* 1024;/,
  );
  assert.match(
    webPreview,
    /fn bounded_agent_browser_clipboard_import_text\(\s*clipboard: &ClipboardItem,\s*\) -> Result<Option<String>, u64>/,
  );
  assert.match(
    webPreview,
    /checked_add\(text\.text\(\)\.len\(\)\)/,
  );
  assert.match(
    webPreview,
    /String::with_capacity\(total_len\)/,
  );
  assert.doesNotMatch(clipboardImport, /\.text\(\)/);

  const bound = clipboardImport.indexOf(
    "Self::bounded_agent_browser_clipboard_import_text(&clipboard)",
  );
  const parse = clipboardImport.indexOf("serde_json::from_str::<Value>(&clipboard_text)");
  const plainTextWrap = clipboardImport.indexOf('"text": clipboard_text');

  assert.ok(bound >= 0, "clipboard text should be explicitly bounded");
  assert.ok(parse > bound, "JSON parsing must happen after clipboard bounding");
  assert.ok(
    plainTextWrap > bound,
    "plain-text wrapping must happen after clipboard bounding",
  );
});

test("Web Preview JSON clipboard imports cap text before parsing", () => {
  const webPreview = source();
  const imports = [
    {
      name: "final validation result import",
      start: "fn import_agent_browser_final_validation_result_from_clipboard",
      end: "fn copy_agent_browser_final_validation_result_import_receipt",
      parse: "serde_json::from_str::<Value>(&clipboard_text)",
    },
    {
      name: "final runtime headroom cleanup-result import",
      start: "fn import_agent_browser_final_runtime_headroom_cleanup_result_from_clipboard",
      end: "fn copy_agent_browser_final_runtime_headroom_cleanup_result",
      parse: "serde_json::from_str::<Value>(&clipboard_text)",
    },
    {
      name: "panel control result import",
      start: "fn import_agent_browser_panel_control_result_from_clipboard",
      end: "fn copy_agent_browser_panel_control_result_import_receipt",
      parse: "serde_json::from_str::<Value>(&clipboard_text)",
    },
  ];

  for (const clipboardImport of imports) {
    const body = sliceBetween(
      webPreview,
      clipboardImport.start,
      clipboardImport.end,
    );
    assert.doesNotMatch(
      body,
      /read_from_clipboard\(\)\.and_then\(\|item\| item\.text\(\)\)/,
      `${clipboardImport.name} should not read unbounded clipboard text`,
    );
    assert.doesNotMatch(
      body,
      /\.text\(\)/,
      `${clipboardImport.name} should not call ClipboardItem::text directly`,
    );

    const read = body.indexOf("cx.read_from_clipboard()");
    const bound = body.indexOf(
      "Self::bounded_agent_browser_clipboard_import_text(&clipboard)",
    );
    const parse = body.indexOf(clipboardImport.parse);

    assert.ok(read >= 0, `${clipboardImport.name} should read the clipboard item`);
    assert.ok(bound > read, `${clipboardImport.name} should bound clipboard text`);
    assert.ok(
      parse > bound,
      `${clipboardImport.name} must parse JSON only after bounding clipboard text`,
    );
  }
});

test("Web Preview managed queue import uses sentinel-byte bounded reads", () => {
  const webPreview = source();

  assert.match(
    webPreview,
    /fn read_agent_browser_action_payload_import_file\(path: &Path\) -> io::Result<String>/,
  );
  assert.match(webPreview, /fs::File::open\(path\)\?/);
  assert.match(
    webPreview,
    /\.take\(MAX_AGENT_BROWSER_ACTION_PAYLOAD_IMPORT_BYTES \+ 1\)/,
  );
  assert.match(webPreview, /read_to_end\(&mut buffer\)/);
  assert.match(
    webPreview,
    /buffer\.len\(\) as u64 > MAX_AGENT_BROWSER_ACTION_PAYLOAD_IMPORT_BYTES/,
  );
  assert.match(webPreview, /String::from_utf8\(buffer\)/);
});

test("Web Preview managed queue import does not call read_to_string directly", () => {
  const webPreview = source();
  const managedQueueImport = sliceBetween(
    webPreview,
    "fn import_agent_browser_action_payload_from_managed_queue",
    "fn copy_agent_browser_action_payload_import_receipt",
  );

  assert.doesNotMatch(managedQueueImport, /read_to_string/);
  assert.match(
    managedQueueImport,
    /Self::read_agent_browser_action_payload_import_file\(&path\)/,
  );
});

test("Web Preview managed queue import parses only after the bounded read succeeds", () => {
  const webPreview = source();
  const managedQueueImport = sliceBetween(
    webPreview,
    "fn import_agent_browser_action_payload_from_managed_queue",
    "fn copy_agent_browser_action_payload_import_receipt",
  );

  const boundedRead = managedQueueImport.indexOf(
    "Self::read_agent_browser_action_payload_import_file(&path)",
  );
  const parse = managedQueueImport.indexOf("serde_json::from_str::<Value>(&contents)");

  assert.ok(boundedRead >= 0, "managed queue import should use bounded reads");
  assert.ok(parse > boundedRead, "JSON parsing must happen after bounded reads");
});
