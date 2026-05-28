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

const assertOrdered = (
  body: string,
  firstNeedle: string,
  secondNeedle: string,
  message: string,
) => {
  const first = body.indexOf(firstNeedle);
  const second = body.indexOf(secondNeedle);

  assert.ok(first >= 0, `${message}: missing ${firstNeedle}`);
  assert.ok(second >= 0, `${message}: missing ${secondNeedle}`);
  assert.ok(second > first, message);
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

test("Web Preview IPC messages are byte-capped before queueing and parsing", () => {
  const webPreview = source();
  const windowsVisualWebView = read("crates/web_preview/src/windows_visual_webview.rs");

  assert.match(
    webPreview,
    /const MAX_WEB_PREVIEW_IPC_MESSAGE_BYTES: usize = 1024 \* 1024;/,
  );
  assert.match(
    webPreview,
    /fn ensure_web_preview_ipc_message_within_byte_limit\(message: &str\) -> Result<\(\)>/,
  );
  assert.match(
    webPreview,
    /message\.len\(\) > MAX_WEB_PREVIEW_IPC_MESSAGE_BYTES/,
  );
  assert.match(webPreview, /IpcMessageRejected\(String\)/);
  assert.ok(
    webPreview.includes(
      "pub(crate) fn push_browser_ipc_event(event_queue: &Arc<Mutex<Vec<BrowserEvent>>>, message: String)",
    ),
    "expected raw IPC queue helper",
  );

  const rawIpcPush = sliceBetween(
    webPreview,
    "pub(crate) fn push_browser_ipc_event",
    "\n#[cfg(target_os = \"windows\")]",
  );
  assertOrdered(
    rawIpcPush,
    "WebPreviewView::ensure_web_preview_ipc_message_within_byte_limit(&message)",
    "queue.push(BrowserEvent::IpcMessage(message))",
    "raw IPC messages should be byte-capped before the browser event queue stores them",
  );
  assertOrdered(
    rawIpcPush,
    "WebPreviewView::ensure_deferred_ipc_queue_has_capacity",
    "queue.push(BrowserEvent::IpcMessage(message))",
    "raw IPC messages should be count-capped before the browser event queue stores them",
  );
  assert.match(rawIpcPush, /queued_browser_ipc_message_count\(&queue\)/);
  assert.match(rawIpcPush, /push_browser_ipc_rejection_once/);

  const applyEvents = sliceBetween(
    webPreview,
    "fn apply_browser_events",
    "fn ensure_web_preview_ipc_message_within_byte_limit",
  );
  assert.doesNotMatch(
    applyEvents,
    /deferred_ipc_messages\.push\(message\)/,
    "IPC messages should not be queued without the ingestion helper",
  );
  assert.match(
    applyEvents,
    /self\.queue_deferred_ipc_message\(message, cx\);/,
    "IPC events should use the guarded queue helper",
  );
  assert.match(
    applyEvents,
    /BrowserEvent::IpcMessageRejected\(message\) => \{\s+self\.report_action_error_message\(message, cx\);/s,
    "raw IPC rejections should surface visibly when events are applied",
  );

  assert.doesNotMatch(
    webPreview,
    /push_browser_event\(\s*&event_queue,\s*BrowserEvent::IpcMessage/,
    "generic raw event push should not be used for IPC messages",
  );
  assert.doesNotMatch(
    windowsVisualWebView,
    /push_browser_event\(\s*&event_queue,\s*BrowserEvent::IpcMessage/,
    "WebView2 IPC should use the raw IPC boundary helper",
  );
  assert.match(
    webPreview,
    /push_browser_ipc_event\(&event_queue, request\.body\(\)\.to_string\(\)\);/,
    "wry IPC should use the raw IPC boundary helper",
  );
  assert.match(
    windowsVisualWebView,
    /push_browser_ipc_event\(&event_queue, take_pwstr\(message\)\);/,
    "WebView2 IPC should use the raw IPC boundary helper",
  );

  const queueHelper = sliceBetween(
    webPreview,
    "fn queue_deferred_ipc_message",
    "fn flush_deferred_ipc",
  );
  assertOrdered(
    queueHelper,
    "Self::ensure_web_preview_ipc_message_within_byte_limit(&message)",
    "self.deferred_ipc_messages.push(message)",
    "IPC messages should be byte-capped before queueing",
  );

  const handler = sliceBetween(
    webPreview,
    "fn handle_ipc_message",
    'match kind {',
  );
  assertOrdered(
    handler,
    "Self::ensure_web_preview_ipc_message_within_byte_limit(message)?",
    "serde_json::from_str(message)",
    "IPC messages should be byte-capped before serde parsing",
  );
});

test("Web Preview deferred IPC queue has an explicit length cap", () => {
  const webPreview = source();

  assert.match(
    webPreview,
    /const MAX_DEFERRED_WEB_PREVIEW_IPC_MESSAGES: usize = 256;/,
  );
  assert.match(
    webPreview,
    /fn ensure_deferred_ipc_queue_has_capacity\(current_len: usize\) -> Result<\(\)>/,
  );
  assert.match(
    webPreview,
    /current_len >= MAX_DEFERRED_WEB_PREVIEW_IPC_MESSAGES/,
  );

  const queueHelper = sliceBetween(
    webPreview,
    "fn queue_deferred_ipc_message",
    "fn flush_deferred_ipc",
  );
  assertOrdered(
    queueHelper,
    "Self::ensure_deferred_ipc_queue_has_capacity(self.deferred_ipc_messages.len())",
    "self.deferred_ipc_messages.push(message)",
    "deferred IPC queue capacity should be checked before queueing",
  );
  assert.match(
    queueHelper,
    /self\.report_action_error\("Web Preview bridge message rejected", error, cx\);/,
    "rejected IPC messages should surface an error instead of being dropped silently",
  );
});

test("Web Preview file payload reads use sentinel-byte bounded helpers", () => {
  const webPreview = source();

  assert.match(
    webPreview,
    /const MAX_WEB_PREVIEW_JSON_PAYLOAD_BYTES: u64 = 16 \* 1024 \* 1024;/,
  );
  assert.match(
    webPreview,
    /const MAX_WEB_PREVIEW_SCREENSHOT_PNG_BYTES: u64 = 64 \* 1024 \* 1024;/,
  );
  assert.match(
    webPreview,
    /fn read_sentinel_bounded_file\(\s*path: &Path,\s*max_bytes: u64,\s*description: &str,\s*\) -> io::Result<Vec<u8>>/,
  );
  assert.match(webPreview, /fs::File::open\(path\)\?/);
  assert.match(webPreview, /\.take\(max_bytes \+ 1\)/);
  assert.match(webPreview, /read_to_end\(&mut buffer\)/);
  assert.match(webPreview, /buffer\.len\(\) as u64 > max_bytes/);
  assert.match(webPreview, /io::ErrorKind::InvalidData/);
  assert.match(
    webPreview,
    /fn read_web_preview_json_payload_file\(path: &Path\) -> io::Result<Vec<u8>>/,
  );
  assert.match(
    webPreview,
    /fn read_web_preview_screenshot_png_file\(path: &Path\) -> io::Result<Vec<u8>>/,
  );
});

test("Web Preview target JSON files are bounded before serde parsing", () => {
  const webPreview = source();
  const jsonTargets = [
    {
      name: "final validation durable evidence",
      start: "fn agent_browser_final_validation_result_durable_evidence",
      end: "fn agent_browser_panel_control_result_latest_paths",
    },
    {
      name: "latest panel control result",
      start: "fn latest_durable_agent_browser_panel_control_result",
      end: "fn agent_browser_panel_control_result_durable_evidence",
    },
    {
      name: "panel control durable evidence",
      start: "fn agent_browser_panel_control_result_durable_evidence",
      end: "fn agent_browser_final_runtime_headroom_cleanup_result_latest_paths",
    },
    {
      name: "latest runtime headroom cleanup result",
      start: "fn latest_durable_agent_browser_final_runtime_headroom_cleanup_result",
      end: "fn agent_browser_final_runtime_headroom_cleanup_result_durable_evidence",
    },
    {
      name: "runtime headroom cleanup durable evidence",
      start: "fn agent_browser_final_runtime_headroom_cleanup_result_durable_evidence",
      end: "fn agent_browser_final_runtime_headroom_cleanup_result_gate_from_template",
    },
    {
      name: "managed Chrome execution summary",
      start: "fn managed_chrome_execution_file_read_summary",
      end: "fn managed_chrome_screenshot_summary",
    },
    {
      name: "PC-use status summary",
      start: "fn pc_use_status_file_read_summary",
      end: "fn pc_use_status_proof_summary",
    },
    {
      name: "adapter manifest readiness",
      start: "fn adapter_manifest_ready",
      end: "fn json_file_schema",
    },
    {
      name: "JSON schema probe",
      start: "fn json_file_schema",
      end: "fn bootstrap_next_actions",
    },
    {
      name: "bookmark loader",
      start: "fn load_bookmarks",
      end: "fn scan_local_extensions",
    },
    {
      name: "extension manifest scanner",
      start: "fn scan_chromium_extensions",
      end: "fn scan_firefox_extensions",
      read: "read_local_browser_extension_manifest",
    },
  ];

  for (const target of jsonTargets) {
    const body = sliceBetween(webPreview, target.start, target.end);
    const boundedRead = target.read ?? "read_web_preview_json_payload_file";

    assert.doesNotMatch(
      body,
      /fs::read\(/,
      `${target.name} should not use unbounded fs::read before JSON parse`,
    );
    assertOrdered(
      body,
      boundedRead,
      "serde_json::from_slice",
      `${target.name} should bound the file before serde parsing`,
    );
  }
});

test("Web Preview local extension discovery caps profile and extension directory walks", () => {
  const webPreview = source();

  assert.match(
    webPreview,
    /const MAX_LOCAL_BROWSER_DISCOVERED_EXTENSIONS: usize = 512;/,
  );
  assert.match(
    webPreview,
    /const MAX_LOCAL_BROWSER_EXTENSION_DIRS: usize = 512;/,
  );
  assert.match(
    webPreview,
    /const MAX_LOCAL_BROWSER_EXTENSION_VERSION_DIRS: usize = 64;/,
  );
  assert.match(
    webPreview,
    /const MAX_LOCAL_BROWSER_FIREFOX_PROFILE_DIRS: usize = 64;/,
  );
  assert.match(
    webPreview,
    /const MAX_LOCAL_BROWSER_FIREFOX_EXTENSIONS_PER_PROFILE: usize = 256;/,
  );

  const chromiumScanner = sliceBetween(
    webPreview,
    "fn scan_chromium_extensions",
    "fn latest_chromium_extension_version_dir",
  );
  assert.match(
    chromiumScanner,
    /fs::read_dir\(root\)\?\s*\.take\(MAX_LOCAL_BROWSER_EXTENSION_DIRS\)/,
    "Chromium extension root scans should be item-capped",
  );
  assert.match(
    chromiumScanner,
    /extensions\.len\(\) >= MAX_LOCAL_BROWSER_DISCOVERED_EXTENSIONS/,
    "Chromium extension discovery should stop at the global result cap",
  );
  assert.match(
    chromiumScanner,
    /latest_chromium_extension_version_dir\(&extension_path\)\?/,
    "Chromium extension scanner should delegate version selection to the capped helper",
  );

  const latestVersionHelper = sliceBetween(
    webPreview,
    "fn latest_chromium_extension_version_dir",
    "fn scan_firefox_extensions",
  );
  assert.match(
    latestVersionHelper,
    /fs::read_dir\(extension_path\)\?\s*\.take\(MAX_LOCAL_BROWSER_EXTENSION_VERSION_DIRS\)/,
    "Chromium extension version scans should be item-capped",
  );
  assert.doesNotMatch(
    latestVersionHelper,
    /collect::<Vec/,
    "Chromium extension version selection should not collect an unbounded directory list",
  );

  const firefoxScanner = sliceBetween(
    webPreview,
    "fn scan_firefox_extensions",
    "#[cfg(target_os = \"macos\")]",
  );
  assert.match(
    firefoxScanner,
    /fs::read_dir\(root\)\?\s*\.take\(MAX_LOCAL_BROWSER_FIREFOX_PROFILE_DIRS\)/,
    "Firefox profile scans should be item-capped",
  );
  assert.match(
    firefoxScanner,
    /fs::read_dir\(&extensions_dir\)\?\s*\.take\(MAX_LOCAL_BROWSER_FIREFOX_EXTENSIONS_PER_PROFILE\)/,
    "Firefox extension scans should be item-capped per profile",
  );
  assert.match(
    firefoxScanner,
    /extensions\.len\(\) >= MAX_LOCAL_BROWSER_DISCOVERED_EXTENSIONS/,
    "Firefox extension discovery should stop at the global result cap",
  );
});

test("Web Preview local extension manifests use a small bounded read before serde parsing", () => {
  const webPreview = source();

  assert.match(
    webPreview,
    /const MAX_LOCAL_BROWSER_EXTENSION_MANIFEST_BYTES: u64 = 512 \* 1024;/,
  );

  const manifestReader = sliceBetween(
    webPreview,
    "fn read_local_browser_extension_manifest",
    "fn scan_local_extensions",
  );
  assert.match(
    manifestReader,
    /read_sentinel_bounded_file\(\s*path,\s*MAX_LOCAL_BROWSER_EXTENSION_MANIFEST_BYTES,\s*"local browser extension manifest",\s*\)/s,
    "extension manifests should use a manifest-specific sentinel-byte cap",
  );

  const chromiumScanner = sliceBetween(
    webPreview,
    "fn scan_chromium_extensions",
    "fn scan_firefox_extensions",
  );
  assert.doesNotMatch(
    chromiumScanner,
    /read_web_preview_json_payload_file\(&manifest_path\)/,
    "extension manifests should not use the broad Web Preview JSON payload cap",
  );
  assertOrdered(
    chromiumScanner,
    "read_local_browser_extension_manifest(&manifest_path)",
    "serde_json::from_slice",
    "extension manifests should be read through the smaller manifest cap before serde parsing",
  );
});

test("Web Preview screenshot files are bounded before image or attachment use", () => {
  const webPreview = source();

  const pngBytes = sliceBetween(
    webPreview,
    "fn capture_screenshot_png_bytes",
    "fn capture_screenshot_payload",
  );
  assert.doesNotMatch(
    pngBytes,
    /fs::read\(/,
    "capture_screenshot_png_bytes should not use unbounded fs::read",
  );
  assert.match(
    pngBytes,
    /read_web_preview_screenshot_png_file\(&path\)/,
    "capture_screenshot_png_bytes should use the bounded PNG helper",
  );

  const payload = sliceBetween(
    webPreview,
    "fn capture_screenshot_payload",
    "fn render_webview_body",
  );
  assert.doesNotMatch(
    payload,
    /fs::read\(/,
    "capture_screenshot_payload should not use unbounded fs::read",
  );
  assertOrdered(
    payload,
    "read_web_preview_screenshot_png_file(&path)",
    "GpuiImage::from_bytes",
    "capture_screenshot_payload should bound PNG bytes before creating the image",
  );
  assertOrdered(
    payload,
    "read_web_preview_screenshot_png_file(&path)",
    "screenshot_agent_blocks",
    "capture_screenshot_payload should bound PNG bytes before attachment encoding",
  );
});
