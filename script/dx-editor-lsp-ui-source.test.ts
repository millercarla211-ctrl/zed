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

test("code action provider and action fanout is capped before task joins and menu storage", () => {
  const source = read("crates/editor/src/code_actions.rs");
  const refreshBody = functionBody(source, "refresh_code_actions_for_selection");

  assert.match(source, /const MAX_CODE_ACTION_PROVIDERS: usize = 64;/);
  assert.match(source, /const MAX_CODE_ACTIONS_FOR_SELECTION: usize = 1_024;/);
  assert.match(
    source,
    /fn bounded_code_action_providers\(/,
    "provider fanout should be isolated in a named helper",
  );
  assert.match(
    source,
    /fn push_bounded_code_actions_for_provider\(/,
    "provider action fanout should be isolated in a named helper",
  );
  assertBefore({
    body: refreshBody,
    before: "bounded_code_action_providers(",
    after: ".collect::<Vec<_>>()",
    message: "code action providers must be capped before task vector collection",
  });
  assertBefore({
    body: refreshBody,
    before: "bounded_code_action_providers(",
    after: "future::join_all(tasks)",
    message: "code action providers must be capped before join_all",
  });
  assertBefore({
    body: refreshBody,
    before: "push_bounded_code_actions_for_provider(",
    after: "Rc::from(actions)",
    message: "provider actions must be capped before Rc menu storage",
  });
});

test("debug scenario fanout is capped before joining and collecting scenario rows", () => {
  const source = read("crates/editor/src/code_actions.rs");
  const body = functionBody(source, "debug_scenarios");

  assert.match(source, /const MAX_DEBUG_SCENARIO_TASKS: usize = 128;/);
  assert.match(source, /const MAX_DEBUG_SCENARIO_RESULTS: usize = 256;/);
  assertBefore({
    body,
    before: "MAX_DEBUG_SCENARIO_TASKS",
    after: "futures::future::join_all(scenarios)",
    message: "debug scenario tasks must be capped before join_all",
  });
  assertBefore({
    body,
    before: ".take(MAX_DEBUG_SCENARIO_RESULTS)",
    after: ".collect::<Vec<_>>()",
    message: "debug scenario results must be capped before Vec materialization",
  });
});

test("code lens fetch and block paths cap visible buffers, actions, resolves, and UI blocks", () => {
  const source = read("crates/editor/src/code_lens.rs");
  const refreshBody = functionBody(source, "refresh_code_lenses");
  const applyBody = functionBody(source, "apply_lens_actions_for_buffer");
  const resolveBody = functionBody(source, "resolve_visible_code_lenses");

  assert.match(source, /const MAX_CODE_LENS_VISIBLE_BUFFERS: usize = 32;/);
  assert.match(source, /const MAX_CODE_LENS_ACTIONS_PER_BUFFER: usize = 4_096;/);
  assert.match(source, /const MAX_CODE_LENS_BLOCKS_PER_BUFFER: usize = 2_048;/);
  assert.match(source, /const MAX_CODE_LENS_RESOLVE_TASKS: usize = 2_048;/);
  assertBefore({
    body: refreshBody,
    before: ".take(MAX_CODE_LENS_VISIBLE_BUFFERS)",
    after: ".collect::<Vec<_>>()",
    message: "visible code-lens buffers must be capped before Vec collection",
  });
  assertBefore({
    body: refreshBody,
    before: ".take(MAX_CODE_LENS_VISIBLE_BUFFERS)",
    after: "join_all(tasks_per_buffer)",
    message: "visible code-lens buffers must be capped before join_all",
  });
  assertBefore({
    body: applyBody,
    before: "cap_code_lens_actions_for_buffer(",
    after: "actions.iter().sorted_by_key",
    message: "code lens actions must be capped before row grouping",
  });
  assertBefore({
    body: applyBody,
    before: ".take(MAX_CODE_LENS_BLOCKS_PER_BUFFER)",
    after: "let mut to_insert = Vec::new();",
    message: "code lens block rows must be capped before insertion vector materialization",
  });
  assertBefore({
    body: resolveBody,
    before: "MAX_CODE_LENS_RESOLVE_TASKS",
    after: ".collect::<FuturesUnordered<_>>()",
    message: "code lens resolve tasks must be capped before task collection",
  });
});

test("completion resolve and markdown parse fanout are capped around nearby candidates", () => {
  const source = read("crates/editor/src/code_context_menus.rs");
  const resolveBody = functionBody(source, "resolve_visible_completions");
  const markdownBody = functionBody(source, "start_markdown_parse_for_nearby_entries");

  assert.match(source, /const MAX_COMPLETION_RESOLVE_CANDIDATES: usize = 256;/);
  assert.match(source, /const MAX_COMPLETION_MARKDOWN_PARSE_CANDIDATES: usize = 16;/);
  assertBefore({
    body: resolveBody,
    before: ".take(MAX_COMPLETION_RESOLVE_CANDIDATES)",
    after: ".collect::<Vec<usize>>()",
    message: "completion resolve candidate ids must be capped before Vec materialization",
  });
  assertBefore({
    body: resolveBody,
    before: ".take(MAX_COMPLETION_RESOLVE_CANDIDATES)",
    after: "provider.resolve_completions(",
    message: "completion resolve candidate ids must be capped before provider fanout",
  });
  assertBefore({
    body: markdownBody,
    before: ".take(MAX_COMPLETION_MARKDOWN_PARSE_CANDIDATES)",
    after: "self.get_or_create_entry_markdown(index, cx);",
    message: "nearby completion markdown parse fanout must be capped before parse creation",
  });
});

test("snippet fuzzy completion skips stale candidate ids before snippet lookup", () => {
  const source = read("crates/editor/src/completions.rs");
  const body = functionBody(source, "snippet_completions");

  assert.match(
    body,
    /sorted_snippet_candidates\s*\.get\(string_match\.candidate_id\)/,
    "snippet fuzzy matches must guard stale candidate ids before candidate lookup",
  );
  assert.doesNotMatch(
    body,
    /sorted_snippet_candidates\s*\[\s*string_match\.candidate_id\s*\]/,
    "snippet fuzzy matches must not directly index sorted candidates by candidate_id",
  );
  assertBefore({
    body,
    before: /sorted_snippet_candidates\s*\.get\(string_match\.candidate_id\)/,
    after: /snippets\.get\(snippet_index\)/,
    message: "snippet candidate ids must be guarded before snippet lookup",
  });
  assert.doesNotMatch(
    body,
    /let snippet = &snippets\[\s*snippet_index\s*\]/,
    "snippet lookup derived from fuzzy candidates must use get()",
  );
});

test("completion menu fuzzy lookups skip stale candidate ids before completion lookup", () => {
  const source = read("crates/editor/src/code_context_menus.rs");
  const resolveBody = functionBody(source, "resolve_visible_completions");

  assert.doesNotMatch(
    source,
    /(?:completions|completions_guard|completions_ref)\s*\[\s*(?:\*i|candidate_id|mat\.candidate_id|result\.candidate_id|string_match\.candidate_id)\s*\]/,
    "completion menu fuzzy lookups must not directly index completions by candidate_id",
  );
  assert.match(
    source,
    /(?:completions|completions_guard|completions_ref)\s*\.get\(\s*(?:\*i|candidate_id|mat\.candidate_id|result\.candidate_id|string_match\.candidate_id)\s*\)/,
    "completion menu fuzzy lookups should use get() for candidate ids",
  );
  assert.doesNotMatch(
    resolveBody,
    /entries\s*\[\s*self\.selected_item\s*\]/,
    "completion resolution must not directly index selected entries",
  );
  assert.doesNotMatch(
    source,
    /entries\s*\[\s*self\.selected_item\s*\]/,
    "completion menu selected-entry paths should use get() instead of direct indexing",
  );
  assert.match(
    resolveBody,
    /entries\s*\.get\(self\.selected_item\)/,
    "completion resolution should guard stale selected entries",
  );
});
