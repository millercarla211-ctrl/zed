import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const inlaySource = read("crates/editor/src/inlays/inlay_hints.rs");
const semanticSource = read("crates/editor/src/semantic_tokens.rs");

const functionBody = (source: string, name: string) => {
  const start = source.indexOf(`fn ${name}(`);
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

const indexOfPattern = (source: string, pattern: string | RegExp) => {
  if (typeof pattern === "string") {
    return source.indexOf(pattern);
  }

  return source.match(pattern)?.index ?? -1;
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

test("editor inlay and semantic token caps are named constants", () => {
  assert.match(inlaySource, /const MAX_VISIBLE_INLAY_HINT_RANGES: usize = 4_096;/);
  assert.match(inlaySource, /const MAX_VISIBLE_INLAY_HINT_BUFFERS: usize = 512;/);
  assert.match(inlaySource, /const MAX_INLAY_HINT_RANGES_PER_BUFFER: usize = 4_096;/);
  assert.match(inlaySource, /const MAX_INLAY_HINT_CHUNKS_PER_BUFFER: usize = 4_096;/);
  assert.match(
    inlaySource,
    /const MAX_INLAY_HINT_REQUEST_TASKS_PER_BUFFER: usize = 4_096;/,
  );
  assert.match(inlaySource, /const MAX_INLAY_HINT_RESULTS_PER_REFRESH: usize = 100_000;/);
  assert.match(inlaySource, /const MAX_INLAY_HINT_LABEL_BYTES: usize = 16 \* 1024;/);
  assert.match(inlaySource, /const MAX_INLAY_HINT_LABEL_PARTS: usize = 512;/);
  assert.match(inlaySource, /const MAX_INLAY_HINT_UI_SPLICE_ITEMS: usize = 100_000;/);

  assert.match(semanticSource, /const MAX_SEMANTIC_TOKEN_VISIBLE_BUFFERS: usize = 512;/);
  assert.match(semanticSource, /const MAX_SEMANTIC_TOKEN_FETCH_TASKS: usize = 512;/);
  assert.match(semanticSource, /const MAX_SEMANTIC_DISABLED_BUFFER_INVALIDATIONS: usize = 4_096;/);
  assert.match(semanticSource, /const MAX_SEMANTIC_TOKEN_SERVERS_PER_BUFFER: usize = 64;/);
  assert.match(semanticSource, /const MAX_SEMANTIC_TOKENS_PER_SERVER: usize = 250_000;/);
  assert.match(semanticSource, /const MAX_SEMANTIC_HIGHLIGHTS_PER_BUFFER: usize = 250_000;/);
});

test("inlay hint request fanout is capped before task joins", () => {
  const refresh = functionBody(inlaySource, "refresh_inlay_hints");
  assertBefore({
    body: refresh,
    before: "visible_excerpts.truncate(MAX_VISIBLE_INLAY_HINT_RANGES)",
    after: "let mut buffers_to_query = HashMap::default();",
    message: "visible inlay ranges must be capped before buffer query materialization",
  });
  assertBefore({
    body: refresh,
    before: "buffers_to_query.len() >= MAX_VISIBLE_INLAY_HINT_BUFFERS",
    after: ".entry(buffer_id)",
    message: "visible inlay buffer fanout must be capped before entry allocation",
  });
  assertBefore({
    body: refresh,
    before: "visible_excerpts.ranges.len() >= MAX_INLAY_HINT_RANGES_PER_BUFFER",
    after: "visible_excerpts.ranges.push(buffer_anchor_range)",
    message: "per-buffer inlay range fanout must be capped before range push",
  });
  assertBefore({
    body: refresh,
    before: "applicable_chunks.truncate(MAX_INLAY_HINT_CHUNKS_PER_BUFFER)",
    after: "spawn_editor_hints_refresh",
    message: "inlay chunk fanout must be capped before spawning refresh tasks",
  });

  const hintsForBuffer = functionBody(inlaySource, "inlay_hints_for_buffer");
  assertBefore({
    body: hintsForBuffer,
    before: "hint_tasks.len() >= MAX_INLAY_HINT_REQUEST_TASKS_PER_BUFFER",
    after: ".push(cx.spawn",
    message: "inlay request task fanout must be capped before task push",
  });

  const spawnRefresh = functionBody(inlaySource, "spawn_editor_hints_refresh");
  assertBefore({
    body: spawnRefresh,
    before: "cap_inlay_hint_tasks_for_join(buffer_id, hint_tasks)",
    after: "join_all(hint_tasks).await",
    message: "inlay request tasks must be capped before join_all",
  });
});

test("inlay hint result and splice vectors are capped before materialization", () => {
  const apply = functionBody(inlaySource, "apply_fetched_hints");
  assertBefore({
    body: apply,
    before: "collect_visible_inlay_hint_ids_for_buffer(",
    after: "self.splice_inlays(&hints_to_remove, hints_to_insert, cx);",
    message: "visible inlay removals must go through a bounded collector before splice",
  });
  assertBefore({
    body: apply,
    before: "push_bounded_inlay_hint_result(",
    after: "new_hints_to_insert.sort_by",
    message: "inlay results must be capped before sort materialization",
  });

  const pushResult = functionBody(inlaySource, "push_bounded_inlay_hint_result");
  assertBefore({
    body: pushResult,
    before: "inlay_hint_label_text_for_cache(buffer_id, &new_hint)",
    after: /inserted_hint_text\s*\.entry\(new_hint\.position\)/,
    message: "inlay cache-label dedupe must bound label text before map insertion",
  });
  assertBefore({
    body: pushResult,
    before: "new_hints_to_insert.len() >= MAX_INLAY_HINT_RESULTS_PER_REFRESH",
    after: "new_hints_to_insert.push((new_id, new_hint))",
    message: "inlay result vectors must be capped before pushing fetched hints",
  });
  assertBefore({
    body: apply,
    before: "hints_to_insert.len() >= MAX_INLAY_HINT_UI_SPLICE_ITEMS",
    after: "hints_to_insert.push(Inlay::hint(*hint_id, position, hint))",
    message: "inlay UI insertion vectors must be capped before splice insertion",
  });
  assert.doesNotMatch(
    apply,
    /\.sorted_by\(/,
    "inlay results should be capped before using Vec sort, not materialized through Itertools",
  );
});

test("semantic token fetch fanout is capped before joins and collections", () => {
  const refresh = functionBody(semanticSource, "refresh_semantic_tokens");
  assertBefore({
    body: refresh,
    before: "buffers_to_query.len() >= MAX_SEMANTIC_TOKEN_VISIBLE_BUFFERS",
    after: "buffers_to_query.insert(editor_buffer_id, editor_buffer)",
    message: "semantic visible-buffer fanout must be capped before query map insertion",
  });
  assertBefore({
    body: refresh,
    before: "disabled_buffer_invalidations.len() >= MAX_SEMANTIC_DISABLED_BUFFER_INVALIDATIONS",
    after: "disabled_buffer_invalidations.push(buffer_id)",
    message: "semantic invalidation vectors must be capped before push",
  });
  assertBefore({
    body: refresh,
    before: "semantic_token_tasks.len() >= MAX_SEMANTIC_TOKEN_FETCH_TASKS",
    after: /semantic_token_tasks\s*\.push\(/,
    message: "semantic token task fanout must be capped before task push",
  });
  assertBefore({
    body: refresh,
    before: "cap_semantic_token_tasks_for_join(semantic_token_tasks)",
    after: "join_all(semantic_token_tasks).await",
    message: "semantic token tasks must be capped before join_all",
  });
  assert.doesNotMatch(
    refresh,
    /\.collect::<HashMap<_, _>>\(\)/,
    "semantic token buffers should not be collected through an unbounded iterator",
  );
  assert.doesNotMatch(
    refresh,
    /\.collect::<Vec<_>>\(\)/,
    "semantic token invalidations and tasks should not use unbounded Vec collection",
  );
});

test("semantic token response and highlight vectors are capped before rendering", () => {
  const refresh = functionBody(semanticSource, "refresh_semantic_tokens");
  assertBefore({
    body: refresh,
    before: "server_index >= MAX_SEMANTIC_TOKEN_SERVERS_PER_BUFFER",
    after: "token_highlights.extend",
    message: "semantic token server fanout must be capped before highlight extension",
  });
  assertBefore({
    body: refresh,
    before: "server_tokens.len().min(MAX_SEMANTIC_TOKENS_PER_SERVER)",
    after: "buffer_into_editor_highlights(",
    message: "semantic token response size must be capped before highlight conversion",
  });
  assertBefore({
    body: refresh,
    before: /MAX_SEMANTIC_HIGHLIGHTS_PER_BUFFER\s*\.saturating_sub\(token_highlights\.len\(\)\)/,
    after: "token_highlights.extend",
    message: "semantic highlight vectors must be capped before extending render data",
  });
  assertBefore({
    body: refresh,
    before: "token_highlights.len() >= MAX_SEMANTIC_HIGHLIGHTS_PER_BUFFER",
    after: "token_highlights.sort_by",
    message: "semantic highlight vectors must be capped before sorting",
  });
  assertBefore({
    body: refresh,
    before: "token_highlights.sort_by",
    after: "Arc::from(token_highlights)",
    message: "semantic highlights must be sorted only after bounded materialization",
  });
});
