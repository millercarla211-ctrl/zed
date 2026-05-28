import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const assertBoundedRemoteConnectionDeserializer = ({
  file,
  maxConst,
  helper,
  context,
}: {
  file: string;
  maxConst: string;
  helper: string;
  context: string;
}) => {
  const source = read(file);

  assert.match(
    source,
    new RegExp(`const ${maxConst}: usize = 64 \\* 1024;`),
  );
  assert.match(
    source,
    new RegExp(
      `fn ${helper}\\(\\s*remote_connection_json: &str,?\\s*\\) -> anyhow::Result<RemoteConnectionOptions>`,
    ),
  );
  assert.match(source, new RegExp(`remote_connection_json\\.len\\(\\) > ${maxConst}`));
  assert.match(
    source,
    new RegExp(`${context}: remote_connection_json is too large`),
  );
  assert.match(
    source,
    new RegExp(
      `serde_json::from_str::<RemoteConnectionOptions>\\(remote_connection_json\\)\\s*\\.context\\("${context}"\\)`,
      "s",
    ),
  );
  assert.match(source, new RegExp(`\\.map\\(${helper}\\)\\s*\\.transpose\\(\\)\\?`, "s"));
  assert.doesNotMatch(
    source,
    /\.map\(serde_json::from_str::<RemoteConnectionOptions>\)/,
  );

  const lengthCheckIndex = source.indexOf(
    `remote_connection_json.len() > ${maxConst}`,
  );
  const parseIndex = source.indexOf(
    "serde_json::from_str::<RemoteConnectionOptions>(remote_connection_json)",
  );
  assert.ok(
    lengthCheckIndex >= 0 && lengthCheckIndex < parseIndex,
    `${file} must reject oversized remote_connection_json before parsing`,
  );
};

test("thread metadata remote connection JSON is bounded before deserialization", () => {
  assertBoundedRemoteConnectionDeserializer({
    file: "crates/agent_ui/src/thread_metadata_store.rs",
    maxConst: "MAX_THREAD_REMOTE_CONNECTION_JSON_BYTES",
    helper: "deserialize_thread_remote_connection",
    context: "deserialize thread metadata remote connection",
  });
});

test("terminal thread metadata remote connection JSON is bounded before deserialization", () => {
  assertBoundedRemoteConnectionDeserializer({
    file: "crates/agent_ui/src/terminal_thread_metadata_store.rs",
    maxConst: "MAX_TERMINAL_THREAD_REMOTE_CONNECTION_JSON_BYTES",
    helper: "deserialize_terminal_thread_remote_connection",
    context: "deserialize terminal thread remote connection",
  });
});
