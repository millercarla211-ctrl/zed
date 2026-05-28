import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const source = readFileSync("crates/agent_ui/src/message_editor.rs", "utf8");

function sourceSlice(startNeedle: string, endNeedle: string): string {
  const start = source.indexOf(startNeedle);
  assert.notEqual(start, -1, `expected ${startNeedle}`);
  const end = source.indexOf(endNeedle, start);
  assert.notEqual(end, -1, `expected ${endNeedle} after ${startNeedle}`);
  return source.slice(start, end);
}

test("message editor caps mention-link paste parsing before scanning text", () => {
  assert.match(
    source,
    /const MAX_MARKDOWN_MENTION_LINK_PASTE_BYTES: usize = 256 \* 1024;/,
  );

  const helper = source.match(
    /fn should_parse_pasted_mention_links\(text: &str\) -> bool \{(?<body>[\s\S]*?)\n\}/,
  );
  assert.ok(helper?.groups?.body, "expected a focused mention-link paste guard");

  const body = helper.groups.body;
  assert.match(body, /text\.len\(\) <= MAX_MARKDOWN_MENTION_LINK_PASTE_BYTES/);
  assert.match(body, /text\.contains\("\[@"\)/);
  assert.ok(
    body.indexOf("text.len()") < body.indexOf('text.contains("[@")'),
    "the size cap must short-circuit before contains scans clipboard text",
  );
});

test("message editor parses pasted mention links only through the guarded path", () => {
  const pasteStart = source.indexOf("    pub fn paste(");
  const pasteEnd = source.indexOf("\n    fn copy(", pasteStart);
  assert.notEqual(pasteStart, -1, "expected MessageEditor::paste");
  assert.notEqual(pasteEnd, -1, "expected MessageEditor::copy after paste");

  const paste = source.slice(pasteStart, pasteEnd);
  assert.match(
    paste,
    /ClipboardEntry::String\(text\) => Some\(text\.text\(\)\)/,
    "clipboard text should be borrowed for the mention-link precheck",
  );
  assert.doesNotMatch(
    paste,
    /ClipboardEntry::String\(text\) => Some\(text\.text\(\)\.to_string\(\)\)/,
    "oversized clipboard text should not be cloned before normal paste",
  );
  assert.match(
    paste,
    /if should_parse_pasted_mention_links\(clipboard_text\) \{/,
  );

  const guardedStart = paste.indexOf(
    "if should_parse_pasted_mention_links(clipboard_text) {",
  );
  const fallbackStart = paste.indexOf(
    "if self.handle_pasted_context(clipboard, window, cx)",
    guardedStart,
  );
  assert.ok(
    guardedStart !== -1 && fallbackStart > guardedStart,
    "mention parsing should sit before the normal paste fallback",
  );

  const guardedPaste = paste.slice(guardedStart, fallbackStart);
  assert.equal(
    [...paste.matchAll(/parse_mention_links/g)].length,
    1,
    "paste should have one mention-link parser call",
  );
  assert.match(guardedPaste, /parse_mention_links\(&inserted_text, path_style\)/);
  assert.ok(
    paste.indexOf("editor.paste_item(clipboard, window, cx)") > fallbackStart,
    "oversized text should continue to the normal editor paste path",
  );
});

test("message editor caps clipboard context entries before materializing them", () => {
  assert.match(
    source,
    /const MAX_MESSAGE_EDITOR_PASTE_CLIPBOARD_ENTRIES: usize = \d+;/,
  );

  const body = sourceSlice(
    "fn limited_pasted_context_entries(clipboard: &ClipboardItem) -> Vec<PastedContextEntry> {",
    "\nasync fn resolve_pasted_context_items(",
  );
  assert.match(
    body,
    /clipboard\s*\.entries\(\)\s*\.iter\(\)\s*\.take\(MAX_MESSAGE_EDITOR_PASTE_CLIPBOARD_ENTRIES\)/,
    "clipboard entries must be capped before they are cloned or collected",
  );

  const handlePastedContext = sourceSlice(
    "    fn handle_pasted_context(",
    "\n    pub fn insert_dragged_files(",
  );
  assert.match(
    handlePastedContext,
    /let entries = limited_pasted_context_entries\(clipboard\);/,
    "pasted context must be materialized only through the limited helper",
  );
  assert.doesNotMatch(
    handlePastedContext,
    /clipboard\.clone\(\)\.into_entries\(\)\.collect::<Vec<_>>\(\)/,
    "pasted context must not clone and collect the whole clipboard item",
  );
});

test("message editor caps external paths before cloning them for paste context", () => {
  assert.match(
    source,
    /const MAX_MESSAGE_EDITOR_PASTE_EXTERNAL_PATHS: usize = \d+;/,
  );

  const body = sourceSlice(
    "fn limited_pasted_context_entries(clipboard: &ClipboardItem) -> Vec<PastedContextEntry> {",
    "\nasync fn resolve_pasted_context_items(",
  );
  assert.match(
    body,
    /let mut external_paths_remaining = MAX_MESSAGE_EDITOR_PASTE_EXTERNAL_PATHS;/,
  );
  assert.match(
    body,
    /paths\s*\.paths\(\)\s*\.iter\(\)\s*\.take\(path_count\)\s*\.cloned\(\)\s*\.map\(PastedContextEntry::ExternalPath\)/,
    "external paths must be capped before each path is cloned",
  );

  const resolver = sourceSlice(
    "async fn resolve_pasted_context_items(",
    "\nfn insert_project_path_as_context(",
  );
  assert.match(
    resolver,
    /entries: Vec<PastedContextEntry>/,
    "the async resolver should receive already-limited context entries",
  );
  assert.match(resolver, /PastedContextEntry::ExternalPath\(path\)/);
  assert.doesNotMatch(
    resolver,
    /ClipboardEntry::ExternalPaths|paths\.paths\(\)\.iter\(\)/,
    "the async resolver must not iterate uncapped ExternalPaths payloads",
  );
});
