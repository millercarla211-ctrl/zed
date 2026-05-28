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

test("navigation hover links and locations are capped before task joins and UI materialization", () => {
  const source = read("crates/editor/src/navigation.rs");
  const definitionBody = functionBody(source, "go_to_definition_of_kind");
  const hoverBody = functionBody(source, "navigate_to_hover_links");
  const referenceBody = functionBody(
    source,
    "go_to_reference_before_or_after_position",
  );
  const allReferencesBody = functionBody(source, "find_all_references");

  assert.match(source, /const MAX_NAVIGATION_HOVER_LINKS: usize = 4_096;/);
  assert.match(source, /const MAX_NAVIGATION_LOCATION_TASKS: usize = 4_096;/);
  assert.match(source, /const MAX_NAVIGATION_LOCATIONS: usize = 20_000;/);
  assert.match(source, /const MAX_NAVIGATION_RANGES_PER_BUFFER: usize = 10_000;/);
  assert.match(source, /const MAX_NAVIGATION_EXCERPT_ANCHORS: usize = 20_000;/);
  assert.match(source, /fn cap_hover_links_for_navigation\(/);
  assert.match(source, /fn cap_reference_locations_for_navigation\(/);
  assert.match(source, /fn push_navigation_location\(/);
  assert.match(source, /fn cap_navigation_ranges_for_buffer\(/);

  assertBefore({
    body: definitionBody,
    before: "cap_hover_links_for_navigation(",
    after: ".collect::<Vec<_>>()",
    message: "definition hover links must be capped before Vec materialization",
  });
  assertBefore({
    body: hoverBody,
    before: "cap_hover_links_for_navigation(definitions)",
    after: ".collect()",
    message: "hover links must be capped before task vector collection",
  });
  assertBefore({
    body: hoverBody,
    before: "MAX_NAVIGATION_LOCATION_TASKS",
    after: "future::join_all(definitions)",
    message: "location tasks must be capped before join_all",
  });
  assertBefore({
    body: hoverBody,
    before: "push_navigation_location(",
    after: ".into_group_map()",
    message: "resolved locations must be capped before group-map materialization",
  });
  assertBefore({
    body: hoverBody,
    before: "cap_navigation_ranges_for_buffer(",
    after: "Self::open_locations_in_multibuffer(",
    message: "multibuffer ranges must be capped before opening UI locations",
  });
  assertBefore({
    body: hoverBody,
    before: ".take(MAX_NAVIGATION_RANGES_PER_BUFFER)",
    after: ".collect::<Vec<_>>()",
    message: "single target ranges must be capped before selection vector collection",
  });
  assertBefore({
    body: referenceBody,
    before: "cap_reference_locations_for_navigation(locations)",
    after: ".collect::<Vec<_>>()",
    message: "reference jumps must cap LSP locations before Vec materialization",
  });
  assertBefore({
    body: allReferencesBody,
    before: "cap_reference_locations_for_navigation(locations)",
    after: ".into_group_map()",
    message: "find-all references must cap LSP locations before grouping",
  });
  assertBefore({
    body: functionBody(source, "expand_excerpts_for_direction"),
    before: ".take(MAX_NAVIGATION_EXCERPT_ANCHORS)",
    after: ".collect::<Vec<_>>()",
    message: "excerpt expansion anchors must be capped before Vec materialization",
  });
});

test("document highlight navigation is capped before sorting highlight rows", () => {
  const source = read("crates/editor/src/navigation.rs");
  const body = functionBody(
    source,
    "go_to_document_highlight_before_or_after_position",
  );

  assert.match(source, /const MAX_DOCUMENT_HIGHLIGHTS_TO_NAVIGATE: usize = 20_000;/);
  assertBefore({
    body,
    before: "MAX_DOCUMENT_HIGHLIGHTS_TO_NAVIGATE",
    after: "all_highlights.sort_by",
    message: "document highlights must be capped before sort/materialization",
  });
});

test("linked editing ranges cap selection, task, and sibling fanout before joins", () => {
  const source = read("crates/editor/src/linked_editing_ranges.rs");
  const body = functionBody(source, "refresh_linked_ranges");

  assert.match(source, /const MAX_LINKED_EDITING_SELECTIONS: usize = 512;/);
  assert.match(source, /const MAX_LINKED_EDITING_TASKS: usize = 512;/);
  assert.match(source, /const MAX_LINKED_EDITING_RANGES_PER_RESPONSE: usize = 256;/);
  assert.match(source, /const MAX_LINKED_EDITING_RANGES_PER_BUFFER: usize = 20_000;/);
  assert.match(source, /fn cap_linked_edit_ranges_for_response\(/);
  assertBefore({
    body,
    before: ".take(MAX_LINKED_EDITING_SELECTIONS)",
    after: "applicable_selections.push(",
    message: "linked-edit selections must be capped before task inputs are stored",
  });
  assertBefore({
    body,
    before: "MAX_LINKED_EDITING_TASKS",
    after: "linked_edits_tasks.push(highlights)",
    message: "linked-edit tasks must be capped before task vector push",
  });
  assertBefore({
    body,
    before: "MAX_LINKED_EDITING_TASKS",
    after: "futures::future::join_all(highlights)",
    message: "linked-edit tasks must be capped before join_all",
  });
  assertBefore({
    body,
    before: "cap_linked_edit_ranges_for_response(edits)",
    after: ".combinations(2)",
    message: "linked-edit response ranges must be capped before pair fanout",
  });
  assertBefore({
    body,
    before: "MAX_LINKED_EDITING_RANGES_PER_BUFFER",
    after: ".extend(ranges)",
    message: "linked-edit stored ranges must be capped before extending editor state",
  });
});

test("signature help caps signature, label, parameter, and highlight materialization", () => {
  const source = read("crates/editor/src/signature_help.rs");
  const showBody = functionBody(source, "show_signature_help_impl");
  const renderBody = functionBody(source, "render");

  assert.match(source, /const MAX_SIGNATURE_HELP_SIGNATURES: usize = 128;/);
  assert.match(source, /const MAX_SIGNATURE_HELP_PARAMETERS_PER_SIGNATURE: usize = 256;/);
  assert.match(source, /const MAX_SIGNATURE_HELP_LABEL_BYTES: usize = 16_384;/);
  assert.match(source, /const MAX_SIGNATURE_HELP_HIGHLIGHTS: usize = 4_096;/);
  assert.match(source, /fn cap_signature_label\(/);
  assert.match(source, /fn active_parameter_documentation\(/);
  assertBefore({
    body: showBody,
    before: ".take(MAX_SIGNATURE_HELP_SIGNATURES)",
    after: "Rope::from(signature.label.as_ref())",
    message: "signature labels must be bounded before syntax highlighting",
  });
  assertBefore({
    body: showBody,
    before: ".take(MAX_SIGNATURE_HELP_HIGHLIGHTS)",
    after: ".collect()",
    message: "signature highlights must be capped before highlight vector collection",
  });
  assertBefore({
    body: showBody,
    before: ".take(MAX_SIGNATURE_HELP_SIGNATURES)",
    after: ".collect::<Vec<_>>()",
    message: "signatures must be capped before popover Vec materialization",
  });
  assertBefore({
    body: showBody,
    before: "active_parameter_documentation(",
    after: "parameter_documentation:",
    message: "active parameter docs must go through the bounded parameter helper",
  });
  assertBefore({
    body: renderBody,
    before: "cap_signature_label(",
    after: "StyledText::new(signature_label)",
    message: "rendered signature labels must be capped before StyledText UI materialization",
  });
});

test("LSP extension helpers cap sources, buffers, and runnables before result materialization", () => {
  const source = read("crates/editor/src/lsp_ext.rs");
  const body = functionBody(source, "lsp_tasks");

  assert.match(source, /const MAX_LSP_TASK_SOURCES: usize = 128;/);
  assert.match(source, /const MAX_LSP_TASK_BUFFERS_PER_SOURCE: usize = 512;/);
  assert.match(source, /const MAX_LSP_RUNNABLES_PER_RESPONSE: usize = 2_048;/);
  assert.match(source, /const MAX_LSP_TASKS_PER_SOURCE: usize = 4_096;/);
  assertBefore({
    body,
    before: ".take(MAX_LSP_TASK_SOURCES)",
    after: "let buffers = buffer_ids",
    message: "LSP task sources must be capped before per-source buffer materialization",
  });
  assertBefore({
    body,
    before: ".take(MAX_LSP_TASK_BUFFERS_PER_SOURCE)",
    after: ".collect::<Vec<_>>()",
    message: "LSP task buffers must be capped before Vec materialization",
  });
  assertBefore({
    body,
    before: ".take(MAX_LSP_RUNNABLES_PER_RESPONSE)",
    after: "new_lsp_tasks.extend(",
    message: "LSP runnables must be capped before extending task rows",
  });
  assertBefore({
    body,
    before: "MAX_LSP_TASKS_PER_SOURCE",
    after: ".append(&mut new_lsp_tasks)",
    message: "LSP task rows must be capped before appending to result map",
  });
  assertBefore({
    body,
    before: "MAX_LSP_TASKS_PER_SOURCE",
    after: "lsp_tasks.into_iter().collect()",
    message: "LSP task rows must be capped before final result collection",
  });
});
