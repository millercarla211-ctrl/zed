import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const pickerSources = [
  {
    path: "crates/settings_ui/src/components/font_picker.rs",
    collection: "fonts",
    current: "current_font",
  },
  {
    path: "crates/settings_ui/src/components/icon_theme_picker.rs",
    collection: "icon_themes",
    current: "current_theme",
  },
  {
    path: "crates/settings_ui/src/components/theme_picker.rs",
    collection: "themes",
    current: "current_theme",
  },
];

const read = (path: string) => readFileSync(path, "utf8");

test("settings UI picker current selections guard stale fuzzy candidate ids", () => {
  for (const { path, collection, current } of pickerSources) {
    const source = read(path);
    const directIndexPattern = new RegExp(
      `${collection}\\s*\\[\\s*m\\.candidate_id\\s*\\]\\s*==\\s*${current}`,
    );
    const guardedLookupPattern = new RegExp(
      `${collection}\\s*\\.get\\(m\\.candidate_id\\)\\s*\\.is_some_and\\(\\|[^|]+\\|\\s*\\*?[^=]+==\\s*${current}\\)`,
    );

    assert.doesNotMatch(
      source,
      directIndexPattern,
      `${path} must not directly index fuzzy candidate ids`,
    );
    assert.match(
      source,
      guardedLookupPattern,
      `${path} must fail closed when a fuzzy candidate id is stale`,
    );
  }
});

test("settings UI picker source guard stays scoped to worker-owned files", () => {
  assert.deepEqual(
    pickerSources.map(({ path }) => path),
    [
      "crates/settings_ui/src/components/font_picker.rs",
      "crates/settings_ui/src/components/icon_theme_picker.rs",
      "crates/settings_ui/src/components/theme_picker.rs",
    ],
  );
});
