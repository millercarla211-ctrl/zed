import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/agent_ui/src/terminal_thread_metadata_store.rs";
const source = readFileSync(sourcePath, "utf8");
const productionSource =
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

function functionBody(name: string): string {
  const signature = new RegExp(
    `\\n(?:    )?(?:pub(?:\\([^)]*\\))?\\s+)?fn ${name}\\(`,
  );
  const match = signature.exec(productionSource);
  assert.ok(match?.index, `expected ${name}`);

  const start = match.index + 1;
  const openBrace = productionSource.indexOf("{", start);
  assert.ok(openBrace > start, `expected ${name} to have a body`);

  let depth = 0;
  for (let index = openBrace; index < productionSource.length; index++) {
    const char = productionSource[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return productionSource.slice(start, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
}

function assertBefore(
  haystack: string,
  before: string | RegExp,
  after: string | RegExp,
  message: string,
) {
  const indexOfPattern = (pattern: string | RegExp) => {
    if (typeof pattern === "string") {
      return haystack.indexOf(pattern);
    }
    return haystack.match(pattern)?.index ?? -1;
  };

  const beforeIndex = indexOfPattern(before);
  const afterIndex = indexOfPattern(after);
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("terminal thread metadata hardening caps are named and explicit", () => {
  for (const pattern of [
    /const MAX_TERMINAL_THREAD_METADATA_DB_ROWS: usize = 10_000;/,
    /const MAX_TERMINAL_THREAD_METADATA_PENDING_DB_OPERATIONS: usize = 2_048;/,
    /const MAX_TERMINAL_THREAD_METADATA_STRING_BYTES: usize = 16 \* 1024;/,
    /const MAX_TERMINAL_THREAD_METADATA_PATH_LIST_ENTRIES: usize = 512;/,
    /const MAX_TERMINAL_THREAD_METADATA_PATH_LIST_BYTES: usize = 256 \* 1024;/,
    /const MAX_TERMINAL_THREAD_REMOTE_CONNECTION_JSON_BYTES: usize = 64 \* 1024;/,
  ]) {
    assert.match(productionSource, pattern);
  }
});

test("database rows are capped before terminal metadata is cached or indexed", () => {
  const reload = functionBody("reload");
  assertBefore(
    reload,
    "db.list()",
    "bounded_terminal_metadata_rows(rows)",
    "terminal metadata DB rows must be capped immediately after db.list()",
  );
  assertBefore(
    reload,
    "bounded_terminal_metadata_rows(rows)",
    "cache_terminal_metadata(row)",
    "terminal metadata DB rows must be capped before cache/index insertion",
  );
  assert.match(reload, /bounded_terminal_metadata\(row,\s*"database load"\)/);

  const list = functionBody("list");
  assert.match(list, /LIMIT \?1/);
  assert.match(list, /terminal_thread_metadata_db_list_limit\(\)/);

  const limit = functionBody("terminal_thread_metadata_db_list_limit");
  assert.match(
    limit,
    /MAX_TERMINAL_THREAD_METADATA_DB_ROWS\s*\.saturating_add\(1\)/,
  );

  const rows = functionBody("bounded_terminal_metadata_rows");
  assert.match(rows, /rows\.truncate\(MAX_TERMINAL_THREAD_METADATA_DB_ROWS\)/);
  assert.match(rows, /log::warn!\(/);
  assert.match(rows, /capped/);
});

test("pending terminal DB operations are queue- and drain-bounded before deduplication", () => {
  assert.match(
    productionSource,
    /async_channel::bounded\(MAX_TERMINAL_THREAD_METADATA_PENDING_DB_OPERATIONS\)/,
  );

  const drain = functionBody("drain_pending_terminal_db_operations");
  assert.match(drain, /rx\.try_recv\(\)/);
  assert.match(
    drain,
    /updates\.len\(\) < MAX_TERMINAL_THREAD_METADATA_PENDING_DB_OPERATIONS/,
  );
  assert.match(drain, /rx\.len\(\)/);
  assert.match(drain, /log::warn!\(/);
  assert.match(drain, /deferred/);

  const worker = productionSource.slice(
    productionSource.indexOf("while let Ok(first_update) = rx.recv().await"),
    productionSource.indexOf("let mut this = Self {"),
  );
  assertBefore(
    worker,
    "Self::drain_pending_terminal_db_operations(first_update, &rx)",
    "Self::dedup_db_operations(updates)",
    "pending operation batches must be capped before dedup/work",
  );

  const queue = functionBody("queue_db_operation");
  assert.match(queue, /try_send\(operation\)/);
  assert.match(queue, /send_blocking\(operation\)/);
  assert.match(queue, /backpressured/);
  assert.match(queue, /log::warn!\(/);
});

test("terminal metadata display strings and path indexes are bounded with visible warnings", () => {
  const text = functionBody("bounded_terminal_metadata_text");
  assert.match(text, /MAX_TERMINAL_THREAD_METADATA_STRING_BYTES/);
  assert.match(text, /truncate_to_byte_limit/);
  assert.match(text, /log::warn!\(/);
  assert.match(text, /truncated/);

  const metadata = functionBody("bounded_terminal_metadata");
  for (const field of ["title", "custom_title", "working_directory", "worktree_paths"]) {
    assert.match(metadata, new RegExp(field));
  }
  assert.match(metadata, /bounded_terminal_metadata_working_directory/);
  assert.match(metadata, /bounded_terminal_metadata_worktree_paths/);

  const workingDirectory = functionBody(
    "bounded_terminal_metadata_working_directory",
  );
  assert.match(workingDirectory, /MAX_TERMINAL_THREAD_METADATA_STRING_BYTES/);
  assert.match(workingDirectory, /log::warn!\(/);
  assert.match(workingDirectory, /skipped/);

  const worktreePaths = functionBody("bounded_terminal_metadata_worktree_paths");
  assert.match(worktreePaths, /ordered_pairs\(\)/);
  assert.match(worktreePaths, /MAX_TERMINAL_THREAD_METADATA_PATH_LIST_ENTRIES/);
  assert.match(worktreePaths, /MAX_TERMINAL_THREAD_METADATA_PATH_LIST_BYTES/);
  assert.match(worktreePaths, /WorktreePaths::from_path_lists/);
  assert.match(worktreePaths, /log::warn!\(/);
  assert.match(worktreePaths, /capped/);
});

test("serialized path lists are bounded before deserialize or database storage", () => {
  const deserialize = functionBody("deserialize_bounded_terminal_path_list");
  assert.match(deserialize, /MAX_TERMINAL_THREAD_METADATA_PATH_LIST_BYTES/);
  assert.match(deserialize, /MAX_TERMINAL_THREAD_METADATA_PATH_LIST_ENTRIES/);
  assertBefore(
    deserialize,
    /serialized_path_list_entry_count\(&paths\)/,
    "PathList::deserialize",
    "serialized path-list entry counts must be checked before deserializing",
  );
  assert.match(deserialize, /log::warn!\(/);
  assert.match(deserialize, /skipped/);

  const column = productionSource.slice(
    productionSource.indexOf("impl Column for TerminalThreadMetadata"),
    productionSource.length,
  );
  assert.doesNotMatch(
    column,
    /\.map\(\|paths\|[\s\S]*?PathList::deserialize/,
    "terminal metadata columns must not directly deserialize unbounded path-list strings",
  );
  assert.match(column, /deserialize_bounded_terminal_path_list\(/);

  const serialize = functionBody("serialize_bounded_terminal_path_list");
  assert.match(serialize, /path_list\.serialize\(\)/);
  assert.match(serialize, /MAX_TERMINAL_THREAD_METADATA_PATH_LIST_BYTES/);
  assert.match(serialize, /anyhow::bail!\(/);
});
