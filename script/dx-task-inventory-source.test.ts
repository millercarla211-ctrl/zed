import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const source = readFileSync("crates/project/src/task_inventory.rs", "utf8");

const functionBody = (name: string) => {
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

test("task inventory caps settings-backed template list materialization", () => {
  assert.match(source, /const MAX_TASK_TEMPLATES_PER_SETTINGS_FILE: usize = 2_048;/);
  assert.match(source, /const MAX_TASK_TEMPLATES_FROM_SETTINGS_LIST: usize = 4_096;/);
  assert.match(source, /fn collect_bounded_task_templates_from_settings\(/);

  const collector = functionBody("collect_bounded_task_templates_from_settings");
  assertBefore({
    body: collector,
    before: ".take(MAX_TASK_TEMPLATES_FROM_SETTINGS_LIST)",
    after: ".collect::<Vec<_>>()",
    message: "settings-backed task templates must be capped before list materialization",
  });

  for (const name of [
    "list_tasks",
    "used_and_current_resolved_tasks",
    "global_templates_with_tag",
    "templates_with_hooks",
  ]) {
    assert.match(
      functionBody(name),
      /collect_bounded_task_templates_from_settings/,
      `${name} should collect settings-backed templates through the bounded helper`,
    );
  }
});

test("task file updates cap template materialization and validation errors", () => {
  assert.match(source, /const MAX_TASK_TEMPLATE_VALIDATION_ERRORS: usize = 128;/);
  assert.match(
    source,
    /const MAX_TASK_TEMPLATE_UNKNOWN_VARIABLES_PER_ERROR: usize = 64;/,
  );

  const updater = functionBody("update_file_based_tasks");

  assertBefore({
    body: updater,
    before: "raw_tasks.len() > MAX_TASK_TEMPLATES_PER_SETTINGS_FILE",
    after: "let mut validation_errors = Vec::new();",
    message: "oversized task files should fail before validation-error accumulation",
  });
  assertBefore({
    body: updater,
    before: ".take(MAX_TASK_TEMPLATES_PER_SETTINGS_FILE)",
    after: ".filter_map(|raw_template|",
    message: "task templates should be capped before parsing into new templates",
  });
  assertBefore({
    body: updater,
    before: ".take(MAX_TASK_TEMPLATE_UNKNOWN_VARIABLES_PER_ERROR)",
    after: ".collect::<Vec<_>>()",
    message: "unknown variable display lists should be capped before collection",
  });
  assertBefore({
    body: updater,
    before: "validation_errors.len() < MAX_TASK_TEMPLATE_VALIDATION_ERRORS",
    after: "validation_errors.push(format!(",
    message: "validation errors should be capped before pushing formatted errors",
  });
  assertBefore({
    body: updater,
    before: ".take(MAX_TASK_TEMPLATES_PER_SETTINGS_FILE)",
    after: ".collect::<Vec<_>>()",
    message: "new task templates should be capped before Vec materialization",
  });
});

test("debug scenario updates cap new-template materialization", () => {
  assert.match(source, /const MAX_DEBUG_SCENARIOS_PER_SETTINGS_FILE: usize = 2_048;/);

  const updater = functionBody("update_file_based_scenarios");
  assertBefore({
    body: updater,
    before: "raw_tasks.len() > MAX_DEBUG_SCENARIOS_PER_SETTINGS_FILE",
    after: "let new_templates = raw_tasks",
    message: "oversized debug scenario files should fail before new-template materialization",
  });
  assertBefore({
    body: updater,
    before: ".take(MAX_DEBUG_SCENARIOS_PER_SETTINGS_FILE)",
    after: ".collect::<Vec<_>>()",
    message: "debug scenario templates should be capped before Vec materialization",
  });
});
