import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/font_panel/src/font_panel.rs";

const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const source = productionSource(readFileSync(sourcePath, "utf8"));

function functionBody(sourceText: string, name: string): string {
  const fnIndex = sourceText.search(new RegExp(`fn\\s+${name}\\s*\\(`));
  assert.ok(fnIndex >= 0, `expected ${name}`);

  const bodyStart = sourceText.indexOf("{", fnIndex);
  assert.ok(bodyStart > fnIndex, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < sourceText.length; index += 1) {
    const char = sourceText[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return sourceText.slice(fnIndex, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
}

function assertBefore(
  haystack: string,
  before: string | RegExp,
  after: string | RegExp,
  message: string,
) {
  const beforeIndex =
    typeof before === "string" ? haystack.indexOf(before) : haystack.match(before)?.index ?? -1;
  const afterIndex =
    typeof after === "string" ? haystack.indexOf(after) : haystack.match(after)?.index ?? -1;

  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("font panel declares focused font materialization bounds", () => {
  assert.match(source, /const MAX_CUSTOM_WEB_FONT_NAME_WORDS: usize = 6;/);
  assert.match(source, /const MAX_CUSTOM_WEB_FONT_WORD_CHARS: usize = 24;/);
  assert.match(source, /const MAX_CUSTOM_WEB_FONT_NAME_CHARS: usize = 96;/);
  assert.match(source, /const MAX_FONT_ELEMENT_ID_VALUE_CHARS: usize = 96;/);
  assert.match(source, /const MAX_FONT_PREVIEW_FILE_STEM_CHARS: usize = 96;/);
});

test("custom web font names are bounded before CSS, status, preview, and row materialization", () => {
  const customWebFontName = functionBody(source, "custom_web_font_name");
  const pushCustomWord = functionBody(source, "push_custom_web_font_word");
  const specByName = functionBody(source, "web_font_spec_by_name");

  assert.match(
    customWebFontName,
    /String::with_capacity\(MAX_CUSTOM_WEB_FONT_NAME_CHARS\)/,
  );
  assert.match(customWebFontName, /\.take\(MAX_CUSTOM_WEB_FONT_NAME_WORDS\)/);
  assertBefore(
    customWebFontName,
    "push_custom_web_font_word(&mut name, word, &mut name_chars);",
    "(!name.is_empty()).then_some(name)",
    "custom query names must be bounded before becoming a FontEntry/WebFontSpec name",
  );
  assert.match(pushCustomWord, /MAX_CUSTOM_WEB_FONT_NAME_CHARS/);
  assert.match(pushCustomWord, /MAX_CUSTOM_WEB_FONT_WORD_CHARS/);
  assert.match(pushCustomWord, /to_uppercase\(\)/);
  assertBefore(
    specByName,
    ".or_else(|| custom_web_font_name(name))?",
    "family_query: google_font_family_query(&name)",
    "web font specs must use the bounded custom name before CSS query materialization",
  );
});

test("preview file stems and element IDs compact oversized font names", () => {
  const elementId = functionBody(source, "font_element_id");
  const previewStem = functionBody(source, "font_preview_file_stem");

  assert.match(elementId, /MAX_FONT_ELEMENT_ID_VALUE_CHARS/);
  assert.match(elementId, /stable_text_hash\(id\)/);
  assert.match(elementId, /\.take\(MAX_FONT_ELEMENT_ID_VALUE_CHARS\)/);
  assert.match(previewStem, /MAX_FONT_PREVIEW_FILE_STEM_CHARS/);
  assert.match(previewStem, /stable_text_hash\(font_name\)/);
  assert.match(previewStem, /\.take\(MAX_FONT_PREVIEW_FILE_STEM_CHARS\)/);
});

test("font panel source guard stays scoped to worker-owned files", () => {
  assert.equal(sourcePath, "crates/font_panel/src/font_panel.rs");
  assert.doesNotMatch(sourcePath, /DX\.md|todo\.txt|changelog\.txt/);
  assert.doesNotMatch(
    source,
    /#\[cfg\(test\)\]/,
    "source guard should only inspect production font panel code",
  );
});
