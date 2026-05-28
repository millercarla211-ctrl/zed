import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

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

test("random workspace discovery caps seed and sibling home enumeration before sorting", () => {
  const source = read("crates/workspace/src/multi_workspace.rs");

  assert.match(source, /const MAX_SPACE_SEED_SCAN_ENTRIES: usize = 128;/);
  assert.match(source, /const MAX_SPACE_SEED_CHILDREN: usize = 24;/);
  assert.match(source, /const MAX_SIBLING_HOME_SCAN_ENTRIES: usize = 32;/);
  assert.match(source, /const MAX_SIBLING_HOME_SEEDS: usize = 8;/);

  const seedCollector = functionBody(source, "collect_space_candidates_from_seed");
  assertBefore({
    body: seedCollector,
    before: ".take(MAX_SPACE_SEED_SCAN_ENTRIES)",
    after: ".collect::<Vec<_>>()",
    message: "seed directory scans must cap entries before child list materialization",
  });
  assertBefore({
    body: seedCollector,
    before: ".collect::<Vec<_>>()",
    after: "child_dirs.sort();",
    message: "seed child sorting should operate on the bounded child list",
  });
  assert.match(seedCollector, /for child_dir in child_dirs\.into_iter\(\)\.take\(MAX_SPACE_SEED_CHILDREN\)/);

  const randomWorkspace = functionBody(source, "create_random_local_workspace");
  assertBefore({
    body: randomWorkspace,
    before: ".take(MAX_SIBLING_HOME_SCAN_ENTRIES)",
    after: ".collect::<Vec<_>>()",
    message: "sibling home scans must cap entries before materialization",
  });
  assertBefore({
    body: randomWorkspace,
    before: ".collect::<Vec<_>>()",
    after: "sibling_homes.sort();",
    message: "sibling home sorting should operate on the bounded sibling list",
  });
  assertBefore({
    body: randomWorkspace,
    before: "sibling_homes.truncate(MAX_SIBLING_HOME_SEEDS);",
    after: "seeds.extend(sibling_homes);",
    message: "sibling home seeds must be item-capped before seed expansion",
  });
  assert.doesNotMatch(randomWorkspace, /take\(8\)/);
});

test("workspace history materialization is capped before jump-list rendering", () => {
  const source = read("crates/workspace/src/history_manager.rs");
  const persistence = read("crates/workspace/src/persistence.rs");

  assert.match(source, /const MAX_HISTORY_ENTRIES: usize = 256;/);
  assert.match(source, /const MAX_HISTORY_DB_RECENT_WORKSPACE_ROWS: usize = MAX_HISTORY_ENTRIES \* 4;/);
  assert.match(source, /const MAX_JUMP_LIST_ENTRIES: usize = 64;/);
  assert.match(
    persistence,
    /fn recent_workspaces_limited_query\(max_rows: i64\)[\s\S]*ORDER BY timestamp DESC[\s\S]*LIMIT \?1/,
  );
  assert.match(
    persistence,
    /pub async fn recent_project_workspaces_limited\(/,
  );

  const historyManagerImpl = source.slice(source.indexOf("impl HistoryManager"));
  const init = functionBody(historyManagerImpl, "init");
  assert.match(
    init,
    /recent_project_workspaces_limited\(\s*fs\.as_ref\(\),\s*MAX_HISTORY_DB_RECENT_WORKSPACE_ROWS,\s*\)/,
  );
  assertBefore({
    body: init,
    before: ".take(MAX_HISTORY_ENTRIES)",
    after: ".collect::<Vec<_>>()",
    message: "startup history must cap local recent entries before history vector materialization",
  });
  assertBefore({
    body: init,
    before: ".collect::<Vec<_>>()",
    after: "recent_folders.reverse();",
    message: "history reversal should preserve the bounded recent set order",
  });

  const updateHistory = functionBody(source, "update_history");
  assertBefore({
    body: updateHistory,
    before: "if self.history.len() > MAX_HISTORY_ENTRIES",
    after: "self.update_jump_list(cx);",
    message: "incremental history updates must trim before jump-list rendering",
  });
  assert.match(updateHistory, /let overflow = self\.history\.len\(\) - MAX_HISTORY_ENTRIES;/);
  assert.match(updateHistory, /self\.history\.drain\(\.\.overflow\);/);

  const jumpList = functionBody(source, "update_jump_list");
  assertBefore({
    body: jumpList,
    before: ".take(MAX_JUMP_LIST_ENTRIES)",
    after: ".collect::<Vec<_>>()",
    message: "jump-list entries must be capped before list materialization",
  });
  assertBefore({
    body: jumpList,
    before: ".collect::<Vec<_>>()",
    after: "cx.update_jump_list(menus, entries)",
    message: "jump-list rendering should receive only the bounded entry list",
  });
});
