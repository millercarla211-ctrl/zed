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
  const beforeIndex =
    typeof before === "string" ? body.indexOf(before) : body.match(before)?.index ?? -1;
  const afterIndex =
    typeof after === "string" ? body.indexOf(after) : body.match(after)?.index ?? -1;
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("worktree fs event batches are bounded before processing", () => {
  const source = read("crates/worktree/src/worktree.rs");
  const run = functionBody(source, "run");

  assert.match(source, /const MAX_WORKTREE_FS_EVENT_BATCH_EVENTS: usize = 8_192;/);
  assert.match(
    source,
    /async fn process_ready_fs_event_batches\(\s*&self,\s*paths: Vec<PathEvent>,\s*fs_events_rx: &mut Pin<Box<dyn Send \+ Stream<Item = Vec<PathEvent>>>>,/,
  );
  assert.match(
    source,
    /if pending_events\.len\(\) >= MAX_WORKTREE_FS_EVENT_BATCH_EVENTS \{/,
  );

  const firstDrain = "self.process_ready_fs_event_batches(paths, &mut fs_events_rx)";
  assert.equal(
    run.split(firstDrain).length - 1,
    2,
    "initial-scan and steady-state event drains should both use the bounded helper",
  );
  assert.doesNotMatch(
    run,
    /paths\.extend\(more_paths\)/,
    "ready filesystem events must not be extended into an unbounded batch",
  );
  assert.doesNotMatch(
    run,
    /\.filter\(\|event\| event\.kind\.is_some\(\)\)\.collect\(\)/,
    "event filtering should happen inside the bounded drain helper",
  );
});

test("process_events caps normalized event paths before list materialization", () => {
  const source = read("crates/worktree/src/worktree.rs");
  const normalized = functionBody(source, "normalized_events_for_worktree");
  const processEvents = functionBody(source, "process_events");

  assert.match(source, /const MAX_WORKTREE_PROCESS_EVENT_PATHS: usize = 8_192;/);
  assert.match(
    source,
    /fn root_rescan_path_event\(root_abs_path: &SanitizedPath\) -> PathEvent/,
  );
  assertBefore({
    body: normalized,
    before: "if events.len() >= MAX_WORKTREE_PROCESS_EVENT_PATHS",
    after: "mapped_events.push(PathEvent",
    message: "symlink remapping must check the process-event cap before pushing mapped paths",
  });
  assert.match(
    normalized,
    /if events\.len\(\) >= MAX_WORKTREE_PROCESS_EVENT_PATHS \{[\s\S]*log::warn!\([\s\S]*return vec!\[Self::root_rescan_path_event\(root_canonical_path\)\];/,
    "pre-remap overflow must log before root-rescan fallback",
  );
  assert.match(
    normalized,
    /if mapped_events_overflowed \{[\s\S]*log::warn!\([\s\S]*return vec!\[Self::root_rescan_path_event\(root_canonical_path\)\];/,
    "symlink fanout overflow must log before root-rescan fallback",
  );
  assertBefore({
    body: processEvents,
    before: "events = Self::cap_process_events(events, &root_canonical_path);",
    after: "let mut relative_paths = Vec::with_capacity(events.len());",
    message: "process_events must cap normalized events before relative-path materialization",
  });
});

test("metadata and changed-path fanout have named caps", () => {
  const source = read("crates/worktree/src/worktree.rs");
  const reload = functionBody(source, "reload_entries_for_paths");
  const insertChangedPath = functionBody(source, "insert_changed_path");

  assert.match(source, /const MAX_WORKTREE_METADATA_FANOUT: usize = 256;/);
  assert.match(source, /const MAX_WORKTREE_CHANGED_PATHS: usize = 8_192;/);
  assert.match(
    reload,
    /Vec::with_capacity\(abs_paths\.len\(\)\.min\(MAX_WORKTREE_METADATA_FANOUT\)\)/,
    "metadata reload should not preallocate from an unbounded request length",
  );
  assertBefore({
    body: reload,
    before: "for abs_paths in abs_paths.chunks(MAX_WORKTREE_METADATA_FANOUT)",
    after: /join_all\(/,
    message: "metadata lookups must be chunked before join_all fanout",
  });
  assert.doesNotMatch(
    reload,
    /\.collect::<Vec<_>>\(\),\s*\)\s*\.await/s,
    "reload metadata must not collect every lookup future into one join_all call",
  );
  assertBefore({
    body: insertChangedPath,
    before: "changed_paths.len() >= MAX_WORKTREE_CHANGED_PATHS",
    after: "changed_paths.insert(ix, path);",
    message: "changed-path lists must check the cap before insertion",
  });
  assert.match(
    insertChangedPath,
    /if changed_paths\.len\(\) >= MAX_WORKTREE_CHANGED_PATHS \{[\s\S]*log::warn!\([\s\S]*changed_paths\.clear\(\);/,
    "changed-path overflow must log before collapsing to a root-changed marker",
  );
  assert.doesNotMatch(
    source,
    /util::extend_sorted\(\s*&mut state\.changed_paths,[\s\S]*usize::MAX,/,
    "changed paths should not extend with an unbounded usize::MAX limit",
  );
});
