import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const indexOfPattern = (source: string, pattern: string | RegExp) => {
  if (typeof pattern === "string") {
    return source.indexOf(pattern);
  }

  return source.match(pattern)?.index ?? -1;
};

const functionBody = (source: string, name: string) => {
  const start = indexOfPattern(
    source,
    new RegExp(`(?:pub\\([^)]*\\)\\s+)?fn\\s+${name}\\s*\\(`),
  );
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

test("editor element layout materialization checks stale layout indexes", () => {
  const source = read("crates/editor/src/element.rs");
  const navigationLabel = functionBody(source, "layout_navigation_label");
  const inlineDiagnostics = functionBody(source, "layout_inline_diagnostics");
  const cursorPopovers = functionBody(source, "layout_cursor_popovers");

  assert.doesNotMatch(
    navigationLabel,
    /context\.line_layouts\s*\[\s*row_index\s*\]/,
    "navigation labels must not direct-index visible line layouts",
  );
  assert.match(
    navigationLabel,
    /context\.line_layouts\.get\(\s*row_index\s*\)/,
    "navigation labels must check the visible line layout index",
  );

  assert.doesNotMatch(
    inlineDiagnostics,
    /crease_trailers\s*\[\s*window_ix\s*\]/,
    "inline diagnostics must not direct-index crease trailer layouts",
  );
  assert.doesNotMatch(
    inlineDiagnostics,
    /line_layouts\s*\[\s*window_ix\s*\]/,
    "inline diagnostics must not direct-index line layouts",
  );
  assert.match(
    inlineDiagnostics,
    /crease_trailers\.get\(\s*window_ix\s*\)/,
    "inline diagnostics must check the crease trailer index",
  );
  assert.match(
    inlineDiagnostics,
    /line_layouts\.get\(\s*window_ix\s*\)/,
    "inline diagnostics must check the line layout index",
  );

  assert.doesNotMatch(
    cursorPopovers,
    /line_layouts\s*\[\s*cursor\.row\(\)\.minus\(start_row\) as usize\s*\]/,
    "cursor popovers must not direct-index cursor row layouts",
  );
  assert.match(
    cursorPopovers,
    /line_layouts\.get\(\s*cursor_row_ix\s*\)/,
    "cursor popovers must check the cursor row layout index",
  );
  assert.doesNotMatch(
    cursorPopovers,
    /laid_out_popovers\s*\[\s*0\s*\]/,
    "cursor popovers must not direct-index the first laid-out popover",
  );
  assert.doesNotMatch(
    cursorPopovers,
    /laid_out_popovers\s*\[\s*last_ix\s*\]/,
    "cursor popovers must not direct-index the last laid-out popover",
  );
  assert.match(
    cursorPopovers,
    /laid_out_popovers\.first\(\)/,
    "cursor popovers must check the first laid-out popover bounds",
  );
  assert.match(
    cursorPopovers,
    /laid_out_popovers\.last\(\)/,
    "cursor popovers must check the last laid-out popover bounds",
  );
});
