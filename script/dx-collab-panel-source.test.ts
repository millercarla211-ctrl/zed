import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path) => readFileSync(path, "utf8");

const extractFunction = (source, startMarker, endMarker) => {
  const start = source.indexOf(startMarker);
  assert.notEqual(start, -1, `missing ${startMarker.trim()}`);

  const end = source.indexOf(endMarker, start);
  assert.notEqual(end, -1, `missing ${endMarker.trim()}`);

  return source.slice(start, end);
};

test("collab panel fuzzy results skip stale candidate ids during materialization", () => {
  const source = read("crates/collab_ui/src/collab_panel.rs");
  const forbiddenLookups = [
    {
      label: "pending participants",
      pattern: /room\.pending_participants\(\)\s*\[\s*mat\.candidate_id\s*\]/,
    },
    {
      label: "channels",
      pattern: /channels\s*\[\s*mat\.candidate_id\s*\]/,
    },
    {
      label: "channel invites",
      pattern: /channel_invites\s*\[\s*mat\.candidate_id\s*\]/,
    },
    {
      label: "incoming requests",
      pattern: /incoming\s*\[\s*mat\.candidate_id\s*\]/,
    },
    {
      label: "outgoing requests",
      pattern: /outgoing\s*\[\s*mat\.candidate_id\s*\]/,
    },
    {
      label: "contacts",
      pattern: /contacts\s*\[\s*mat\.candidate_id\s*\]/,
    },
  ];

  for (const { label, pattern } of forbiddenLookups) {
    assert.doesNotMatch(
      source,
      pattern,
      `${label} fuzzy matches must fail closed instead of indexing stale candidate ids`,
    );
  }

  for (const safeLookup of [
    /room\.pending_participants\(\)\s*\.get\(mat\.candidate_id\)/,
    /channels\s*\.get\(mat\.candidate_id\)/,
    /channel_invites\s*\.get\(mat\.candidate_id\)/,
    /incoming\s*\.get\(mat\.candidate_id\)/,
    /outgoing\s*\.get\(mat\.candidate_id\)/,
    /contacts\s*\.get\(mat\.candidate_id\)/,
  ]) {
    assert.match(source, safeLookup);
  }
});

test("collab panel list row rendering skips stale row indexes", () => {
  const source = read("crates/collab_ui/src/collab_panel.rs");
  const renderListEntry = extractFunction(
    source,
    "    fn render_list_entry(",
    "    fn render_signed_in(",
  );

  assert.doesNotMatch(
    renderListEntry,
    /self\.entries\s*\[\s*ix\s*\]/,
    "render_list_entry must not index self.entries with a stale row index",
  );

  const checkedLookupIndex = renderListEntry.search(
    /self\.entries\s*\.get\(ix\)\s*\.cloned\(\)/,
  );
  assert.notEqual(
    checkedLookupIndex,
    -1,
    "render_list_entry must clone entries through a checked lookup",
  );

  const firstEntryRenderIndex = renderListEntry.search(
    /let\s+is_selected\s*=|match\s+entry/,
  );
  assert.ok(
    checkedLookupIndex < firstEntryRenderIndex,
    "render_list_entry must check the row index before rendering the entry",
  );

  assert.match(
    renderListEntry,
    /return\s+Empty\.into_any_element\(\)/,
    "render_list_entry must return an empty element when the row index is stale",
  );
});
