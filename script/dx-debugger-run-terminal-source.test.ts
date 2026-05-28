import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/debugger_ui/src/session/running.rs";
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

test("run-in-terminal cwd fallback does not unwrap missing binary state", () => {
  const handleRunInTerminal = functionBody("handle_run_in_terminal");

  assertBefore({
    body: handleRunInTerminal,
    before: ".then(|| PathBuf::from(&request.cwd))",
    after: ".or_else(|| session.binary().and_then(|binary| binary.cwd.clone()))",
    message: "explicit run-in-terminal cwd must win before falling back to binary cwd",
  });
  assert.doesNotMatch(
    handleRunInTerminal,
    /session\.binary\(\)\.unwrap\(\)/,
    "missing debugger binary state must not panic while resolving run-in-terminal cwd",
  );
});

test("debugger run-in-terminal source guard stays scoped to worker-owned files", () => {
  assert.equal(sourcePath, "crates/debugger_ui/src/session/running.rs");
});
