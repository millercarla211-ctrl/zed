import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const functionBody = (source: string, name: string) => {
  const start = source.search(new RegExp(`fn\\s+${name}(?:<[^>]+>)?\\s*\\(`));
  assert.ok(start >= 0, `expected ${name}`);

  const bodyStart = source.indexOf("{", start);
  assert.ok(bodyStart > start, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(start, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
};

const sliceBetween = (source: string, start: string, end: string) => {
  const startIndex = source.indexOf(start);
  assert.notEqual(startIndex, -1, `missing start marker: ${start}`);
  const endIndex = source.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `missing end marker after ${start}: ${end}`);
  return source.slice(startIndex, endIndex);
};

const assertBefore = ({
  body,
  before,
  after,
  message,
}: {
  body: string;
  before: string | RegExp;
  after: string | RegExp;
  message: string;
}) => {
  const beforeIndex =
    typeof before === "string" ? body.indexOf(before) : body.match(before)?.index ?? -1;
  const afterIndex =
    typeof after === "string" ? body.indexOf(after) : body.match(after)?.index ?? -1;
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("project search match labels saturate stale active indexes before display", () => {
  const source = read("crates/search/src/project_search.rs");

  assert.match(
    source,
    /fn project_search_match_text\(\s+active_match_index: Option<usize>,\s+match_quantity: usize,\s+limit_reached: bool,\s+\) -> String/,
  );

  const helper = functionBody(source, "project_search_match_text");
  assert.match(helper, /match_quantity\.checked_sub\(1\)/);
  assert.match(helper, /index\.min\(last_match_index\)\.saturating_add\(1\)/);
  assert.doesNotMatch(helper, /index \+ 1/);
  assertBefore({
    body: helper,
    before: "match_quantity.checked_sub(1)",
    after: "format!(\"{index}/{match_quantity",
    message: "match-count labels must prove a non-empty count before formatting",
  });

  const projectSearchBarRender = sliceBetween(
    source,
    "impl Render for ProjectSearchBar",
    "let query_focus = search.query_editor.focus_handle(cx);",
  );
  assert.match(
    projectSearchBarRender,
    /let match_text = project_search_match_text\(\s+search\.active_match_index,\s+project_search\.match_ranges\.len\(\),\s+limit_reached,\s+\);/,
  );
});

test("project search navigation clamps stale active indexes before indexing ranges", () => {
  const source = read("crates/search/src/project_search.rs");
  const selectMatch = functionBody(source, "select_match");

  assert.match(
    selectMatch,
    /let Some\(last_match_index\) = match_ranges\.len\(\)\.checked_sub\(1\) else \{/,
  );
  assertBefore({
    body: selectMatch,
    before: "self.active_match_index = None;",
    after: "editor.match_index_for_direction",
    message: "empty result sets must clear the stale active index before editor navigation",
  });
  assertBefore({
    body: selectMatch,
    before: "let index = index.min(last_match_index);",
    after: "editor.match_index_for_direction",
    message: "stale active index must be clamped before editor navigation",
  });
  assertBefore({
    body: selectMatch,
    before: "direction == Direction::Next && index == last_match_index",
    after: "editor.match_index_for_direction",
    message: "non-wrapping next navigation must use the bounded last index",
  });
  assert.doesNotMatch(selectMatch, /index \+ 1 >= match_ranges\.len\(\)/);
});

test("project search navigation guards stale editor indexes after navigation", () => {
  const source = read("crates/search/src/project_search.rs");
  const selectMatch = functionBody(source, "select_match");

  assert.doesNotMatch(selectMatch, /match_ranges\s*\[\s*new_index\s*\]/);
  assert.match(
    selectMatch,
    /let Some\(range_to_select\) = match_ranges\.get\(new_index\)\.cloned\(\) else \{/,
  );
  assertBefore({
    body: selectMatch,
    before: "match_ranges.get(new_index).cloned()",
    after: "editor.range_for_match(&range_to_select)",
    message: "editor-provided match indexes must be guarded before selecting a range",
  });
});

test("project search field cycling guards stale focus indexes before focusing views", () => {
  const source = read("crates/search/src/project_search.rs");
  const cycleField = functionBody(source, "cycle_field");

  assert.doesNotMatch(cycleField, /views\s*\[\s*new_index\s*\]/);
  assert.match(
    cycleField,
    /let Some\(next_focus_handle\) = views\.get\(new_index\) else \{/,
  );
  assertBefore({
    body: cycleField,
    before: "views.get(new_index)",
    after: "window.focus(next_focus_handle, cx);",
    message: "field cycling must guard the computed focus index before focusing",
  });
});

test("buffer search field cycling guards stale focus indexes before focusing handles", () => {
  const source = read("crates/search/src/buffer_search.rs");
  const cycleField = functionBody(source, "cycle_field");

  assert.doesNotMatch(cycleField, /handles\s*\[\s*new_index\s*\]/);
  assert.match(
    cycleField,
    /let Some\(next_focus_handle\) = handles\.get\(new_index\) else \{/,
  );
  assertBefore({
    body: cycleField,
    before: "handles.get(new_index)",
    after: "self.focus(next_focus_handle, window, cx);",
    message: "field cycling must guard the computed focus index before focusing",
  });
});
