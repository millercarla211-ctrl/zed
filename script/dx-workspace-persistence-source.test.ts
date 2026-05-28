import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const persistence = read("crates/workspace/src/persistence.rs");
const dock = read("crates/workspace/src/dock.rs");
const workspace = read("crates/workspace/src/workspace.rs");

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

test("workspace persistence JSON byte caps are explicit", () => {
  const expectedConstants = [
    /const MAX_DEFAULT_WINDOW_BOUNDS_JSON_BYTES: usize = 16 \* 1024;/,
    /const MAX_MULTI_WORKSPACE_STATE_JSON_BYTES: usize = 64 \* 1024;/,
    /const MAX_DEFAULT_DOCK_STATE_JSON_BYTES: usize = 64 \* 1024;/,
    /const MAX_USER_TOOLCHAIN_JSON_BYTES: usize = 64 \* 1024;/,
    /const MAX_REMOTE_ENV_JSON_BYTES: usize = 64 \* 1024;/,
    /const MAX_PANE_GROUP_FLEXES_JSON_BYTES: usize = 16 \* 1024;/,
  ];

  for (const pattern of expectedConstants) {
    assert.match(persistence, pattern);
  }

  const helper = functionBody(persistence, "ensure_persisted_json_within_limit");
  assert.match(helper, /json\.len\(\) > max_bytes/);
  assert.match(helper, /persisted JSON is too large/);
  assert.match(helper, /bail!\(/);
});

test("workspace persistence indexing avoids stale direct index panics", () => {
  const readSerialized = functionBody(
    persistence,
    "read_serialized_multi_workspaces",
  );
  assert.doesNotMatch(
    readSerialized,
    /window_groups\s*\[\s*group_index\s*\]/,
  );
  assert.match(readSerialized, /window_groups\s*\.get_mut\(group_index\)/);

  const dedupe = functionBody(persistence, "dedupe_recent_workspaces");
  assert.doesNotMatch(dedupe, /result\s*\[\s*existing_index\s*\]/);
  assert.match(dedupe, /result\s*\.get\(existing_index\)/);
  assert.match(dedupe, /result\s*\.get_mut\(existing_index\)/);
});

test("workspace KVP JSON is bounded before deserialization", () => {
  const defaultWindowBounds = functionBody(persistence, "read_default_window_bounds");
  assertBefore({
    body: defaultWindowBounds,
    before:
      /ensure_persisted_json_within_limit\(\s*&json_str,\s*MAX_DEFAULT_WINDOW_BOUNDS_JSON_BYTES/,
    after: "serde_json::from_str::<(Uuid, WindowBoundsJson)>(&json_str)",
    message: "default window bounds must reject oversized JSON before parsing",
  });
  assert.match(defaultWindowBounds, /deserialize persisted default window bounds/);
  assert.doesNotMatch(defaultWindowBounds, /\.ok\(\)\?/);

  const multiWorkspaceState = functionBody(persistence, "read_multi_workspace_state");
  assertBefore({
    body: multiWorkspaceState,
    before:
      /ensure_persisted_json_within_limit\(\s*&json,\s*MAX_MULTI_WORKSPACE_STATE_JSON_BYTES/,
    after: "serde_json::from_str(&json)",
    message: "multi-workspace state must reject oversized JSON before parsing",
  });
  assert.match(multiWorkspaceState, /deserialize persisted multi-workspace state/);

  const defaultDockState = functionBody(persistence, "read_default_dock_state");
  assertBefore({
    body: defaultDockState,
    before:
      /ensure_persisted_json_within_limit\(\s*&json_str,\s*MAX_DEFAULT_DOCK_STATE_JSON_BYTES/,
    after: "serde_json::from_str::<DockStructure>(&json_str)",
    message: "default dock state must reject oversized JSON before parsing",
  });
  assert.match(defaultDockState, /deserialize persisted default dock state/);
  assert.doesNotMatch(defaultDockState, /\.ok\(\)/);
});

test("workspace DB JSON columns are bounded before deserialization", () => {
  const userToolchains = functionBody(persistence, "user_toolchains");
  assertBefore({
    body: userToolchains,
    before:
      /ensure_persisted_json_within_limit\(\s*&raw_json,\s*MAX_USER_TOOLCHAIN_JSON_BYTES/,
    after: "serde_json::from_str::<serde_json::Value>(&raw_json)",
    message: "user toolchain raw_json must be bounded before parsing",
  });
  assert.match(userToolchains, /deserialize persisted user toolchain JSON/);

  const remoteConnection = functionBody(persistence, "remote_connection_from_row");
  assertBefore({
    body: remoteConnection,
    before:
      /ensure_persisted_json_within_limit\(\s*&remote_env_json,\s*MAX_REMOTE_ENV_JSON_BYTES/,
    after: "serde_json::from_str(&remote_env_json)",
    message: "Docker remote_env JSON must be bounded before parsing",
  });
  assert.match(remoteConnection, /deserialize persisted Docker remote_env/);

  const paneGroup = functionBody(persistence, "get_pane_group");
  assertBefore({
    body: paneGroup,
    before:
      /ensure_persisted_json_within_limit\(\s*&flexes,\s*MAX_PANE_GROUP_FLEXES_JSON_BYTES/,
    after: "serde_json::from_str::<Vec<f32>>(&flexes)",
    message: "pane-group flexes JSON must be bounded before parsing",
  });
  assert.match(paneGroup, /deserialize persisted pane-group flexes/);
});

test("dock panel-size KVP JSON is bounded before deserialization", () => {
  assert.match(
    dock,
    /pub\(crate\) const MAX_PANEL_SIZE_STATE_JSON_BYTES: usize = 4 \* 1024;/,
  );

  const helper = functionBody(dock, "ensure_panel_size_state_json_within_limit");
  assert.match(helper, /json\.len\(\) > MAX_PANEL_SIZE_STATE_JSON_BYTES/);
  assert.match(helper, /panel size state KVP payload is too large/);
  assert.match(helper, /\.log_err\(\)/);

  const loadPersisted = functionBody(dock, "load_persisted_size_state");
  assertBefore({
    body: loadPersisted,
    before: "ensure_panel_size_state_json_within_limit(&json, panel_key)?",
    after: "serde_json::from_str::<PanelSizeState>(&json)",
    message: "panel-size KVP JSON must be bounded before parsing",
  });

  const legacyPanelSize = functionBody(workspace, "load_legacy_panel_size");
  assertBefore({
    body: legacyPanelSize,
    before: "dock::ensure_panel_size_state_json_within_limit(&json, &legacy_key)?",
    after: "serde_json::from_str::<LegacyPanelState>(&json)",
    message: "legacy panel-size JSON must be bounded before parsing",
  });
});
