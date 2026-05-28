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

test("file finder history tasks are capped before join_all", () => {
  const source = read("crates/file_finder/src/file_finder.rs");
  const open = functionBody(source, "open");

  assert.match(source, /const MAX_FILE_FINDER_HISTORY_TASKS: usize = MAX_RECENT_SELECTIONS;/);
  assertBefore({
    body: open,
    before: "if history_item_tasks.len() >= MAX_FILE_FINDER_HISTORY_TASKS",
    after: "history_item_tasks.push(",
    message: "history task fanout must check the cap before pushing tasks",
  });
  assertBefore({
    body: open,
    before: "MAX_FILE_FINDER_HISTORY_TASKS",
    after: "join_all(history_item_tasks)",
    message: "history task fanout must be bounded before join_all",
  });
  assert.doesNotMatch(
    open,
    /\.filter_map\([\s\S]*\.collect::<Vec<_>>\(\)/,
    "history tasks should not be collected from an unbounded iterator chain",
  );
});

test("file finder search materialization has named caps before vectors", () => {
  const source = read("crates/file_finder/src/file_finder.rs");
  const spawnSearch = functionBody(source, "spawn_search");
  const pushNewMatches = functionBody(source, "push_new_matches");
  const matchingHistoryItems = functionBody(source, "matching_history_items");

  assert.match(source, /const MAX_FILE_FINDER_RESULTS: usize = 100;/);
  assert.match(source, /const MAX_FILE_FINDER_SEARCH_MATCHES: usize = MAX_FILE_FINDER_RESULTS;/);
  assert.match(source, /const MAX_FILE_FINDER_WORKTREE_CANDIDATE_SETS: usize = 1_024;/);
  assert.match(source, /const MAX_FILE_FINDER_HISTORY_ITEMS: usize = 256;/);
  assertBefore({
    body: spawnSearch,
    before: "if candidate_sets.len() >= MAX_FILE_FINDER_WORKTREE_CANDIDATE_SETS",
    after: "candidate_sets.push(PathMatchCandidateSet",
    message: "worktree candidate sets must be capped before vector push",
  });
  assertBefore({
    body: pushNewMatches,
    before: ".take(MAX_FILE_FINDER_SEARCH_MATCHES)",
    after: ".collect()",
    message: "path matches must be capped before result-vector materialization",
  });
  assertBefore({
    body: matchingHistoryItems,
    before: "if processed_history_items >= MAX_FILE_FINDER_HISTORY_ITEMS",
    after: "PathMatchCandidate::new",
    message: "history path candidates must be capped before fuzzy materialization",
  });
  assert.match(
    pushNewMatches,
    /if self\.matches\.len\(\) >= MAX_FILE_FINDER_RESULTS \{/,
    "final result insertion must use the named result cap",
  );
});

test("file finder workspace-driven channel and create-new lists are bounded", () => {
  const source = read("crates/file_finder/src/file_finder.rs");
  const setSearchMatches = functionBody(source, "set_search_matches");
  const collectWorktrees = functionBody(source, "collect_available_worktrees_for_create_new");

  assert.match(source, /const MAX_FILE_FINDER_CHANNELS: usize = 2_048;/);
  assertBefore({
    body: setSearchMatches,
    before: "if channels.len() >= MAX_FILE_FINDER_CHANNELS",
    after: "channels.push(channel);",
    message: "channels must be capped before collecting cloned channel state",
  });
  assertBefore({
    body: setSearchMatches,
    before: "if channel_matches.len() >= MAX_FILE_FINDER_RESULTS",
    after: "channel_matches.push(Match::Channel",
    message: "channel match vectors must be capped before result materialization",
  });
  assertBefore({
    body: collectWorktrees,
    before: "if worktrees.len() >= MAX_FILE_FINDER_WORKTREE_CANDIDATE_SETS",
    after: "worktrees.push(worktree);",
    message: "create-new worktree lists must be capped before collection",
  });
  assert.match(
    setSearchMatches,
    /let Some\(available_worktree\) =\s+self\.collect_available_worktrees_for_create_new\(cx\)\s+else \{\s+break 'create_new;\s+\};/,
    "create-new affordance must fail closed when worktree collection overflows",
  );
});

test("file finder rejects oversized user queries before parsing or fuzzy search", () => {
  const source = read("crates/file_finder/src/file_finder.rs");
  const updateMatches = functionBody(source, "update_matches");

  assert.match(source, /const MAX_FILE_FINDER_QUERY_CHARS: usize = 4_096;/);
  assert.match(source, /fn query_exceeds_file_finder_limit\(query: &str\) -> bool/);
  assertBefore({
    body: updateMatches,
    before: "if query_exceeds_file_finder_limit(raw_query)",
    after: "parse_file_search_query(raw_query)",
    message: "oversized queries must fail closed before query/result materialization",
  });
  assertBefore({
    body: updateMatches,
    before: "self.cancel_flag.store(true, atomic::Ordering::Release);",
    after: "return Task::ready(());",
    message: "oversized queries must cancel in-flight fuzzy work before returning",
  });
});
