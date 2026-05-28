import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/agent_ui/src/config_options.rs";
const source = readFileSync(sourcePath, "utf8").replace(/\r\n/g, "\n");

function sliceBetween(startNeedle: string, endNeedle: string): string {
  const start = source.indexOf(startNeedle);
  assert.notEqual(start, -1, `expected ${startNeedle}`);
  const end = source.indexOf(endNeedle, start + startNeedle.length);
  assert.ok(end > start, `expected ${endNeedle} after ${startNeedle}`);
  return source.slice(start, end);
}

function functionBody(name: string): string {
  const fnIndex = source.indexOf(`fn ${name}`);
  assert.notEqual(fnIndex, -1, `expected ${name}`);

  const bodyStart = source.indexOf("{", fnIndex);
  assert.ok(bodyStart > fnIndex, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(fnIndex, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
}

function assertBefore(
  haystack: string,
  before: string,
  after: string,
  message: string,
) {
  const beforeIndex = haystack.indexOf(before);
  const afterIndex = haystack.indexOf(after);
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("agent config option picker declares named caps for rows, entries, and fuzzy work", () => {
  assert.equal(sourcePath, "crates/agent_ui/src/config_options.rs");
  assert.match(source, /const MAX_CONFIG_OPTION_PICKER_OPTIONS: usize = 4096;/);
  assert.match(
    source,
    /const MAX_CONFIG_OPTION_PICKER_ENTRIES: usize = \(MAX_CONFIG_OPTION_PICKER_OPTIONS \* 3\) \+ 1;/,
  );
  assert.match(source, /const MAX_CONFIG_OPTION_FUZZY_CANDIDATES: usize = 4096;/);
  assert.match(source, /const MAX_CONFIG_OPTION_FUZZY_MATCHES: usize = 100;/);
});

test("agent config option rows are capped before picker entries are materialized", () => {
  const extractOptions = functionBody("extract_options");
  const entries = sliceBetween(
    "fn options_to_picker_entries(",
    "\nasync fn fuzzy_search_options",
  );

  assert.match(
    extractOptions,
    /options\s*\.iter\(\)\s*\.take\(MAX_CONFIG_OPTION_PICKER_OPTIONS\)/,
  );
  assert.match(
    extractOptions,
    /\.flat_map\(\|group\|[\s\S]*?\.take\(MAX_CONFIG_OPTION_PICKER_OPTIONS\)/,
  );
  assert.match(entries, /fn push_config_option_picker_entry/);
  assert.match(entries, /entries\.len\(\) >= MAX_CONFIG_OPTION_PICKER_ENTRIES/);
  const helperStart = entries.indexOf("fn push_config_option_picker_entry");
  assert.notEqual(helperStart, -1, "expected bounded picker-entry push helper");
  assert.doesNotMatch(entries.slice(0, helperStart), /entries\.push\(/);
});

test("agent config option extraction preserves current overflow option after row cap", () => {
  const extractOptions = functionBody("extract_options");
  const preservationHelper = functionBody("preserve_overflow_current_option");

  assert.equal(
    [...extractOptions.matchAll(/let overflow_current_option/g)].length,
    2,
    "ungrouped and grouped options should both look for an overflow current row",
  );
  assert.equal(
    [...extractOptions.matchAll(/preserve_overflow_current_option\(/g)].length,
    2,
    "ungrouped and grouped options should both preserve current overflow rows",
  );
  assertBefore(
    extractOptions,
    ".take(MAX_CONFIG_OPTION_PICKER_OPTIONS)",
    "let overflow_current_option",
    "configured options must be capped before scanning for the current overflow row",
  );
  assert.match(
    extractOptions,
    /options\s*\.iter\(\)\s*\.skip\(MAX_CONFIG_OPTION_PICKER_OPTIONS\)/,
  );
  assert.match(
    extractOptions,
    /groups\s*\.iter\(\)[\s\S]*\.skip\(MAX_CONFIG_OPTION_PICKER_OPTIONS\)/,
  );
  assert.match(extractOptions, /&select\.current_value/);
  assert.match(
    preservationHelper,
    /capped_options\s*\.iter\(\)\s*\.any\(\|option\| &option\.value == current_value\)/,
  );
  assert.match(preservationHelper, /capped_options\.pop\(\);/);
  assert.match(preservationHelper, /capped_options\.push\(option\);/);
});

test("agent config option fuzzy search caps candidates and guards match ids", () => {
  const fuzzy = sliceBetween(
    "async fn fuzzy_search_options(",
    "\nfn find_option_name",
  );

  assertBefore(
    fuzzy,
    ".take(MAX_CONFIG_OPTION_FUZZY_CANDIDATES)",
    "StringMatchCandidate::new(ix, &opt.name)",
    "fuzzy candidate rows must be capped before candidate materialization",
  );
  assert.match(fuzzy, /MAX_CONFIG_OPTION_FUZZY_MATCHES/);
  assert.match(fuzzy, /candidates\s*\.get\(mat\.candidate_id\)/);
  assert.match(fuzzy, /options\.get\(mat\.candidate_id\)/);
  assert.doesNotMatch(fuzzy, /candidates\[mat\.candidate_id\]/);
  assert.doesNotMatch(fuzzy, /options\[mat\.candidate_id\]/);
});

test("agent config option selection is clamped after filtered entries are replaced", () => {
  const updateMatches = sliceBetween(
    "fn update_matches(",
    "\n    fn confirm(",
  );

  assert.match(source, /fn clamp_config_option_selected_index/);
  assertBefore(
    updateMatches,
    "this.delegate.filtered_entries =",
    "clamp_config_option_selected_index(new_index",
    "selection must be clamped against the replacement entries",
  );
  assertBefore(
    updateMatches,
    "clamp_config_option_selected_index(new_index",
    "this.set_selected_index(new_index",
    "clamped selection must be applied before the picker is notified",
  );
});
