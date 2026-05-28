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
    new RegExp(`(?:pub\\([^)]*\\)\\s+)?(?:pub\\s+)?fn\\s+${name}\\s*\\(`),
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

test("diagnostic renderer caps group, hint, markdown, and copy materialization", () => {
  const source = read("crates/diagnostics/src/diagnostic_renderer.rs");
  const body = functionBody(source, "diagnostic_blocks_for_group");

  assert.match(source, /const MAX_DIAGNOSTIC_BLOCKS_PER_GROUP: usize = [\d_]+;/);
  assert.match(source, /const MAX_DIAGNOSTIC_HINT_LINKS_PER_GROUP: usize = [\d_]+;/);
  assert.match(source, /const MAX_DIAGNOSTIC_MARKDOWN_CHARS: usize = [\d_]+;/);
  assert.match(source, /const MAX_DIAGNOSTIC_COPY_MESSAGE_CHARS: usize = [\d_]+;/);
  assert.match(source, /fn bounded_diagnostic_group_entries(?:<[^>]+>)?\(/);
  assert.match(source, /fn bounded_diagnostic_markdown\(/);
  assert.match(source, /fn bounded_diagnostic_copy_message\(/);

  assertBefore({
    body,
    before: "bounded_diagnostic_group_entries(",
    after: "for entry in bounded_entries.iter()",
    message: "diagnostic groups must be capped before block iteration",
  });
  assertBefore({
    body,
    before: ".take(MAX_DIAGNOSTIC_HINT_LINKS_PER_GROUP)",
    after: '"\\n- hint: ["',
    message: "diagnostic hint links must be capped before markdown rows are appended",
  });
  assertBefore({
    body,
    before: "bounded_diagnostic_markdown(",
    after: "Markdown::new(markdown.into()",
    message: "diagnostic markdown must be bounded before Markdown entities are created",
  });
  assert.match(body, /bounded_diagnostic_copy_message\(\s*&primary\.diagnostic\.message,\s*\)/s);
  assert.match(body, /bounded_diagnostic_copy_message\(\s*&entry\.entry\.diagnostic\.message,\s*\)/s);
});

test("project diagnostics caps full-buffer collection, grouping, blocks, excerpts, and refresh paths", () => {
  const source = read("crates/diagnostics/src/diagnostics.rs");
  const refreshBody = functionBody(source, "refresh");
  const updateBody = functionBody(source, "update_excerpts");

  assert.match(source, /const MAX_DIAGNOSTICS_PER_BUFFER: usize = [\d_]+;/);
  assert.match(source, /const MAX_DIAGNOSTIC_GROUPS_PER_BUFFER: usize = [\d_]+;/);
  assert.match(source, /const MAX_DIAGNOSTIC_BLOCKS_PER_BUFFER: usize = [\d_]+;/);
  assert.match(source, /const MAX_DIAGNOSTIC_EXCERPTS_PER_BUFFER: usize = [\d_]+;/);
  assert.match(source, /const MAX_DIAGNOSTIC_PATHS_PER_REFRESH: usize = [\d_]+;/);
  assert.match(source, /pub\(crate\) fn push_diagnostic_group_entry/);
  assert.match(source, /pub\(crate\) fn truncate_diagnostic_blocks_for_buffer/);

  assertBefore({
    body: refreshBody,
    before: ".take(MAX_DIAGNOSTIC_PATHS_PER_REFRESH)",
    after: ".collect::<BTreeSet<_>>()",
    message: "diagnostic refresh paths must be capped before set materialization",
  });
  assertBefore({
    body: updateBody,
    before: ".take(MAX_DIAGNOSTICS_PER_BUFFER)",
    after: ".collect::<Vec<_>>()",
    message: "project diagnostics must be capped before Vec materialization",
  });
  assertBefore({
    body: updateBody,
    before: "push_diagnostic_group_entry(",
    after: "diagnostic_blocks_for_group(",
    message: "project diagnostic groups must be bounded before rendering blocks",
  });
  assertBefore({
    body: updateBody,
    before: "truncate_diagnostic_blocks_for_buffer(&mut blocks);",
    after: "for b in blocks",
    message: "project diagnostic blocks must be capped before excerpt expansion",
  });
  assertBefore({
    body: updateBody,
    before: "MAX_DIAGNOSTIC_EXCERPTS_PER_BUFFER",
    after: "result_blocks.insert",
    message: "project excerpts must stop growing before result blocks are inserted",
  });
});

test("buffer diagnostics reuses the same collection, grouping, block, and excerpt caps", () => {
  const source = read("crates/diagnostics/src/buffer_diagnostics.rs");
  const body = functionBody(source, "update_excerpts");

  assert.match(source, /MAX_DIAGNOSTICS_PER_BUFFER/);
  assert.match(source, /MAX_DIAGNOSTIC_EXCERPTS_PER_BUFFER/);
  assert.match(source, /push_diagnostic_group_entry/);
  assert.match(source, /truncate_diagnostic_blocks_for_buffer/);

  assertBefore({
    body,
    before: ".take(MAX_DIAGNOSTICS_PER_BUFFER)",
    after: ".collect::<Vec<_>>()",
    message: "buffer diagnostics must be capped before Vec materialization",
  });
  assertBefore({
    body,
    before: "push_diagnostic_group_entry(",
    after: "DiagnosticRenderer::diagnostic_blocks_for_group(",
    message: "buffer diagnostic groups must be bounded before rendering blocks",
  });
  assertBefore({
    body,
    before: "truncate_diagnostic_blocks_for_buffer(&mut blocks);",
    after: "for diagnostic_block in blocks.iter()",
    message: "buffer diagnostic blocks must be capped before excerpt expansion",
  });
  assertBefore({
    body,
    before: "Vec::with_capacity(blocks.len().min(MAX_DIAGNOSTIC_EXCERPTS_PER_BUFFER))",
    after: "for diagnostic_block in blocks.iter()",
    message: "buffer excerpt storage must be sized from the capped block set",
  });
});

test("diagnostic status labels are bounded before toolbar button materialization", () => {
  const source = read("crates/diagnostics/src/items.rs");
  const body = functionBody(source, "render");

  assert.match(source, /const MAX_STATUS_DIAGNOSTIC_LABEL_CHARS: usize = [\d_]+;/);
  assert.match(source, /fn diagnostic_status_message\(diagnostic: &Diagnostic\) -> SharedString/);
  assertBefore({
    body,
    before: "diagnostic_status_message(diagnostic)",
    after: 'Button::new("diagnostic_message"',
    message: "status diagnostic labels must be bounded before toolbar button creation",
  });
  assert.doesNotMatch(body, /SharedString::new\(message\)/);
});
