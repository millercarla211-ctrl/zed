import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const dock = read("crates/workspace/src/dock.rs");
const historyManager = read("crates/workspace/src/history_manager.rs");
const item = read("crates/workspace/src/item.rs");

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
  const indexOfPattern = (pattern: string | RegExp) => {
    if (typeof pattern === "string") {
      return body.indexOf(pattern);
    }
    return body.match(pattern)?.index ?? -1;
  };

  const beforeIndex = indexOfPattern(before);
  const afterIndex = indexOfPattern(after);
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("history entries cap workspace path materialization before collection", () => {
  assert.match(historyManager, /const MAX_HISTORY_ENTRY_PATHS: usize = 32;/);

  const entryImpl = historyManager.slice(
    historyManager.indexOf("impl HistoryManagerEntry"),
  );
  const newEntry = functionBody(entryImpl, "new");
  assert.doesNotMatch(
    newEntry,
    /ordered_paths\(\)[\s\S]*collect::<SmallVec/,
    "history entries must not use PathList::ordered_paths because it sorts before caller caps",
  );
  assertBefore({
    body: newEntry,
    before: /paths\.paths\(\)\.len\(\)\.min\(MAX_HISTORY_ENTRY_PATHS\)/,
    after: "path.push(source_path.compact());",
    message: "history path list must be capped before SmallVec push materialization",
  });
  assert.match(newEntry, /workspace history entry path list is too large/);
  assert.match(newEntry, /\.log_err\(\)/);
});

test("history deletion lists fail closed before delete-id collection", () => {
  assert.match(
    historyManager,
    /const MAX_JUMP_LIST_REMOVED_ENTRIES: usize = MAX_JUMP_LIST_ENTRIES;/,
  );

  const jumpList = functionBody(historyManager, "update_jump_list");
  assertBefore({
    body: jumpList,
    before: "if user_removed.len() > MAX_JUMP_LIST_REMOVED_ENTRIES",
    after: /let mut deleted_ids\s*=\s*Vec::with_capacity\(/,
    message: "jump-list removal payloads must be capped before delete-id materialization",
  });
  assert.match(jumpList, /refusing to process oversized jump-list removal payload/);
  assertBefore({
    body: jumpList,
    before: "deleted_ids.len() >= MAX_HISTORY_DELETION_IDS",
    after: "deleted_ids.push(entry.id);",
    message: "history delete-id collection must be capped before pushing ids",
  });
});

test("dock panel-size persist batches are bounded before deferred persistence", () => {
  assert.match(
    dock,
    /const MAX_PANEL_SIZE_STATE_PERSIST_BATCH: usize = 128;/,
  );

  const resizeAll = functionBody(dock, "resize_all_panels");
  assertBefore({
    body: resizeAll,
    before:
      /Vec::with_capacity\(\s*self\.panel_entries\s*\.len\(\)\s*\.min\(MAX_PANEL_SIZE_STATE_PERSIST_BATCH\),\s*\)/,
    after: "size_states_to_persist.push(",
    message: "panel-size persist batches must reserve only the capped batch size",
  });
  assertBefore({
    body: resizeAll,
    before: "size_states_to_persist.len() < MAX_PANEL_SIZE_STATE_PERSIST_BATCH",
    after: "size_states_to_persist.push(",
    message: "panel-size persist batches must cap entries before vector push",
  });
  assertBefore({
    body: resizeAll,
    before: "skipped_panel_size_state_persist_count",
    after: "cx.defer(move |cx|",
    message: "oversized panel-size persist batches must warn before deferred persistence",
  });
});

test("item project-handle collections cap visited items before pushing handles", () => {
  assert.match(item, /const MAX_PROJECT_ITEMS_PER_ITEM: usize = 512;/);

  const itemHandleImpl = item.slice(
    item.indexOf("impl<T: Item> ItemHandle for Entity<T>"),
  );
  for (const name of [
    "project_entry_ids",
    "project_paths",
    "project_item_model_ids",
  ]) {
    const body = functionBody(itemHandleImpl, name);
    assertBefore({
      body,
      before: "if !should_collect_project_item",
      after: "result.push(",
      message: `${name} must cap project-item visits before pushing handles`,
    });
    assert.match(body, /log_project_item_collection_truncated/);
  }

  const helper = functionBody(item, "should_collect_project_item");
  assertBefore({
    body: helper,
    before: "*visited >= MAX_PROJECT_ITEMS_PER_ITEM",
    after: "*visited += 1;",
    message: "project-item collection helper must check cap before incrementing visits",
  });
});
