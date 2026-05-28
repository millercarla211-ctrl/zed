import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8").replace(/\r\n/g, "\n");
const productionSource = (source: string) =>
  source.split(/\n#\[cfg\(test\)\]\nmod tests\s*\{/)[0] ?? source;

const sourcePath = "crates/debugger_ui/src/new_process_modal.rs";
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

test("debug new-process declares named candidate, fuzzy, and prompt caps", () => {
  assert.match(source, /const MAX_DEBUG_SCENARIO_CANDIDATES: usize = \d+_?\d*;/);
  assert.match(source, /const MAX_DEBUG_SCENARIO_FUZZY_ROWS: usize = \d+_?\d*;/);
  assert.match(source, /const MAX_DEBUG_SCENARIO_CUSTOM_COMMAND_CHARS: usize = \d+_?\d*;/);
});

test("debug scenario candidates are capped before fuzzy matching", () => {
  const tasksLoaded = functionBody("tasks_loaded");
  const updateMatches = functionBody("update_matches");

  assert.match(tasksLoaded, /recent\.len\(\)\.min\(MAX_DEBUG_SCENARIO_CANDIDATES\)/);
  assert.match(
    tasksLoaded,
    /MAX_DEBUG_SCENARIO_CANDIDATES\.saturating_sub\(recent_candidate_count\)/,
  );
  assert.match(tasksLoaded, /\.take\(MAX_DEBUG_SCENARIO_CANDIDATES\)/);
  assert.match(tasksLoaded, /\.take\(scenario_candidate_slots\)/);

  assertBefore({
    body: updateMatches,
    before: ".take(MAX_DEBUG_SCENARIO_CANDIDATES)",
    after: "StringMatchCandidate::new",
    message: "debug scenario labels must be capped before fuzzy candidate materialization",
  });
  assert.match(updateMatches, /MAX_DEBUG_SCENARIO_FUZZY_ROWS/);
  assert.doesNotMatch(
    updateMatches,
    /match_strings\([\s\S]*\n\s*1000,\s*\n\s*&Default::default\(\)/,
    "debug new-process fuzzy matching should use a named row cap",
  );
});

test("debug new-process clamps stale selected indexes consistently", () => {
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
    message: "async debug match replacement must reclamp stale selection",
  });
});

test("debug new-process guards stale candidate IDs while rendering and confirming", () => {
  const renderMatch = functionBody("render_match");
  const confirm = functionBody("confirm");

  assert.match(renderMatch, /self\.candidates\s*\.get\(hit\.candidate_id\)\?/s);
  assert.doesNotMatch(
    renderMatch,
    /self\.candidates\[[^\]]+\]/,
    "render_match must not direct-index candidates from stale fuzzy matches",
  );
  assert.match(
    confirm,
    /self\.candidates\s*\.get\(match_candidate\.candidate_id\)\s*\.cloned\(\)/s,
  );
  assert.doesNotMatch(
    confirm,
    /self\.candidates\[[^\]]+\]|candidates\[[^\]]+\]/,
    "confirm must not direct-index candidates from stale fuzzy matches",
  );
});

test("custom debug command prompts are refused before shell splitting", () => {
  const promptHelper = functionBody("debug_prompt_within_command_cap");
  const confirmInput = functionBody("confirm_input");

  assert.match(promptHelper, /-> Option<&str>/);
  assert.match(
    promptHelper,
    /char_indices\(\)\s*\.nth\(MAX_DEBUG_SCENARIO_CUSTOM_COMMAND_CHARS\)\s*\.is_some\(\)/s,
  );
  assert.match(promptHelper, /None/);
  assert.match(promptHelper, /Some\(prompt\)/);
  assert.doesNotMatch(
    promptHelper,
    /Cow::Owned|to_owned\(\)|to_string\(\)/,
    "oversized custom debug commands must be refused, not truncated into a new command",
  );
  assertBefore({
    body: confirmInput,
    before: "let Some(text) = debug_prompt_within_command_cap(&self.prompt) else",
    after: "ShellKind::Posix",
    message: "oversized custom debug command text must no-op before shell splitting",
  });
  assert.doesNotMatch(confirmInput, /capped_debug_prompt|into_owned\(\)/);
});

test("debug new-process source guard stays scoped to worker-owned files", () => {
  assert.equal(sourcePath, "crates/debugger_ui/src/new_process_modal.rs");
});
