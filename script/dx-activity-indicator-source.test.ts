import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const activityIndicatorSourcePath =
  "crates/activity_indicator/src/activity_indicator.rs";
const activityIndicatorSource = read(activityIndicatorSourcePath);

function functionBody(source: string, name: string): string {
  const fnIndex = source.indexOf(`fn ${name}(`);
  assert.notEqual(fnIndex, -1, `expected ${name}`);

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
    typeof before === "string"
      ? haystack.indexOf(before)
      : haystack.match(before)?.index ?? -1;
  const afterIndex =
    typeof after === "string"
      ? haystack.indexOf(after)
      : haystack.match(after)?.index ?? -1;

  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("activity indicator bounds status and menu labels before UI materialization", () => {
  assert.equal(
    activityIndicatorSourcePath,
    "crates/activity_indicator/src/activity_indicator.rs",
  );
  assert.match(
    activityIndicatorSource,
    /const MAX_ACTIVITY_INDICATOR_MESSAGE_LABEL_LEN: usize = 4_096;/,
  );
  assert.match(
    activityIndicatorSource,
    /const MAX_ACTIVITY_INDICATOR_MENU_LABEL_LEN: usize = 512;/,
  );

  assert.match(
    functionBody(activityIndicatorSource, "bounded_activity_indicator_message"),
    /truncate_and_trailoff\(message, MAX_ACTIVITY_INDICATOR_MESSAGE_LABEL_LEN\)/,
  );
  assert.match(
    functionBody(activityIndicatorSource, "bounded_activity_indicator_menu_label"),
    /truncate_and_trailoff\(label, MAX_ACTIVITY_INDICATOR_MENU_LABEL_LEN\)/,
  );

  const renderBody = sliceBetween(
    activityIndicatorSource,
    "impl Render for ActivityIndicator {",
    "impl StatusItemView for ActivityIndicator {",
  );
  assertBefore(
    renderBody,
    "let bounded_message = bounded_activity_indicator_message(&content.message);",
    "Button::new(\"activity-indicator-trigger\"",
    "activity indicator status text must be bounded before button materialization",
  );
  assertBefore(
    renderBody,
    /let bounded_tooltip_message = content\s*\.tooltip_message\s*\.as_deref\(\)\s*\.map\(bounded_activity_indicator_message\);/,
    "Tooltip::text(tooltip_message)",
    "activity indicator tooltip text must be bounded before tooltip materialization",
  );
  assert.doesNotMatch(renderBody, /Tooltip::text\(content\.message\)/);

  const menuBody = sliceBetween(
    renderBody,
    "this.menu(move |window, cx|",
    "has_cancellable_work.then_some(menu)",
  );
  assertBefore(
    menuBody,
    /bounded_activity_indicator_menu_label\(\s*&format!\("Cancel \{title\}"\),?\s*\)/,
    "menu.custom_entry",
    "cancellable work labels must be bounded before custom menu entries",
  );
  assertBefore(
    menuBody,
    "let title = bounded_activity_indicator_menu_label(&title);",
    "menu = menu.label(title);",
    "pending work labels must be bounded before menu labels",
  );
});
