import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const functionBody = (source: string, name: string): string => {
  const signature = new RegExp(
    `\\n(?:    )?(?:pub\\(crate\\)\\s+|pub\\s+)?(?:async\\s+)?fn ${name}\\(`,
  );
  const match = signature.exec(source);
  assert.ok(match?.index !== undefined, `expected function ${name}`);

  const start = match.index + 1;
  const openBrace = source.indexOf("{", start);
  assert.ok(openBrace > start, `expected ${name} to have a body`);

  let depth = 0;
  for (let index = openBrace; index < source.length; index++) {
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

const assertBefore = (
  haystack: string,
  before: string | RegExp,
  after: string | RegExp,
  message: string,
) => {
  const find = (pattern: string | RegExp) => {
    if (typeof pattern === "string") {
      return haystack.indexOf(pattern);
    }
    return haystack.match(pattern)?.index ?? -1;
  };

  const beforeIndex = find(before);
  const afterIndex = find(after);
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("terminal panel persistence restore has named caps before serde and join fanout", () => {
  const persistence = read("crates/terminal_view/src/persistence.rs");
  const panel = read("crates/terminal_view/src/terminal_panel.rs");

  for (const pattern of [
    /pub\(crate\) const MAX_SERIALIZED_TERMINAL_PANEL_JSON_BYTES: usize = 2 \* 1024 \* 1024;/,
    /const MAX_RESTORED_TERMINAL_PANEL_ITEMS_TOTAL: usize = 2_048;/,
    /const MAX_RESTORED_TERMINAL_ITEMS_PER_PANE: usize = 512;/,
    /const MAX_RESTORED_TERMINAL_PANEL_NODES: usize = 2_048;/,
    /const MAX_RESTORED_TERMINAL_GROUP_CHILDREN: usize = 256;/,
    /const MAX_RESTORED_TERMINAL_GROUP_FLEXES: usize = 256;/,
  ]) {
    assert.match(persistence, pattern);
  }

  const parse = functionBody(
    persistence,
    "deserialize_serialized_terminal_panel_json",
  );
  assertBefore(
    parse,
    "panel_json.len() > MAX_SERIALIZED_TERMINAL_PANEL_JSON_BYTES",
    "serde_json::from_str::<SerializedTerminalPanel>(panel_json)",
    "persisted terminal panel JSON must be byte-capped before serde materializes it",
  );
  assert.match(parse, /sanitize_serialized_terminal_panel\(&mut serialized_panel\)/);
  assert.match(parse, /log::warn!\(/);

  const load = functionBody(panel, "load");
  assert.match(load, /deserialize_serialized_terminal_panel_json\(&panel\)/);
  assert.doesNotMatch(
    load,
    /serde_json::from_str::<SerializedTerminalPanel>/,
    "terminal panel load must not directly serde persisted JSON",
  );

  const restoreViews = functionBody(persistence, "deserialize_terminal_views");
  assertBefore(
    restoreViews,
    "bounded_terminal_view_item_ids(item_ids",
    "join_all(",
    "terminal restore item IDs must be capped before join_all fanout",
  );
  assertBefore(
    restoreViews,
    "bounded_item_ids.iter()",
    "TerminalView::deserialize",
    "terminal restore must iterate the bounded item IDs",
  );

  const restoreGroup = functionBody(persistence, "deserialize_pane_group");
  assertBefore(
    restoreGroup,
    "restore_limits.consume_pane_node()",
    "let mut members = Vec::new();",
    "terminal pane nodes must be counted before child members are materialized",
  );
});

test("terminal panel task and selection materialization is bounded before collect and join", () => {
  const panel = read("crates/terminal_view/src/terminal_panel.rs");

  for (const pattern of [
    /const MAX_TERMINAL_PANEL_TASK_MATCHES: usize = 1_024;/,
    /const MAX_TERMINAL_PANEL_TASK_JOIN_HANDLES: usize = 1_024;/,
    /const MAX_TERMINAL_PANEL_SELECTIONS: usize = 1_024;/,
  ]) {
    assert.match(panel, pattern);
  }

  const taskMatches = functionBody(panel, "terminals_for_task");
  assertBefore(
    taskMatches,
    ".take(MAX_TERMINAL_PANEL_TASK_MATCHES.saturating_add(1))",
    ".collect::<Vec<_>>()",
    "matching task terminals must be capped before collection",
  );
  assert.match(taskMatches, /matching_terminals\.truncate\(MAX_TERMINAL_PANEL_TASK_MATCHES\)/);
  assert.match(taskMatches, /log::warn!\(/);

  const taskJoin = functionBody(panel, "wait_for_terminals_tasks");
  assertBefore(
    taskJoin,
    ".take(MAX_TERMINAL_PANEL_TASK_JOIN_HANDLES)",
    "join_all(pending_tasks).await",
    "terminal task waits must be capped before join_all",
  );
  assert.match(taskJoin, /log::warn!\(/);

  const selections = functionBody(panel, "terminal_selections");
  assertBefore(
    selections,
    ".take(MAX_TERMINAL_PANEL_SELECTIONS.saturating_add(1))",
    ".collect::<Vec<_>>()",
    "terminal selections must be capped before collection",
  );
  assert.match(selections, /selections\.truncate\(MAX_TERMINAL_PANEL_SELECTIONS\)/);
});

test("terminal panel next and previous pane actions use checked pane lookups", () => {
  const panel = read("crates/terminal_view/src/terminal_panel.rs");
  const nextPaneAction = sliceBetween(
    panel,
    "_action: &ActivateNextPane",
    "_action: &ActivatePreviousPane",
  );
  const previousPaneAction = sliceBetween(
    panel,
    "_action: &ActivatePreviousPane",
    "action: &ActivatePane",
  );

  assert.doesNotMatch(
    nextPaneAction,
    /panes\s*\[\s*next_ix\s*\]/,
    "ActivateNextPane must not index panes directly with next_ix",
  );
  assert.match(
    nextPaneAction,
    /panes\.get\(next_ix\)/,
    "ActivateNextPane must focus only a pane returned by panes.get(next_ix)",
  );

  assert.doesNotMatch(
    previousPaneAction,
    /panes\s*\[\s*prev_ix\s*\]/,
    "ActivatePreviousPane must not index panes directly with prev_ix",
  );
  assert.match(
    previousPaneAction,
    /panes\.get\(prev_ix\)/,
    "ActivatePreviousPane must focus only a pane returned by panes.get(prev_ix)",
  );
});

test("terminal view user-data vectors are bounded before path paste and match storage", () => {
  const view = read("crates/terminal_view/src/terminal_view.rs");

  for (const pattern of [
    /const MAX_TERMINAL_PATHS_TO_PASTE: usize = 512;/,
    /const MAX_TERMINAL_PATH_PASTE_BYTES: usize = 256 \* 1024;/,
    /const MAX_TERMINAL_VIEW_STORED_MATCHES: usize = 20_000;/,
  ]) {
    assert.match(view, pattern);
  }

  const addPaths = functionBody(view, "add_paths_to_terminal");
  assertBefore(
    addPaths,
    "bounded_terminal_paths_for_paste(paths.iter())",
    "terminal.paste(&text)",
    "terminal path paste text must be capped before paste materialization",
  );

  const dropHandler = sliceBetween(
    view,
    "} else if let Some(selection) = dropped.downcast_ref::<DraggedSelection>() {",
    "} else if let Some(&entry_id) = dropped.downcast_ref::<ProjectEntryId>() {",
  );
  assert.doesNotMatch(dropHandler, /collect::<Vec<_>>\(\)/);
  assert.match(dropHandler, /bounded_terminal_paths_for_paste/);

  const updateMatches = functionBody(view, "update_matches");
  assertBefore(
    updateMatches,
    "bounded_terminal_view_matches(matches)",
    "term.matches = bounded_matches",
    "terminal matches must be capped before storage",
  );
  assert.doesNotMatch(updateMatches, /matches\.to_vec\(\)/);

  const findMatches = functionBody(view, "find_matches");
  assertBefore(
    findMatches,
    "let matches = matches.await;",
    "bounded_terminal_view_matches(&matches)",
    "terminal search results returned to the search UI must share the stored-match cap",
  );
});

test("terminal element render materialization is bounded before row cell and highlight pushes", () => {
  const element = read("crates/terminal_view/src/terminal_element.rs");

  for (const pattern of [
    /const MAX_TERMINAL_ELEMENT_VISIBLE_ROWS: usize = 10_000;/,
    /const MAX_TERMINAL_ELEMENT_RENDER_CELLS: usize = 1_000_000;/,
    /const MAX_TERMINAL_ELEMENT_BACKGROUND_REGIONS: usize = 1_000_000;/,
    /const MAX_TERMINAL_ELEMENT_LAYOUT_RECTS: usize = 1_000_000;/,
    /const MAX_TERMINAL_ELEMENT_HIGHLIGHT_RANGES: usize = 20_000;/,
    /const MAX_TERMINAL_ELEMENT_HIGHLIGHT_LINES: usize = 10_000;/,
  ]) {
    assert.match(element, pattern);
  }

  const prepaint = sliceBetween(
    element,
    "bounded_terminal_element_search_matches",
    "LayoutState {",
  );
  assertBefore(
    prepaint,
    "bounded_terminal_element_search_matches",
    "relative_highlighted_ranges.push",
    "terminal search highlights must be capped before highlight ranges are pushed",
  );
  assertBefore(
    prepaint,
    "let capped_visible_row_count",
    ".take(capped_visible_row_count)",
    "visible terminal rows must be capped before row groups are taken",
  );

  const layoutGrid = functionBody(element, "layout_grid");
  assertBefore(
    layoutGrid,
    "processed_cells >= MAX_TERMINAL_ELEMENT_RENDER_CELLS",
    "background_regions.push",
    "terminal cells must be capped before background region materialization",
  );
  assertBefore(
    layoutGrid,
    "background_regions.len() >= MAX_TERMINAL_ELEMENT_BACKGROUND_REGIONS",
    "background_regions.push",
    "terminal background regions must be capped before vector push",
  );
  assertBefore(
    layoutGrid,
    "rects.len() >= MAX_TERMINAL_ELEMENT_LAYOUT_RECTS",
    "rects.push",
    "terminal layout rects must be capped before vector push",
  );

  const highlights = functionBody(element, "to_highlighted_range_lines");
  assertBefore(
    highlights,
    "highlighted_range_lines.len() >= MAX_TERMINAL_ELEMENT_HIGHLIGHT_LINES",
    "highlighted_range_lines.push",
    "terminal highlight lines must be capped before vector push",
  );
});
