import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8").replace(/\r\n/g, "\n");
const productionSource = (source: string) =>
  source.split(/\n#\[cfg\(test\)\]/)[0] ?? source;

const sourcePath = "crates/debugger_ui/src/attach_modal.rs";
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

test("attach modal caps process match candidate and fuzzy row materialization", () => {
  assert.match(source, /const MAX_ATTACH_MODAL_MATCH_CANDIDATES: usize = 2_000;/);
  assert.match(source, /const MAX_ATTACH_MODAL_MATCHES: usize = 100;/);
  assert.match(source, /const MAX_ATTACH_MODAL_COMMAND_ARGS: usize = 32;/);
  assert.match(source, /const MAX_ATTACH_MODAL_FIELD_CHARS: usize = 512;/);

  const candidateHelper = functionBody("process_match_candidates");
  const updateMatches = functionBody("update_matches");

  assertBefore({
    body: candidateHelper,
    before: ".take(MAX_ATTACH_MODAL_MATCH_CANDIDATES)",
    after: "StringMatchCandidate::new",
    message: "process match candidates must be capped before fuzzy candidate allocation",
  });
  assertBefore({
    body: candidateHelper,
    before: ".take(MAX_ATTACH_MODAL_COMMAND_ARGS)",
    after: '.join(" ")',
    message: "process command args must be capped before command text is joined",
  });
  assert.match(candidateHelper, /bounded_process_field\(arg\)/);
  assert.match(candidateHelper, /bounded_process_field\(candidate\.name\.as_ref\(\)\)/);
  assert.doesNotMatch(
    candidateHelper,
    /candidate\.command\.join\(" "\)/,
    "match candidates must not join the full process command vector",
  );

  assert.match(updateMatches, /process_match_candidates\(&processes\)/);
  assert.match(
    updateMatches,
    /let mut matches = fuzzy::match_strings\([\s\S]*MAX_ATTACH_MODAL_MATCHES,[\s\S]*&Default::default\(\)/,
    "fuzzy process matches must use the named row cap",
  );
  assertBefore({
    body: updateMatches,
    before: ".await;",
    after: "matches.truncate(MAX_ATTACH_MODAL_MATCHES);",
    message: "empty-query fuzzy results must be truncated before replacing modal matches",
  });
  assert.doesNotMatch(
    updateMatches,
    /match_strings\([\s\S]*\n\s*100,\s*\n\s*&Default::default\(\)/,
    "attach modal should not keep an inline fuzzy row cap",
  );
});

test("attach modal clamps selected indexes in setter and async match replacement", () => {
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
  assert.doesNotMatch(
    updateMatches,
    /delegate\.selected_index\s*=\s*delegate\.selected_index\.min/,
    "async match replacement should use the shared clamp helper",
  );
});

test("attach modal bounds command text for rows and preserves guarded confirm lookup", () => {
  const commandText = functionBody("process_command_text");
  const renderMatch = functionBody("render_match");
  const confirm = functionBody("confirm");

  assert.match(
    commandText,
    /\.iter\(\)\s*\.skip\(skip_args\)\s*\.take\(MAX_ATTACH_MODAL_COMMAND_ARGS\)/s,
  );
  assert.match(commandText, /bounded_process_field\(arg\)/);
  assertBefore({
    body: renderMatch,
    before: "let command_text = process_command_text(&candidate.command, 0);",
    after: "Tooltip::text(command_text)",
    message: "tooltips must use bounded command text",
  });
  assertBefore({
    body: renderMatch,
    before: "let command_args_text = process_command_text(&candidate.command, 1);",
    after: "Label::new(format!",
    message: "row labels must use bounded command text",
  });
  assert.doesNotMatch(
    renderMatch,
    /candidate\s*\.command\s*\.clone\(\)/,
    "render_match must not clone and materialize the full process command vector",
  );
  assert.doesNotMatch(
    renderMatch,
    /candidate\.command\.join\(" "\)/,
    "render_match must not join the full process command vector",
  );
  assert.match(
    confirm,
    /self\s*\.matches\s*\.get\(self\.selected_index\(\)\)[\s\S]*self\s*\.candidates\.get\(ix\)/,
  );
  assert.doesNotMatch(
    confirm,
    /self\.(?:matches|candidates)\[[^\]]+\]/,
    "confirm must keep stale match and candidate lookup guarded",
  );
});

test("attach modal source guard stays scoped to worker-owned files", () => {
  assert.equal(sourcePath, "crates/debugger_ui/src/attach_modal.rs");
});
