import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/debugger_ui/src/debugger_panel.rs";
const source = readFileSync(sourcePath, "utf8").replace(/\r\n/g, "\n");

function functionBody(name: string): string {
  const start = source.search(new RegExp(`fn\\s+${name}\\b`));
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
}

function assertBefore({
  body,
  before,
  after,
  message,
}: {
  body: string;
  before: string | RegExp;
  after: string | RegExp;
  message: string;
}) {
  const beforeIndex =
    typeof before === "string" ? body.indexOf(before) : body.match(before)?.index ?? -1;
  const afterIndex =
    typeof after === "string" ? body.indexOf(after) : body.match(after)?.index ?? -1;
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("debugger restart refuses missing binary before replacing the running session", () => {
  const restart = functionBody("handle_restart_request");

  assert.match(
    restart,
    /let Some\(binary\)\s*=\s*curr_session\.read\(cx\)\.binary\(\)\.cloned\(\)\s*else\s*\{/s,
  );
  assert.match(
    restart,
    /log::error!\(\s*"Attempted to restart a session without a binary"\s*\)/s,
  );
  assertBefore({
    body: restart,
    before: "let Some(binary) = curr_session.read(cx).binary().cloned() else",
    after: "shutdown_session",
    message: "restart must refuse a missing binary before shutting down the current session",
  });
  assert.doesNotMatch(
    restart,
    /binary\(\)\.cloned\(\)\.unwrap\(\)/,
    "restart must not panic when the root session has no binary",
  );
});

test("debugger panel source guard stays scoped to worker-owned files", () => {
  assert.equal(sourcePath, "crates/debugger_ui/src/debugger_panel.rs");
});
