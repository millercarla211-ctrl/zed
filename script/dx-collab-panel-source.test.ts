import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path) => readFileSync(path, "utf8");

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
