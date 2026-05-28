import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/breadcrumbs/src/breadcrumbs.rs";
const source = readFileSync(sourcePath, "utf8");

const functionBody = (name: string): string => {
  const start = source.search(new RegExp(`fn\\s+${name}\\s*\\(`));
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

const sliceBetween = (startNeedle: string, endNeedle: string): string => {
  const start = source.indexOf(startNeedle);
  assert.notEqual(start, -1, `expected ${startNeedle}`);
  const end = source.indexOf(endNeedle, start + startNeedle.length);
  assert.ok(end > start, `expected ${endNeedle} after ${startNeedle}`);
  return source.slice(start, end);
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

test("breadcrumbs bound segment materialization before the global renderer", () => {
  assert.equal(sourcePath, "crates/breadcrumbs/src/breadcrumbs.rs");
  assert.match(source, /const MAX_BREADCRUMB_SEGMENTS_FOR_RENDERER: usize = 256;/);

  const helper = functionBody("bounded_breadcrumb_segments");
  assert.match(helper, /segments\.len\(\) <= MAX_BREADCRUMB_SEGMENTS_FOR_RENDERER/);
  assert.match(
    helper,
    /let prefix_segment_count = MAX_BREADCRUMB_SEGMENTS_FOR_RENDERER \/ 2;/,
  );
  assert.match(
    helper,
    /let suffix_segment_count\s*=\s*MAX_BREADCRUMB_SEGMENTS_FOR_RENDERER\s*\.saturating_sub\(prefix_segment_count \+ 1\);/,
  );
  assert.match(
    helper,
    /segments\.splice\(\s*prefix_segment_count\.\.suffix_start,/,
    "bounded breadcrumbs should compact the middle instead of dropping the suffix",
  );
  assert.match(helper, /text: "\.\.\."\.into\(\)/);

  const renderBody = sliceBetween(
    "impl Render for Breadcrumbs {",
    "impl ToolbarItemView for Breadcrumbs {",
  );
  assertBefore({
    body: renderBody,
    before: "let segments = bounded_breadcrumb_segments(segments);",
    after: "let prefix_element = active_item.breadcrumb_prefix(window, cx);",
    message: "breadcrumb segments must be bounded before prefix/global render materialization",
  });
  assertBefore({
    body: renderBody,
    before: "let segments = bounded_breadcrumb_segments(segments);",
    after: "(render_fn.0)(",
    message: "breadcrumb segments must be bounded before invoking the global renderer",
  });
});
