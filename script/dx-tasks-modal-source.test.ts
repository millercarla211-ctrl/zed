import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8").replace(/\r\n/g, "\n");
const productionSource = (source: string) =>
  source.split(/\n#\[cfg\(test\)\]\nmod tests\s*\{/)[0] ?? source;

const sourcePath = "crates/tasks_ui/src/modal.rs";
const source = productionSource(read(sourcePath));

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

test("tasks modal caps match candidate and fuzzy row materialization", () => {
  assert.match(source, /const MAX_TASK_MODAL_MATCH_CANDIDATES: usize = 2_000;/);
  assert.match(source, /const MAX_TASK_MODAL_MATCHES: usize = 100;/);

  const candidateHelper = functionBody("string_match_candidates");
  assertBefore({
    body: candidateHelper,
    before: ".take(MAX_TASK_MODAL_MATCH_CANDIDATES)",
    after: "StringMatchCandidate::new",
    message: "task match candidates must be capped before label materialization",
  });

  const updateMatches = functionBody("update_matches");
  assert.match(
    updateMatches,
    /match_strings\([\s\S]*MAX_TASK_MODAL_MATCHES,[\s\S]*&Default::default\(\)/,
    "fuzzy task matches must use the named row cap",
  );
  assert.doesNotMatch(
    updateMatches,
    /match_strings\([\s\S]*\n\s*1000,\s*\n\s*&Default::default\(\)/,
    "tasks modal should not keep a hardcoded fuzzy row cap",
  );
});

test("tasks modal clamps stale selected indexes after match replacement", () => {
  const setSelectedIndex = functionBody("set_selected_index");
  const clampedMatchIndex = functionBody("clamped_match_index");
  const clampSelectedIndex = functionBody("clamp_selected_index_to_matches");
  const updateMatches = functionBody("update_matches");

  assert.match(setSelectedIndex, /self\.selected_index = self\.clamped_match_index\(ix\);/);
  assert.match(clampedMatchIndex, /self\.matches\.len\(\)\.saturating_sub\(1\)/);
  assert.match(
    clampSelectedIndex,
    /self\.selected_index = self\.clamped_match_index\(self\.selected_index\);/,
  );
  assertBefore({
    body: updateMatches,
    before: "delegate.matches = matches;",
    after: "delegate.clamp_selected_index_to_matches();",
    message: "async match replacement must reclamp stale selection",
  });
});

test("tasks modal guards confirm-time candidate IDs", () => {
  const confirm = functionBody("confirm");

  assert.match(
    confirm,
    /self\.candidates\s*\.as_ref\(\)\?\s*\.get\(current_match\.candidate_id\)\s*\.cloned\(\)/s,
  );
  assert.doesNotMatch(
    confirm,
    /candidates\[[^\]]+\]/,
    "confirm must not direct-index candidates from stale matches",
  );
});

test("tasks modal caps tag labels before tooltip and row text materialization", () => {
  assert.match(source, /const MAX_TASK_MODAL_TOOLTIP_TAGS: usize = 32;/);

  const tagHelper = functionBody("task_modal_tag_labels");
  const renderMatch = functionBody("render_match");

  assertBefore({
    body: tagHelper,
    before: ".take(MAX_TASK_MODAL_TOOLTIP_TAGS)",
    after: 'format!("#{}", tag)',
    message: "task tag labels must be capped before allocation",
  });
  assertBefore({
    body: renderMatch,
    before: "let tag_labels = task_modal_tag_labels(&template.tags);",
    after: "Tooltip::simple(tooltip_label_text, cx)",
    message: "tooltip text must be built from capped tag labels",
  });
  assert.doesNotMatch(
    renderMatch,
    /template\s*\.tags\s*\.iter\(\)\s*\.map[\s\S]*collect::<Vec<_>>\(\)\s*\.join\(""\)/,
    "render_match should not join every task tag into tooltip text",
  );
});

test("tasks modal source guard stays scoped to worker-owned files", () => {
  assert.equal(sourcePath, "crates/tasks_ui/src/modal.rs");
});
