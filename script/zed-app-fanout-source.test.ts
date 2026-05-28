import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const zedSource = read("crates/zed/src/zed.rs");
const mainSource = read("crates/zed/src/main.rs");
const multiWorkspaceSource = read("crates/workspace/src/multi_workspace.rs");

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

const indexOfPattern = (body: string, pattern: string | RegExp) => {
  if (typeof pattern === "string") {
    return body.indexOf(pattern);
  }
  return body.match(pattern)?.index ?? -1;
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

test("zed app-level fanout caps are named constants", () => {
  assert.match(zedSource, /const MAX_QUIT_WORKSPACE_WINDOWS: usize = 512;/);
  assert.match(zedSource, /const MAX_QUIT_WORKSPACES_PER_WINDOW: usize = 512;/);
  assert.match(zedSource, /const MAX_QUIT_SERIALIZATION_FLUSH_TASKS: usize = 8_192;/);
  assert.match(mainSource, /const MAX_CHANNEL_NOTE_OPEN_PROMISES: usize = 256;/);
  assert.match(mainSource, /const MAX_DUMP_GPUI_ACTION_DEFINITIONS: usize = 16_384;/);
});

test("quit bounds window and workspace materialization before push loops", () => {
  const collectWindows = functionBody(zedSource, "collect_quit_workspace_windows");
  assertBefore({
    body: collectWindows,
    before: "workspace_windows.len() >= MAX_QUIT_WORKSPACE_WINDOWS",
    after: "workspace_windows.push(window);",
    message: "quit window fanout must check the cap before collecting windows",
  });
  assert.match(
    collectWindows,
    /log::warn!\([\s\S]*MAX_QUIT_WORKSPACE_WINDOWS/,
    "window truncation must be visible",
  );
  assert.match(
    collectWindows,
    /return None;/,
    "window truncation must cancel quit instead of skipping unchecked windows",
  );

  const collectWorkspaces = functionBody(zedSource, "collect_quit_workspaces_for_window");
  assertBefore({
    body: collectWorkspaces,
    before: "workspaces.len() >= MAX_QUIT_WORKSPACES_PER_WINDOW",
    after: "workspaces.push(workspace.clone());",
    message: "quit workspace fanout must check the cap before collecting workspaces",
  });
  assert.match(
    collectWorkspaces,
    /log::warn!\([\s\S]*MAX_QUIT_WORKSPACES_PER_WINDOW/,
    "workspace truncation must be visible",
  );
  assert.match(
    collectWorkspaces,
    /return None;/,
    "workspace truncation must cancel quit instead of skipping unchecked workspaces",
  );

  const quit = functionBody(zedSource, "quit");
  assert.match(
    quit,
    /let Some\(mut workspace_windows\)/,
    "quit must fail closed if window collection exceeds the cap",
  );
  assertBefore({
    body: quit,
    before: "collect_quit_workspace_windows(cx)",
    after: "workspace_windows.sort_by_key",
    message: "quit must cap workspace windows before sorting/display ordering",
  });
  assert.doesNotMatch(
    quit,
    /collect::<Vec<_>>\(\)/,
    "quit should not collect unbounded window or workspace vectors directly",
  );
});

test("quit bounds serialization flush task fanout before join_all", () => {
  const pushFlushTask = functionBody(zedSource, "push_quit_flush_task");
  assertBefore({
    body: pushFlushTask,
    before: "flush_tasks.len() >= MAX_QUIT_SERIALIZATION_FLUSH_TASKS",
    after: "flush_tasks.push(make_task());",
    message: "flush task fanout must check the cap before creating and pushing tasks",
  });
  assert.match(
    pushFlushTask,
    /log::warn!\([\s\S]*MAX_QUIT_SERIALIZATION_FLUSH_TASKS/,
    "flush task truncation must be visible",
  );
  assert.match(
    pushFlushTask,
    /return false;/,
    "flush task overflow must cancel quit instead of skipping serialization tasks",
  );

  const quit = functionBody(zedSource, "quit");
  assertBefore({
    body: quit,
    before: /push_quit_flush_task\(\s*&mut flush_tasks/,
    after: "futures::future::join_all(flush_tasks).await",
    message: "flush tasks must be capped before join_all",
  });
  assertBefore({
    body: quit,
    before: "collect_quit_workspaces_for_window(multi_workspace)",
    after: "workspace.flush_serialization(window, cx)",
    message: "workspace flushes must be capped before serialization tasks are created",
  });
  assertBefore({
    body: quit,
    before: "pending_removal_task_count()",
    after: "multi_workspace.take_pending_removal_tasks()",
    message: "quit must check flush capacity before draining pending removal tasks",
  });
  assert.match(
    quit,
    /quit_flush_ready != Some\(true\)[\s\S]*futures::future::join_all\(flush_tasks\)\.await;[\s\S]*return Ok\(\(\)\);/,
    "quit must await already-collected flush tasks before cancelling",
  );
  assert.doesNotMatch(
    quit,
    /flush_tasks\.append|flush_tasks\.push/,
    "quit should route flush task fanout through the bounded helper",
  );

  const pendingRemovalTaskCount = functionBody(
    multiWorkspaceSource,
    "pending_removal_task_count",
  );
  assert.match(pendingRemovalTaskCount, /filter\(\|task\| !task\.is_ready\(\)\)/);
  assert.match(pendingRemovalTaskCount, /\.count\(\)/);
});

test("channel-note opens are capped before promise fanout and join_all", () => {
  const handleOpenRequest = functionBody(mainSource, "handle_open_request");
  const channelStart = handleOpenRequest.indexOf(
    "if !request.open_channel_notes.is_empty() || request.join_channel.is_some()",
  );
  assert.ok(channelStart >= 0, "expected channel-note branch");
  const channelEnd = handleOpenRequest.indexOf("} else if let Some(task) = task", channelStart);
  assert.ok(channelEnd > channelStart, "expected channel-note branch end");
  const channelBranch = handleOpenRequest.slice(channelStart, channelEnd);

  assert.match(
    channelBranch,
    /log::warn!\([\s\S]*MAX_CHANNEL_NOTE_OPEN_PROMISES/,
    "channel-note truncation must be visible",
  );
  assertBefore({
    body: channelBranch,
    before: "open_channel_notes.len() > MAX_CHANNEL_NOTE_OPEN_PROMISES",
    after: "let mut promises = Vec::with_capacity(",
    message: "channel notes must be checked before promise vector allocation",
  });
  assertBefore({
    body: channelBranch,
    before: ".take(MAX_CHANNEL_NOTE_OPEN_PROMISES)",
    after: "promises.push(",
    message: "channel-note promise fanout must be capped before push",
  });
  assertBefore({
    body: channelBranch,
    before: ".take(MAX_CHANNEL_NOTE_OPEN_PROMISES)",
    after: "future::join_all(promises).await",
    message: "channel-note promises must be capped before join_all",
  });
});

test("action definition dump is bounded before schema materialization and sort", () => {
  const dumpActions = functionBody(mainSource, "dump_all_gpui_actions");
  assertBefore({
    body: dumpActions,
    before: "actions.len() >= MAX_DUMP_GPUI_ACTION_DEFINITIONS",
    after: "let schema = (action.json_schema)(&mut generator)",
    message: "action dump must cap entries before schema generation",
  });
  assertBefore({
    body: dumpActions,
    before: "actions.len() >= MAX_DUMP_GPUI_ACTION_DEFINITIONS",
    after: "actions.push(ActionDef",
    message: "action dump must check the cap before collecting action definitions",
  });
  assertBefore({
    body: dumpActions,
    before: "actions.len() >= MAX_DUMP_GPUI_ACTION_DEFINITIONS",
    after: "actions.sort_by_key",
    message: "action definitions must be capped before sorting/display",
  });
  assert.match(
    dumpActions,
    /eprintln!\([\s\S]*MAX_DUMP_GPUI_ACTION_DEFINITIONS/,
    "action dump truncation must be visible before JSON is emitted",
  );
  assert.doesNotMatch(
    dumpActions,
    /collect::<Vec<ActionDef>>\(\)/,
    "action definitions should not be collected through an unbounded iterator",
  );
});
