import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const functionBody = (source: string, name: string) => {
  const start = source.search(new RegExp(`pub\\s+fn\\s+${name}\\s*\\(`));
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

const assertNewlineRemapUsesCheckedRows = (body: string, name: string) => {
  assert.doesNotMatch(
    body,
    /let\s+row\s*=\s*rows\s*\[\s*index\s*\]\s*;\s*index\s*\+=\s*1\s*;/s,
    `${name} cursor remap must not panic on stale row-vector indexes`,
  );
  assert.doesNotMatch(
    body,
    /rows\s*\[\s*index\s*\]/,
    `${name} cursor remap must not direct-index row mappings`,
  );
  assert.match(
    body,
    /(?:let\s+Some\(row\)\s*=\s*rows\.get\(index\)\.copied\(\)\s*else\s*\{\s*return\s+\(cursor,\s*goal\);\s*\};|let\s+row\s*=\s*rows\.get\(index\)\.copied\(\)\.flatten\(\);\s*index\s*\+=\s*1;\s*let\s+Some\(row\)\s*=\s*row\s*else\s*\{\s*return\s+\(cursor,\s*goal\);\s*\};)/s,
    `${name} cursor remap must preserve the existing cursor when mappings drift`,
  );
  assert.match(
    body,
    /s\.move_cursors_with\(&mut\s+\|map,\s*cursor,\s*goal\|/s,
    `${name} cursor remap must retain the original cursor and goal for stale mappings`,
  );
  assert.match(
    body,
    /index\s*\+=\s*1;[\s\S]*?let\s+point\s*=\s*Point::new\(row,\s*0\);/s,
    `${name} cursor remap must still advance valid checked mappings`,
  );
  assert.match(
    body,
    /let\s+boundary\s*=\s*map\.next_line_boundary\(point\)\.1;\s*let\s+clipped\s*=\s*map\.clip_point\(boundary,\s*Bias::Left\);\s*\(clipped,\s*SelectionGoal::None\)/s,
    `${name} cursor remap must preserve the valid newline cursor target`,
  );
};

const assertNewlineBelowKeepsSelectionOrdinalMappings = (body: string) => {
  assert.match(
    body,
    /else\s*\{\s*rows\.push\(None\);\s*continue;\s*\};/s,
    "newline_below must keep a row-mapping slot for skipped selections",
  );
  assert.match(
    body,
    /rows\.push\(Some\(row \+ rows_inserted\)\);/,
    "newline_below must store successful row mappings as optional slots",
  );
  assert.match(
    body,
    /let\s+row\s*=\s*rows\.get\(index\)\.copied\(\)\.flatten\(\);\s*index\s*\+=\s*1;\s*let\s+Some\(row\)\s*=\s*row\s*else\s*\{\s*return\s+\(cursor,\s*goal\);\s*\};/s,
    "newline_below must advance the cursor ordinal for both skipped and valid mappings",
  );
  assert.match(
    body,
    /for\s+row\s+in\s+rows\.into_iter\(\)\.flatten\(\)/,
    "newline_below indent pass must ignore skipped row mappings",
  );
};

test("newline-above and newline-below cursor remaps use checked stale-safe row lookups", () => {
  const source = read("crates/editor/src/input.rs");

  assertNewlineRemapUsesCheckedRows(functionBody(source, "newline_above"), "newline_above");
  assertNewlineRemapUsesCheckedRows(functionBody(source, "newline_below"), "newline_below");
});

test("newline-below preserves cursor-to-row mapping when selections are skipped", () => {
  const source = read("crates/editor/src/input.rs");

  assertNewlineBelowKeepsSelectionOrdinalMappings(functionBody(source, "newline_below"));
});
