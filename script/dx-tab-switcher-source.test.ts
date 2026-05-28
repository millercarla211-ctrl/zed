import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const source = readFileSync("crates/tab_switcher/src/tab_switcher.rs", "utf8")
  .replace(/\r\n/g, "\n");

const functionBody = (bodySource: string, name: string) => {
  const start = bodySource.indexOf(`fn ${name}(`);
  assert.ok(start >= 0, `expected ${name}`);

  const bodyStart = bodySource.indexOf("{", start);
  assert.ok(bodyStart > start, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < bodySource.length; index += 1) {
    const char = bodySource[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return bodySource.slice(start, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
};

const indexOfPattern = (body: string, pattern: string | RegExp) => {
  if (typeof pattern === "string") {
    return body.indexOf(pattern);
  }
  return body.match(pattern)?.index ?? -1;
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
  const beforeIndex = indexOfPattern(body, before);
  const afterIndex = indexOfPattern(body, after);
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("all-pane tab switcher bounds tab collection and fuzzy materialization", () => {
  assert.match(source, /const MAX_ALL_PANE_TAB_ROWS: usize = 10_000;/);
  assert.match(
    source,
    /const MAX_ALL_PANE_FUZZY_ROWS: usize = MAX_ALL_PANE_TAB_ROWS;/,
  );
  assert.match(source, /const MAX_ALL_PANE_FUZZY_MATCHES: usize = 10_000;/);
  assert.match(source, /const MAX_ALL_PANE_FUZZY_LABEL_CHARS: usize = 512;/);

  const updateAllPaneMatches = functionBody(source, "update_all_pane_matches");
  assertBefore({
    body: updateAllPaneMatches,
    before: "let remaining = MAX_ALL_PANE_TAB_ROWS.saturating_sub(all_items.len());",
    after: "let pane = pane_handle.read(cx);",
    message: "all-pane collection must compute the remaining cap before reading pane items",
  });
  assertBefore({
    body: updateAllPaneMatches,
    before: ".take(remaining)",
    after: ".collect();",
    message: "pane item cloning must be bounded before collection",
  });
  assertBefore({
    body: updateAllPaneMatches,
    before: ".take(MAX_ALL_PANE_FUZZY_ROWS)",
    after: "StringMatchCandidate::new(ix, bounded_tab_match_label(tab_match, cx))",
    message: "fuzzy candidates must be row-capped before label materialization",
  });
  assertBefore({
    body: updateAllPaneMatches,
    before: "bounded_tab_match_label(tab_match, cx)",
    after: "fuzzy_nucleo::match_strings(",
    message: "candidate labels must be bounded before fuzzy matching starts",
  });
  assert.match(updateAllPaneMatches, /MAX_ALL_PANE_FUZZY_MATCHES/);
  assert.match(
    updateAllPaneMatches,
    /\.filter_map\(\|m\| all_items\.get\(m\.candidate_id\)\.cloned\(\)\)/,
    "fuzzy result mapping must not index all_items directly",
  );
  assert.doesNotMatch(updateAllPaneMatches, /all_items\[m\.candidate_id\]/);

  const labelHelper = functionBody(source, "bounded_tab_match_label");
  assert.match(labelHelper, /MAX_ALL_PANE_FUZZY_LABEL_CHARS/);
  assert.match(labelHelper, /\.char_indices\(\)/);
  assert.match(labelHelper, /label\[..truncate_at\]\.to_string\(\)/);
  assert.match(labelHelper, /bounded\.push_str\("\.\.\."\)/);
});

test("tab switcher clamps stale selected indices before preview activation", () => {
  const clampHelper = functionBody(source, "clamped_selected_index");
  assert.match(clampHelper, /if self\.matches\.is_empty\(\)\s*\{\s*0\s*\}/);
  assert.match(clampHelper, /ix\.min\(self\.matches\.len\(\) - 1\)/);

  const setter = functionBody(source, "set_selected_index");
  assertBefore({
    body: setter,
    before: "let selected_index = self.clamped_selected_index(ix);",
    after: "self.selected_index = selected_index;",
    message: "selection requests must be clamped before storing delegate state",
  });
  assertBefore({
    body: setter,
    before: "self.selected_index = selected_index;",
    after: "self.matches.get(selected_index)",
    message: "preview activation must look up the clamped selected index",
  });
  assert.doesNotMatch(setter, /self\.matches\.get\(self\.selected_index\(\)\)/);

  const computeSelectedIndex = functionBody(source, "compute_selected_index");
  assert.match(
    computeSelectedIndex,
    /return self\.clamped_selected_index\(self\.selected_index\);/,
    "preserving a stale selected index must clamp to the current match list",
  );
  assert.match(
    computeSelectedIndex,
    /let item_index = self\.clamped_selected_index\(self\.matches\.len\(\) - 1\);/,
  );
  assert.match(
    computeSelectedIndex,
    /let item_index = self\.clamped_selected_index\(1\);/,
  );
});
