import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath =
  "crates/settings_profile_selector/src/settings_profile_selector.rs";

const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const source = productionSource(readFileSync(sourcePath, "utf8"));

function functionBody(sourceText: string, name: string): string {
  const fnIndex = sourceText.indexOf(`fn ${name}`);
  assert.notEqual(fnIndex, -1, `expected ${name}`);

  const bodyStart = sourceText.indexOf("{", fnIndex);
  assert.ok(bodyStart > fnIndex, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < sourceText.length; index += 1) {
    const char = sourceText[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return sourceText.slice(fnIndex, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
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

test("settings profile selector declares focused materialization caps", () => {
  assert.match(
    source,
    /const MAX_SETTINGS_PROFILE_SELECTOR_CONFIGURED_PROFILES: usize = \d+;/,
  );
  assert.match(
    source,
    /const MAX_SETTINGS_PROFILE_SELECTOR_MATCHES: usize =\s*MAX_SETTINGS_PROFILE_SELECTOR_CONFIGURED_PROFILES \+ 1;/,
  );
});

test("configured profiles are capped while preserving disabled and current rows", () => {
  const delegateImplStart = source.indexOf("impl SettingsProfileSelectorDelegate {");
  assert.notEqual(delegateImplStart, -1, "expected SettingsProfileSelectorDelegate impl");
  const constructor = functionBody(source.slice(delegateImplStart), "new");
  const profileHelper = functionBody(source, "capped_settings_profile_names");

  assert.match(
    constructor,
    /capped_settings_profile_names\(settings_store,\s*profile_name\.as_deref\(\)\)/,
  );
  assertBefore(
    profileHelper,
    ".take(MAX_SETTINGS_PROFILE_SELECTOR_CONFIGURED_PROFILES)",
    ".map(|profile_name| Some(profile_name.to_string()))",
    "configured profiles must be capped before row strings are materialized",
  );
  assertBefore(
    profileHelper,
    ".take(MAX_SETTINGS_PROFILE_SELECTOR_CONFIGURED_PROFILES)",
    ".collect::<Vec<_>>()",
    "configured profiles must be capped before Vec materialization",
  );
  assert.match(profileHelper, /overflow_active_profile/);
  assert.match(profileHelper, /configured_profiles\.pop\(\);/);
  assert.match(
    profileHelper,
    /configured_profiles\.push\(Some\(active_profile_name\.to_string\(\)\)\);/,
  );
  assertBefore(
    profileHelper,
    "profile_names.push(None);",
    "profile_names.extend(configured_profiles);",
    "Disabled must remain the first selector row",
  );
});

test("string candidates and matches use the named selector row cap", () => {
  const delegateImplStart = source.indexOf("impl SettingsProfileSelectorDelegate {");
  assert.notEqual(delegateImplStart, -1, "expected SettingsProfileSelectorDelegate impl");
  const constructor = functionBody(source.slice(delegateImplStart), "new");
  const candidatesHelper = functionBody(source, "profile_match_candidates");
  const emptyMatches = functionBody(source, "empty_profile_matches");
  const updateMatches = functionBody(source, "update_matches");

  assert.match(constructor, /empty_profile_matches\(profile_match_candidates\(&profile_names\)\)/);
  assertBefore(
    candidatesHelper,
    ".take(MAX_SETTINGS_PROFILE_SELECTOR_MATCHES)",
    "StringMatchCandidate::new",
    "profile candidates must be capped before StringMatchCandidate materialization",
  );
  assertBefore(
    emptyMatches,
    ".take(MAX_SETTINGS_PROFILE_SELECTOR_MATCHES)",
    "StringMatch {",
    "empty-query rows must be capped before StringMatch materialization",
  );
  assert.match(updateMatches, /let candidates = profile_match_candidates\(&self\.profile_names\);/);
  assert.match(updateMatches, /empty_profile_matches\(candidates\)/);
  assert.match(
    updateMatches,
    /match_strings\([\s\S]*MAX_SETTINGS_PROFILE_SELECTOR_MATCHES,[\s\S]*&Default::default\(\)/,
  );
  assert.doesNotMatch(
    updateMatches,
    /match_strings\([\s\S]*\n\s*100,\s*\n\s*&Default::default\(\)/,
    "fuzzy search must not use an inline row cap",
  );
});

test("selection and profile lookup guard stale match state", () => {
  const selectIfMatching = functionBody(source, "select_if_matching");
  const setSelectedProfile = functionBody(source, "set_selected_profile");
  const selectedProfileForUpdate = functionBody(source, "selected_profile_for_update");
  const profileNameForMatch = functionBody(source, "profile_name_for_match");
  const setter = functionBody(source, "set_selected_index");
  const updateMatches = functionBody(source, "update_matches");
  const renderMatch = functionBody(source, "render_match");

  assert.match(setter, /self\.selected_index = self\.clamped_match_index\(ix\);/);
  assertBefore(
    updateMatches,
    "this.delegate.matches = matches;",
    "this.delegate.clamp_selected_index_to_matches();",
    "async match replacement must clamp stale selected indexes",
  );
  assert.match(selectIfMatching, /self\.profile_name_for_match\(mat\)/);
  assert.match(setSelectedProfile, /self\.selected_profile_for_update\(\)\?/);
  assert.match(selectedProfileForUpdate, /Option<Option<String>>/);
  assertBefore(
    selectedProfileForUpdate,
    ".and_then(|mat| self.profile_name_for_match(mat))",
    ".cloned()",
    "candidate id must be checked before cloning the profile name",
  );
  assert.match(profileNameForMatch, /self\.profile_names\.get\(mat\.candidate_id\)/);
  assert.match(renderMatch, /self\.profile_name_for_match\(mat\)\?/);
});

test("settings profile selector source guard stays scoped to worker-owned files", () => {
  assert.equal(
    sourcePath,
    "crates/settings_profile_selector/src/settings_profile_selector.rs",
  );
  assert.doesNotMatch(sourcePath, /DX\.md|todo\.txt|changelog\.txt/);
  assert.doesNotMatch(
    source,
    /script\/dx-handoff-source-guard-registry\.test\.ts/,
    "this worker guard should not require registry edits",
  );
});
