import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const statusToastSourcePath = "crates/notifications/src/status_toast.rs";
const statusToastSource = read(statusToastSourcePath);

function functionBody(source: string, name: string): string {
  const fnIndex = source.search(new RegExp(`fn\\s+${name}\\s*\\(`));
  assert.ok(fnIndex >= 0, `expected ${name}`);

  const bodyStart = source.indexOf("{", fnIndex);
  assert.ok(bodyStart > fnIndex, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(fnIndex, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
}

function sliceBetween(source: string, startNeedle: string, endNeedle: string): string {
  const start = source.indexOf(startNeedle);
  assert.notEqual(start, -1, `expected ${startNeedle}`);
  const end = source.indexOf(endNeedle, start + startNeedle.length);
  assert.ok(end > start, `expected ${endNeedle} after ${startNeedle}`);
  return source.slice(start, end);
}

function assertBefore(
  haystack: string,
  before: string | RegExp,
  after: string | RegExp,
  message: string,
) {
  const beforeIndex =
    typeof before === "string" ? haystack.indexOf(before) : haystack.match(before)?.index ?? -1;
  const afterIndex =
    typeof after === "string" ? haystack.indexOf(after) : haystack.match(after)?.index ?? -1;
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("status toast bounds message and action labels before UI materialization", () => {
  assert.equal(statusToastSourcePath, "crates/notifications/src/status_toast.rs");
  assert.match(statusToastSource, /use util::truncate_and_trailoff;/);
  assert.match(statusToastSource, /const MAX_STATUS_TOAST_MESSAGE_LABEL_LEN: usize = 4_096;/);
  assert.match(statusToastSource, /const MAX_STATUS_TOAST_ACTION_LABEL_LEN: usize = 512;/);

  assert.match(
    functionBody(statusToastSource, "bounded_status_toast_message"),
    /truncate_and_trailoff\(&text, MAX_STATUS_TOAST_MESSAGE_LABEL_LEN\)\.into\(\)/,
  );
  assert.match(
    functionBody(statusToastSource, "bounded_status_toast_action_label"),
    /truncate_and_trailoff\(&label, MAX_STATUS_TOAST_ACTION_LABEL_LEN\)\.into\(\)/,
  );

  const newBody = functionBody(statusToastSource, "new");
  assert.match(
    newBody,
    /text:\s*bounded_status_toast_message\(text\),/,
    "status toast message text must be bounded before storage and Label rendering",
  );
  assert.doesNotMatch(newBody, /text:\s*text\.into\(\),/);

  const actionBody = functionBody(statusToastSource, "action");
  assertBefore(
    actionBody,
    "let label = bounded_status_toast_action_label(label);",
    "ToastAction::new(",
    "status toast action labels must be bounded before action/button/id materialization",
  );
  assertBefore(
    actionBody,
    "let label = bounded_status_toast_action_label(label);",
    /ToastAction::new\(\s*label,/,
    "bounded status toast action label must be passed into ToastAction",
  );
  assert.doesNotMatch(actionBody, /label\.into\(\)/);

  const renderBody = sliceBetween(
    statusToastSource,
    "impl Render for StatusToast {",
    "impl ToastView for StatusToast {",
  );
  assert.match(renderBody, /Label::new\(self\.text\.clone\(\)\)/);
  assert.match(renderBody, /Button::new\(action\.id\.clone\(\), action\.label\.clone\(\)\)/);
  assert.match(renderBody, /Tooltip::for_action_title\(\s*action\.label\.clone\(\),/);
});
