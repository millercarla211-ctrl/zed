import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const functionBody = (source: string, name: string): string => {
  const start = source.search(new RegExp(`fn\\s+${name}(?:<[^>]+>)?\\s*\\(`));
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

test("debug console DAP completion rows are capped before UI materialization", () => {
  const source = read("crates/debugger_ui/src/session/running/console.rs");
  const clientCompletions = functionBody(source, "client_completions");

  assert.match(source, /const MAX_CONSOLE_DAP_COMPLETIONS: usize = 2_048;/);
  assertBefore({
    body: clientCompletions,
    before: "completions.truncate(MAX_CONSOLE_DAP_COMPLETIONS)",
    after: ".map(|completion|",
    message: "debug adapter completions must be capped before UI completion rows are materialized",
  });
  assertBefore({
    body: clientCompletions,
    before: "let is_incomplete = completions.len() > MAX_CONSOLE_DAP_COMPLETIONS;",
    after: "is_incomplete,",
    message: "truncated debug adapter completion responses must remain marked incomplete",
  });
  assert.match(clientCompletions, /log::warn!\(/);
});
