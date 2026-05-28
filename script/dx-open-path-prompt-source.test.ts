import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/open_path_prompt/src/open_path_prompt.rs";
const source = readFileSync(sourcePath, "utf8").replace(/\r\n/g, "\n");

const functionBody = (name: string) => {
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
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  const afterBody = body.slice(beforeIndex + 1);
  const afterIndex =
    typeof after === "string"
      ? afterBody.indexOf(after)
      : afterBody.match(after)?.index ?? -1;
  assert.ok(afterIndex >= 0, `${message}: missing ${after} after ${before}`);
};

test("open path prompt declares named caps for source-bounded materialization", () => {
  assert.match(source, /const MAX_OPEN_PATH_PROMPT_DIRECTORY_ENTRIES: usize = 4_096;/);
  assert.match(source, /const MAX_OPEN_PATH_PROMPT_MATCH_ROWS: usize = 100;/);
  assert.match(source, /const MAX_OPEN_PATH_PROMPT_USER_INPUT_CHARS: usize = 4_096;/);
  assert.match(source, /fn capped_open_path_prompt_display_suffix\(suffix: &str\) -> String/);
  assert.match(
    source,
    /fn open_path_prompt_user_input\(\s*id: usize,\s*suffix: &str,\s*exists: bool,\s*is_dir: bool,\s*\) -> Option<UserInput>/,
  );
});

test("open path prompt bounds directory and fuzzy candidates before materialization", () => {
  const updateMatches = functionBody("update_matches");
  const pathCandidates = functionBody("path_candidates");

  assertBefore({
    body: pathCandidates,
    before: ".take(MAX_OPEN_PATH_PROMPT_DIRECTORY_ENTRIES)",
    after: "StringMatchCandidate::new(ix, &item.path.to_string_lossy())",
    message: "directory entries must be capped before CandidateInfo materialization",
  });
  assertBefore({
    body: updateMatches,
    before: ".take(MAX_OPEN_PATH_PROMPT_MATCH_ROWS)",
    after: "StringMatch {\n                            candidate_id",
    message: "empty suffix rows must be capped before StringMatch materialization",
  });
  assertBefore({
    body: updateMatches,
    before: ".take(MAX_OPEN_PATH_PROMPT_DIRECTORY_ENTRIES)",
    after: ".collect::<Vec<_>>()",
    message: "fuzzy candidates must be capped before candidate vector materialization",
  });
  assert.match(
    updateMatches,
    /fuzzy::match_strings\(\s*candidates\.as_slice\(\),\s*&display_suffix,\s*false,\s*true,\s*MAX_OPEN_PATH_PROMPT_MATCH_ROWS,/,
    "fuzzy search must use the named match-row cap",
  );
});

test("open path prompt omits oversized suffixes before user-input candidates", () => {
  const updateMatches = functionBody("update_matches");
  const userInput = functionBody("open_path_prompt_user_input");

  assertBefore({
    body: userInput,
    before: /suffix\s*\.chars\(\)\s*\.nth\(MAX_OPEN_PATH_PROMPT_USER_INPUT_CHARS\)\s*\.is_some\(\)/,
    after: "StringMatchCandidate::new(id, suffix)",
    message: "oversized suffixes must be rejected before path-bearing user input is built",
  });
  assertBefore({
    body: updateMatches,
    before: "let display_suffix = capped_open_path_prompt_display_suffix(&suffix);",
    after: "fuzzy::match_strings",
    message: "bounded suffix text may only feed fuzzy/display matching",
  });
  assert.match(
    updateMatches,
    /open_path_prompt_user_input\(\s*new_id,\s*&suffix,\s*exists,\s*is_dir,\s*\)/,
    "create-state user input must use the original suffix after cap validation",
  );
  assert.match(
    updateMatches,
    /open_path_prompt_user_input\(0, &suffix, false, false\)/,
    "fallback create-state user input must use the original suffix after cap validation",
  );
  assert.doesNotMatch(
    updateMatches,
    /StringMatchCandidate::new\([^)]*&(?:display_)?suffix\)/,
    "update_matches must not directly turn suffix text into path-bearing user input",
  );
});

test("open path prompt clamps stale selection after direct and async match replacement", () => {
  const setSelectedIndex = functionBody("set_selected_index");
  const clampSelectedIndex = functionBody("clamp_selected_index");
  const updateMatches = functionBody("update_matches");

  assert.match(clampSelectedIndex, /self\.match_count\(\)\.saturating_sub\(1\)/);
  assert.match(clampSelectedIndex, /self\.selected_index = self\.selected_index\.min\(max_index\);/);
  assertBefore({
    body: setSelectedIndex,
    before: "self.selected_index = ix;",
    after: "self.clamp_selected_index();",
    message: "direct picker selection writes must clamp against the current match count",
  });
  assertBefore({
    body: updateMatches,
    before: "this.delegate.string_matches = new_entries",
    after: "this.delegate.clamp_selected_index();",
    message: "empty-suffix match replacement must clamp stale selection before notify",
  });
  assertBefore({
    body: updateMatches,
    before: "this.delegate.string_matches = matches.clone();",
    after: "this.delegate.clamp_selected_index();",
    message: "fuzzy match replacement must clamp stale selection before notify",
  });
});

test("open path prompt source guard stays in worker-owned files", () => {
  assert.equal(sourcePath, "crates/open_path_prompt/src/open_path_prompt.rs");
});
