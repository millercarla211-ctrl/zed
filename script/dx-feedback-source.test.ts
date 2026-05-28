import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8").replace(/\r\n/g, "\n");
const productionSource = (source: string) =>
  source.split(/\n#\[cfg\(test\)\]\nmod tests\s*\{/)[0] ?? source;

const sourcePath = "crates/feedback/src/feedback.rs";
const source = productionSource(read(sourcePath));

function functionBody(sourceText: string, name: string): string {
  const fnIndex = sourceText.indexOf(`fn ${name}`);
  assert.notEqual(fnIndex, -1, `expected ${name}`);

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
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("feedback extension report declares explicit materialization caps", () => {
  assert.match(source, /const MAX_INSTALLED_EXTENSIONS_FOR_BUG_REPORT: usize = \d+;/);
  assert.match(source, /const MAX_INSTALLED_EXTENSION_FIELD_CHARS: usize = \d+;/);
  assert.match(
    source,
    /const MAX_INSTALLED_EXTENSIONS_PROMPT_CHARS: usize = (?:\d+|\d+ \* \d+);/,
  );
});

test("feedback extension report caps rows before clipboard materialization", () => {
  const formatter = functionBody(source, "format_installed_extensions_for_clipboard");
  const lineFormatter = functionBody(source, "format_installed_extension_line");

  assert.match(formatter, /let extension_count = store\.extension_index\.extensions\.len\(\);/);
  assert.match(
    formatter,
    /let line_limit = extension_count\.min\(MAX_INSTALLED_EXTENSIONS_FOR_BUG_REPORT\);/,
  );
  assert.match(formatter, /Vec::with_capacity\(line_limit\)/);
  assert.doesNotMatch(
    formatter,
    /Vec::with_capacity\(store\.extension_index\.extensions\.len\(\)\)/,
    "the full extension index size must not drive bug-report allocation",
  );
  assertBefore(
    formatter,
    ".take(MAX_INSTALLED_EXTENSIONS_FOR_BUG_REPORT)",
    "format_installed_extension_line",
    "installed extensions must be capped before report lines are formatted",
  );
  assert.match(
    formatter,
    /format_installed_extension_overflow_notice\(\s*extension_count - line_limit,\s*\)/,
  );

  assert.match(
    lineFormatter,
    /installed_extension_report_field\(entry\.manifest\.name\.as_str\(\)\)/,
  );
  assert.match(lineFormatter, /installed_extension_report_field\(extension_id\.as_ref\(\)\)/);
  assert.match(
    lineFormatter,
    /installed_extension_report_field\(entry\.manifest\.version\.as_ref\(\)\)/,
  );
});

test("feedback extension report prompt displays a bounded preview", () => {
  assert.match(
    source,
    /fn installed_extensions_prompt_text\(clipboard_text: &str\) -> Cow<'_, str>/,
  );
  assert.match(source, /let prompt_text = installed_extensions_prompt_text\(&clipboard_text\);/);
  assert.match(source, /Some\(prompt_text\.as_ref\(\)\)/);
  assert.doesNotMatch(
    source,
    /Some\(&clipboard_text\)/,
    "the copied extension report should not be used as an unbounded prompt body",
  );
  assertBefore(
    source,
    "let prompt_text = installed_extensions_prompt_text(&clipboard_text);",
    "Some(prompt_text.as_ref())",
    "extension prompt body must be bounded before prompt materialization",
  );
});

test("feedback source guard stays scoped to worker-owned source", () => {
  assert.equal(sourcePath, "crates/feedback/src/feedback.rs");
  assert.doesNotMatch(sourcePath, /DX\.md|todo\.txt|changelog\.txt/);
});
