import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const source = readFileSync("crates/agent_ui/src/message_editor.rs", "utf8");

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
