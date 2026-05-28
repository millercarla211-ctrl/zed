import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const functionBody = (source: string, name: string) => {
  const start = source.search(new RegExp(`fn\\s+${name}\\s*\\(`));
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

test("remote directory listings are bounded before response materialization", () => {
  const source = read("crates/remote_server/src/headless_project.rs");
  const handleListRemoteDirectory = functionBody(source, "handle_list_remote_directory");

  assert.match(source, /const MAX_REMOTE_DIRECTORY_LISTING_ENTRIES: usize = 10_000;/);
  assertBefore({
    body: handleListRemoteDirectory,
    before: "entries.len() >= MAX_REMOTE_DIRECTORY_LISTING_ENTRIES",
    after: "entries.push(file_name.to_string_lossy().into_owned())",
    message: "remote directory entries must be capped before response-vector pushes",
  });
  assertBefore({
    body: handleListRemoteDirectory,
    before: "entries.len() >= MAX_REMOTE_DIRECTORY_LISTING_ENTRIES",
    after: "entry_info.push(proto::EntryInfo { is_dir })",
    message: "remote directory entry_info must stay aligned with capped entries",
  });
});
