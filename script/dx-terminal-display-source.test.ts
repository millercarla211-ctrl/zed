import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const sliceBetween = (source: string, start: string, end: string) => {
  const startIndex = source.indexOf(start);
  assert.notEqual(startIndex, -1, `missing start marker: ${start}`);
  const endIndex = source.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `missing end marker after ${start}: ${end}`);
  return source.slice(startIndex, endIndex);
};

const assertOrdered = (source: string, first: string, second: string) => {
  const firstIndex = source.indexOf(first);
  assert.notEqual(firstIndex, -1, `missing first marker: ${first}`);
  const secondIndex = source.indexOf(second);
  assert.notEqual(secondIndex, -1, `missing second marker: ${second}`);
  assert.ok(
    firstIndex < secondIndex,
    `expected ${first} before ${second}`,
  );
};

test("terminal display and search materialization has named defensive caps", () => {
  const source = read("crates/terminal/src/terminal.rs");

  for (const constant of [
    "TERMINAL_EVENT_DRAIN_BATCH_LIMIT",
    "TERMINAL_DISPLAY_CELL_LIMIT",
    "TERMINAL_SEARCH_MATCH_LIMIT",
    "TERMINAL_SELECT_MATCH_LIMIT",
    "TERMINAL_INPUT_LOG_ENTRY_LIMIT",
    "TERMINAL_INPUT_LOG_ENTRY_BYTE_LIMIT",
    "TERMINAL_LOGICAL_LINE_RESULT_LIMIT",
    "TERMINAL_LOGICAL_LINE_ROW_LIMIT",
    "TERMINAL_ROW_STRING_CELL_LIMIT",
  ]) {
    assert.match(source, new RegExp(`const ${constant}: usize =`));
  }

  assert.match(source, /log::warn!\(\s*"Terminal event drain batch limit reached/);
  assert.match(source, /log::warn!\(\s*"Terminal display content truncated/);
  assert.match(source, /log::warn!\(\s*"Terminal search matches truncated/);
  assert.match(source, /log::warn!\(\s*"Terminal selected search matches truncated/);
  assert.match(source, /log::warn!\(\s*"Terminal input log truncated/);
  assert.match(source, /log::warn!\(\s*"Terminal logical line truncated/);
  assert.match(source, /log::warn!\(\s*"Terminal row string truncated/);
  assert.match(source, /log::warn!\(\s*"Terminal hyperlink cell lookup skipped/);
});

test("terminal display vectors are capped before rows cells and matches are pushed", () => {
  const source = read("crates/terminal/src/terminal.rs");

  const eventDrain = sliceBetween(
    source,
    "let mut events = Vec::new();",
    "if events.is_empty() && !wakeup",
  );
  assertOrdered(
    eventDrain,
    "events.len() >= TERMINAL_EVENT_DRAIN_BATCH_LIMIT",
    "events.push(event)",
  );

  const makeContent = sliceBetween(
    source,
    "fn make_content(term: &Term<ZedListener>",
    "pub fn get_content(&self)",
  );
  assert.match(makeContent, /Vec::with_capacity\(estimated_size\.min\(TERMINAL_DISPLAY_CELL_LIMIT\)\)/);
  assertOrdered(
    makeContent,
    "cells.len() >= TERMINAL_DISPLAY_CELL_LIMIT",
    "cells.push(IndexedCell",
  );
  assert.doesNotMatch(makeContent, /cells\.extend\(/);

  const findMatches = sliceBetween(
    source,
    "pub fn find_matches(",
    "pub fn working_directory(&self)",
  );
  assertOrdered(
    findMatches,
    "matches.len() >= TERMINAL_SEARCH_MATCH_LIMIT",
    "matches.push(search_match)",
  );
  assert.doesNotMatch(findMatches, /all_search_matches\(&term, &mut searcher\)\.collect/);

  const selectMatches = sliceBetween(
    source,
    "pub fn select_matches(",
    "pub fn select_all(&mut self)",
  );
  assertOrdered(
    selectMatches,
    "matches_to_select.len() >= TERMINAL_SELECT_MATCH_LIMIT",
    "matches_to_select.push",
  );
  assert.doesNotMatch(selectMatches, /collect::<Vec/);

  const inputLog = sliceBetween(
    source,
    "fn record_input_log_entry(&mut self",
    "pub fn take_input_log(&mut self)",
  );
  assertOrdered(
    inputLog,
    "self.input_log.len() >= TERMINAL_INPUT_LOG_ENTRY_LIMIT",
    "self.input_log.push",
  );

  const logicalLines = sliceBetween(
    source,
    "pub fn last_n_non_empty_lines(&self",
    "fn process_line(&self",
  );
  assert.match(logicalLines, /let requested_lines = n\.min\(TERMINAL_LOGICAL_LINE_RESULT_LIMIT\);/);
  assert.match(logicalLines, /TERMINAL_LOGICAL_LINE_ROW_LIMIT/);
});

test("terminal hyperlink materialization bounds byte and regex vectors", () => {
  const source = read("crates/terminal/src/terminal_hyperlinks.rs");

  for (const constant of [
    "TERMINAL_HYPERLINK_LINE_CELL_LIMIT",
    "TERMINAL_HYPERLINK_LINE_BYTE_LIMIT",
    "TERMINAL_HYPERLINK_REGEX_MATCH_LIMIT",
    "TERMINAL_HYPERLINK_OSC8_PATH_BYTE_LIMIT",
  ]) {
    assert.match(source, new RegExp(`const ${constant}: usize =`));
  }

  assert.match(source, /log::warn!\(\s*"Skipping terminal hyperlink search/);
  assert.match(source, /log::warn!\(\s*"Skipping terminal hyperlink OSC8 path/);
  assert.match(source, /log::warn!\(\s*"Terminal hyperlink regex match limit reached/);
  assert.doesNotMatch(source, /collect::<Vec<u8>>/);

  const osc8 = sliceBetween(
    source,
    "fn try_osc8_url_to_path",
    "fn sanitize_url_punctuation",
  );
  assertOrdered(
    osc8,
    "bytes.len() >= TERMINAL_HYPERLINK_OSC8_PATH_BYTE_LIMIT",
    "bytes.push(byte)",
  );

  const pathMatch = sliceBetween(
    source,
    "fn path_match<T>(",
    "#[cfg(test)]",
  );
  assertOrdered(
    pathMatch,
    "line_cell_count(term, line_start, line_end)",
    "String::with_capacity",
  );
  assertOrdered(
    pathMatch,
    "captures_seen >= TERMINAL_HYPERLINK_REGEX_MATCH_LIMIT",
    "captures_seen += 1",
  );
});
