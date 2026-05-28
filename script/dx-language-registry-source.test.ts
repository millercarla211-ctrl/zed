import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const registrySourcePath = "crates/language/src/language_registry.rs";
const settingsSourcePath = "crates/language/src/language_settings.rs";
const registrySource = read(registrySourcePath);
const settingsSource = read(settingsSourcePath);

function functionBody(source: string, name: string): string {
  const fnIndex = source.indexOf(`fn ${name}`);
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
    typeof before === "string" ? haystack.indexOf(before) : haystack.match(before)?.index ?? -1;
  const afterIndex =
    typeof after === "string" ? haystack.indexOf(after) : haystack.match(after)?.index ?? -1;
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("language registry caps names before collection and sorting", () => {
  assert.match(
    registrySource,
    /const MAX_LANGUAGE_NAMES_TO_MATERIALIZE: usize = \d+;/,
  );
  assert.match(
    registrySource,
    /const MAX_GRAMMAR_NAMES_TO_MATERIALIZE: usize = \d+;/,
  );

  const languageNames = functionBody(registrySource, "language_names");
  assertBefore(
    languageNames,
    ".take(MAX_LANGUAGE_NAMES_TO_MATERIALIZE)",
    ".collect::<Vec<_>>()",
    "language names must be capped before vector materialization",
  );
  assertBefore(
    languageNames,
    ".take(MAX_LANGUAGE_NAMES_TO_MATERIALIZE)",
    "result.sort_unstable_by_key",
    "language names must be capped before sorting",
  );

  const grammarNames = functionBody(registrySource, "grammar_names");
  assertBefore(
    grammarNames,
    ".take(MAX_GRAMMAR_NAMES_TO_MATERIALIZE)",
    ".collect::<Vec<_>>()",
    "grammar names must be capped before vector materialization",
  );
  assertBefore(
    grammarNames,
    ".take(MAX_GRAMMAR_NAMES_TO_MATERIALIZE)",
    "result.sort_unstable_by_key",
    "grammar names must be capped before sorting",
  );
});

test("language registry caps WASM grammar registration and bytes before loading", () => {
  assert.match(
    registrySource,
    /const MAX_REGISTERED_WASM_GRAMMARS: usize = \d+;/,
  );
  assert.match(
    registrySource,
    /const MAX_WASM_GRAMMAR_BYTES: u64 = \d+ \* 1024 \* 1024;/,
  );

  const registerWasm = functionBody(registrySource, "register_wasm_grammars");
  assertBefore(
    registerWasm,
    "cap_wasm_grammar_entries(grammars)",
    "state.grammars.extend",
    "WASM grammar entries must be capped before registry insertion",
  );

  const capWasm = functionBody(registrySource, "cap_wasm_grammar_entries");
  assertBefore(
    capWasm,
    ".take(MAX_REGISTERED_WASM_GRAMMARS)",
    ".collect::<Vec<_>>()",
    "WASM grammar entries must be capped before bounded vector materialization",
  );
  assert.match(capWasm, /warn_truncated_registry_materialization/);
  assert.match(functionBody(registrySource, "warn_truncated_registry_materialization"), /log::warn!/);

  const loadGrammar = functionBody(registrySource, "get_or_load_grammar");
  assertBefore(
    loadGrammar,
    "guard_wasm_grammar_size(&wasm_path)?;",
    "std::fs::read(&wasm_path)",
    "WASM grammar byte size must be checked before reading the file",
  );

  const byteGuard = functionBody(registrySource, "guard_wasm_grammar_size");
  assert.match(byteGuard, /std::fs::metadata\(wasm_path\)/);
  assert.match(byteGuard, /MAX_WASM_GRAMMAR_BYTES/);
  assert.match(byteGuard, /log::warn!/);
  assert.match(byteGuard, /anyhow::bail!/);
});

test("language settings caps user influenced settings vectors before lookups", () => {
  assert.match(
    settingsSource,
    /const MAX_LANGUAGE_SETTINGS_ENTRIES: usize = \d+;/,
  );
  assert.match(
    settingsSource,
    /const MAX_LANGUAGE_FILE_TYPE_ENTRIES: usize = \d+;/,
  );
  assert.match(
    settingsSource,
    /const MAX_LANGUAGE_FILE_TYPE_PATTERNS_PER_LANGUAGE: usize = \d+;/,
  );
  assert.match(
    settingsSource,
    /const MAX_LANGUAGE_SETTINGS_ARRAY_ENTRIES: usize = \d+;/,
  );
  assert.match(
    settingsSource,
    /const MAX_EDIT_PREDICTION_DISABLED_GLOBS: usize = \d+;/,
  );

  const settingsImpl = sliceBetween(
    settingsSource,
    "impl settings::Settings for AllLanguageSettings",
    "#[cfg(test)]",
  );
  assertBefore(
    settingsImpl,
    ".take(MAX_LANGUAGE_SETTINGS_ENTRIES)",
    "languages.insert",
    "language settings entries must be capped before lookup-map insertion",
  );
  assertBefore(
    settingsImpl,
    ".take(MAX_LANGUAGE_FILE_TYPE_ENTRIES)",
    "file_types.insert",
    "language file type entries must be capped before lookup-map insertion",
  );
  assertBefore(
    settingsImpl,
    ".take(MAX_EDIT_PREDICTION_DISABLED_GLOBS)",
    ".collect();",
    "edit prediction disabled globs must be capped before collection",
  );
  assert.match(
    settingsImpl,
    /language_servers:\s*cap_language_settings_array\(\s*settings\.language_servers\.unwrap\(\),\s*MAX_LANGUAGE_SETTINGS_ARRAY_ENTRIES,/,
  );
  assert.match(
    settingsImpl,
    /edit_predictions_disabled_in:\s*cap_language_settings_array\(\s*settings\.edit_predictions_disabled_in\.unwrap\(\),\s*MAX_LANGUAGE_SETTINGS_ARRAY_ENTRIES,/,
  );
  assert.match(
    settingsImpl,
    /debuggers:\s*cap_language_settings_array\(\s*settings\.debuggers\.unwrap\(\),\s*MAX_LANGUAGE_SETTINGS_ARRAY_ENTRIES,/,
  );

  const patternHelper = functionBody(settingsSource, "capped_language_file_type_patterns");
  assertBefore(
    patternHelper,
    ".take(MAX_LANGUAGE_FILE_TYPE_PATTERNS_PER_LANGUAGE)",
    ".collect::<Vec<_>>()",
    "file type patterns must be capped before vector materialization",
  );
  assert.match(patternHelper, /warn_truncated_language_settings_collection/);
  assert.match(
    functionBody(settingsSource, "warn_truncated_language_settings_collection"),
    /log::warn!/,
  );
});

test("language server settings resolution caps configured and available arrays", () => {
  const resolve = functionBody(settingsSource, "resolve_language_servers");
  assertBefore(
    resolve,
    ".take(MAX_LANGUAGE_SETTINGS_ARRAY_ENTRIES)",
    ".partition_map",
    "configured language servers must be capped before partitioning",
  );

  const rest = resolve.slice(resolve.indexOf("let rest ="));
  assertBefore(
    rest,
    ".take(MAX_LANGUAGE_SETTINGS_ARRAY_ENTRIES)",
    ".collect::<Vec<_>>()",
    "available language servers must be capped before rest collection",
  );

  const resolved = resolve.slice(resolve.indexOf("let mut resolved_language_servers ="));
  assertBefore(
    resolved,
    ".take(MAX_LANGUAGE_SETTINGS_ARRAY_ENTRIES + 1)",
    ".collect::<Vec<_>>()",
    "resolved language servers must use a sentinel cap before collection",
  );
  assert.match(resolve, /resolved_language_servers\.truncate\(MAX_LANGUAGE_SETTINGS_ARRAY_ENTRIES\);/);
  assert.match(resolve, /log::warn!/);
});

test("language registry guard is focused on worker-owned production source", () => {
  assert.equal(registrySourcePath, "crates/language/src/language_registry.rs");
  assert.equal(settingsSourcePath, "crates/language/src/language_settings.rs");
  assert.doesNotMatch(registrySourcePath, /test/i);
  assert.doesNotMatch(settingsSourcePath, /test/i);
});
