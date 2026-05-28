import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const source = readFileSync(
  "crates/toolchain_selector/src/toolchain_selector.rs",
  "utf8",
);

const functionBody = (bodySource: string, name: string) => {
  const start = bodySource.indexOf(`fn ${name}(`);
  assert.ok(start >= 0, `expected ${name}`);

  const bodyStart = bodySource.indexOf("{", start);
  assert.ok(bodyStart > start, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < bodySource.length; index += 1) {
    const char = bodySource[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return bodySource.slice(start, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
};

const assertOrdered = (
  body: string,
  firstNeedle: string,
  secondNeedle: string,
  message: string,
) => {
  const first = body.indexOf(firstNeedle);
  const second = body.indexOf(secondNeedle);

  assert.ok(first >= 0, `${message}: missing ${firstNeedle}`);
  assert.ok(second >= 0, `${message}: missing ${secondNeedle}`);
  assert.ok(first < second, message);
};

test("toolchain selector caps picker row materialization", () => {
  assert.match(
    source,
    /const MAX_TOOLCHAIN_SELECTOR_MATCHES: usize = 100;/,
  );

  const updateMatches = functionBody(source, "update_matches");
  assertOrdered(
    updateMatches,
    ".take(MAX_TOOLCHAIN_SELECTOR_MATCHES)",
    "StringMatch {",
    "empty queries must cap selectable rows before StringMatch allocation",
  );
  assert.match(
    updateMatches,
    /match_strings\([\s\S]*MAX_TOOLCHAIN_SELECTOR_MATCHES,[\s\S]*&Default::default\(\)/,
  );
  assert.doesNotMatch(
    updateMatches,
    /match_strings\([\s\S]*\n\s*100,\s*\n\s*&Default::default\(\)/,
    "fuzzy searches should reuse the named selector row cap",
  );
});

test("toolchain selector reclamps selection after async match replacement", () => {
  const clampHelper = functionBody(source, "clamp_selected_index_to_matches");
  assert.match(
    clampHelper,
    /self\.selected_index = self\s*\.selected_index\s*\.min\(self\.matches\.len\(\)\.saturating_sub\(1\)\);/s,
  );

  const updateMatches = functionBody(source, "update_matches");
  assertOrdered(
    updateMatches,
    "delegate.matches = matches;",
    "delegate.clamp_selected_index_to_matches();",
    "async match replacement must reclamp the selected row before notify",
  );
});
