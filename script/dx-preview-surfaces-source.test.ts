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

test("csv preview rendered-index metrics are bounded before collection", () => {
  const csvPreview = read("crates/csv_preview/src/csv_preview.rs");
  const renderTable = read("crates/csv_preview/src/renderer/render_table.rs");
  const recordRenderedIndex = functionBody(csvPreview, "record_rendered_index");
  const recordRenderedIndices = functionBody(csvPreview, "record_rendered_indices");
  const createTableInner = functionBody(renderTable, "create_table_inner");

  assert.match(csvPreview, /const MAX_CSV_PREVIEW_RENDERED_INDICES: usize = 2_048;/);
  assertBefore({
    body: recordRenderedIndex,
    before: "self.rendered_indices.len() >= MAX_CSV_PREVIEW_RENDERED_INDICES",
    after: "self.rendered_indices.push(index)",
    message: "rendered-index metrics must check the cap before vector pushes",
  });
  assert.match(
    recordRenderedIndices,
    /self\.record_rendered_index\(index\);/,
    "bulk rendered-index recording must use the capped single-index helper",
  );
  assertBefore({
    body: recordRenderedIndices,
    before: "self.rendered_indices.len() >= MAX_CSV_PREVIEW_RENDERED_INDICES",
    after: "self.record_rendered_index(index)",
    message: "bulk rendered-index recording must stop once the cap is reached",
  });
  assert.match(
    createTableInner,
    /this\.performance_metrics\.record_rendered_index\(display_row\)/,
    "variable-height CSV rendering must use the capped metrics helper",
  );
  assert.match(
    createTableInner,
    /this\.performance_metrics\s*\.record_rendered_indices\(range\.clone\(\)\)/,
    "uniform CSV rendering must use the capped metrics helper",
  );
  assert.doesNotMatch(
    renderTable,
    /rendered_indices\s*\.\s*(?:push|extend)\s*\(/,
    "rendering code must not bypass rendered-index metric caps",
  );
});
