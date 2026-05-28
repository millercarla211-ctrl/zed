import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const source = productionSource(
  read("crates/agent_ui/src/completion_provider.rs"),
);

const sliceBetween = (haystack: string, start: string, end: string) => {
  const startIndex = haystack.indexOf(start);
  assert.notEqual(startIndex, -1, `expected ${start}`);
  const endIndex = haystack.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `expected ${end} after ${start}`);
  return haystack.slice(startIndex, endIndex + end.length);
};

const assertBefore = (
  haystack: string,
  before: string,
  after: string,
  message: string,
) => {
  const beforeIndex = haystack.indexOf(before);
  const afterIndex = haystack.indexOf(after);
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("prompt completion provider names collection caps", () => {
  assert.match(
    source,
    /const MAX_SLASH_COMPLETION_SKILL_CANDIDATES: usize = 1024;/,
  );
  assert.match(
    source,
    /const MAX_SLASH_COMPLETION_COMMAND_CANDIDATES: usize = 1024;/,
  );
  assert.match(
    source,
    /const MAX_SKILL_MENTION_COMPLETION_CANDIDATES: usize = 1024;/,
  );
  assert.match(source, /const MAX_THREAD_COMPLETION_CANDIDATES: usize = 256;/);
  assert.match(
    source,
    /const MAX_PROMPT_COMPLETION_VISIBLE_WORKTREES: usize = 128;/,
  );
  assert.match(
    source,
    /const MAX_EMPTY_FILE_COMPLETION_MATCHES: usize = 512;/,
  );
  assert.match(
    source,
    /const MAX_SYMBOL_COMPLETION_CANDIDATES: usize = 10_000;/,
  );
});

test("slash command completions cap skill and command candidates before materializing labels", () => {
  const searchSlash = sliceBetween(
    source,
    "fn search_slash_commands(",
    "fn fetch_branch_diff_match(",
  );

  assertBefore(
    searchSlash,
    ".take(MAX_SLASH_COMPLETION_SKILL_CANDIDATES)",
    ".map(SlashCompletionCandidate::Skill)",
    "slash skills must be capped before becoming completion candidates",
  );
  assertBefore(
    searchSlash,
    ".take(MAX_SLASH_COMPLETION_COMMAND_CANDIDATES)",
    ".map(SlashCompletionCandidate::Command)",
    "slash commands must be capped before becoming completion candidates",
  );
  assertBefore(
    searchSlash,
    ".take(MAX_SLASH_COMPLETION_SKILL_CANDIDATES)",
    ".collect::<Vec<_>>()",
    "slash skills must be capped before collecting the candidate vector",
  );
});

test("file completions cap visible worktrees and empty-query file rows", () => {
  const searchFiles = sliceBetween(
    source,
    "pub(crate) fn search_files(",
    "pub(crate) fn search_symbols(",
  );
  const emptyQuery = sliceBetween(
    searchFiles,
    "if query.is_empty() {",
    "Task::ready(recent_matches.chain(file_matches).collect())",
  );
  const queried = sliceBetween(
    searchFiles,
    "} else {\n        let workspace = workspace.read(cx);",
    "let executor = cx.background_executor().clone();",
  );

  assert.match(
    source,
    /fn has_multiple_visible_worktrees\(workspace: &Workspace, cx: &App\) -> bool \{\s+workspace\.visible_worktrees\(cx\)\.take\(2\)\.count\(\) > 1\s+\}/,
  );
  assertBefore(
    emptyQuery,
    "has_multiple_visible_worktrees(workspace, cx)",
    ".visible_worktrees(cx)",
    "empty file completions should determine root-label mode without counting every worktree",
  );
  assertBefore(
    emptyQuery,
    ".take(MAX_PROMPT_COMPLETION_VISIBLE_WORKTREES)",
    ".collect::<Vec<_>>()",
    "empty file completions must cap visible worktrees before collecting them",
  );
  assertBefore(
    emptyQuery,
    ".take(MAX_EMPTY_FILE_COMPLETION_MATCHES)",
    "Task::ready(recent_matches.chain(file_matches).collect())",
    "empty file completions must cap file rows before collecting the UI response",
  );
  assertBefore(
    queried,
    "has_multiple_visible_worktrees(workspace, cx)",
    ".visible_worktrees(cx)",
    "queried file completions should determine root-label mode without counting every worktree",
  );
  assertBefore(
    queried,
    ".take(MAX_PROMPT_COMPLETION_VISIBLE_WORKTREES)",
    ".collect::<Vec<_>>()",
    "queried file completions must cap visible worktrees before path candidate sets are collected",
  );
  assert.doesNotMatch(searchFiles, /visible_worktrees\(cx\)\s*\.collect::<Vec<_>>/);
  assert.doesNotMatch(searchFiles, /visible_worktrees\(cx\)\.count\(\)/);
});

test("symbol, thread, and skill mention completions cap searchable candidates", () => {
  const searchSymbols = sliceBetween(
    source,
    "pub(crate) fn search_symbols(",
    "fn collect_session_matches(",
  );
  const collectSessions = sliceBetween(
    source,
    "fn collect_session_matches(",
    "fn filter_sessions_by_query(",
  );
  const collectSessionMetadata = sliceBetween(
    source,
    "fn collect_recent_session_metadata",
    "fn filter_sessions_by_query(",
  );
  const searchSkills = sliceBetween(
    source,
    "pub(crate) fn search_skills(",
    "pub struct SymbolMatch",
  );

  assertBefore(
    searchSymbols,
    ".take(MAX_SYMBOL_COMPLETION_CANDIDATES)",
    ".map(|(id, symbol)|",
    "symbol candidates must be capped before StringMatchCandidate allocation",
  );
  assertBefore(
    searchSymbols,
    ".take(MAX_SYMBOL_COMPLETION_CANDIDATES)",
    ".partition(|candidate|",
    "symbol candidates must be capped before partition materialization",
  );
  assertBefore(
    collectSessions,
    "collect_recent_session_metadata(",
    ".map(|metadata|",
    "thread metadata must be bounded before becoming completion matches",
  );
  assert.match(
    collectSessionMetadata,
    /let mut entries = Vec::new\(\);/,
    "thread metadata collection should start with a bounded accumulator",
  );
  assert.match(
    collectSessionMetadata,
    /insert_recent_session_metadata\(&mut entries, metadata\);/,
    "thread metadata should be inserted through the bounded top-recent helper",
  );
  assertBefore(
    collectSessionMetadata,
    "entries.partition_point",
    "entries.insert(insert_at, metadata);",
    "thread metadata must find a bounded insertion point before materializing the row",
  );
  assertBefore(
    collectSessionMetadata,
    "entries.len() > MAX_THREAD_COMPLETION_CANDIDATES",
    "entries.pop();",
    "thread metadata must shed overflow rows immediately after insertion",
  );
  assert.doesNotMatch(
    collectSessionMetadata,
    /\.collect\(\)|sort_by_key/,
    "thread metadata must not collect or sort the full matching history before capping",
  );
  assertBefore(
    searchSkills,
    ".take(MAX_SKILL_MENTION_COMPLETION_CANDIDATES)",
    ".collect::<Vec<_>>();",
    "skill mention inputs must be capped before collecting searchable skills",
  );
  assertBefore(
    searchSkills,
    ".take(MAX_SKILL_MENTION_COMPLETION_CANDIDATES)",
    ".map(|(id, skill)|",
    "skill mentions must be capped before StringMatchCandidate allocation",
  );
});
