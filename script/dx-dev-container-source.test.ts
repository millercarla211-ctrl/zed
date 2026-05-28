import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/dev_container/src/lib.rs";
const source = readFileSync(sourcePath, "utf8").replace(/\r\n/g, "\n");

function balancedBlock(sourceText: string, startNeedle: string): string {
  const start = sourceText.indexOf(startNeedle);
  assert.notEqual(start, -1, `expected ${startNeedle}`);

  const bodyStart = sourceText.indexOf("{", start);
  assert.ok(bodyStart > start, `expected ${startNeedle} body`);

  let depth = 0;
  for (let index = bodyStart; index < sourceText.length; index += 1) {
    const char = sourceText[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return sourceText.slice(start, index + 1);
      }
    }
  }

  assert.fail(`expected ${startNeedle} body to close`);
}

function delegateMethod(delegateName: string, methodName: string): string {
  const delegateImpl = balancedBlock(source, `impl PickerDelegate for ${delegateName} {`);
  return balancedBlock(delegateImpl, `fn ${methodName}`);
}

test("dev container template picker render path ignores stale match rows", () => {
  const renderMatch = delegateMethod("TemplatePickerDelegate", "render_match");

  assert.match(
    renderMatch,
    /\.matching_indices\s*\.get\(ix\)\s*\.and_then\(\|ix\|\s*self\.candidate_templates\.get\(\*ix\)\)/,
  );
  assert.doesNotMatch(
    renderMatch,
    /\.matching_indices\s*\[\s*ix\s*\]/,
    "template rendering must not index stale matching rows directly",
  );
});

test("dev container feature picker render path ignores stale match rows", () => {
  const renderMatch = delegateMethod("FeaturePickerDelegate", "render_match");

  assert.match(
    renderMatch,
    /\.matching_indices\s*\.get\(ix\)\s*\.and_then\(\|ix\|\s*self\.candidate_features\.get\(\*ix\)\)/,
  );
  assert.doesNotMatch(
    renderMatch,
    /\.matching_indices\s*\[\s*ix\s*\]/,
    "feature rendering must not index stale matching rows directly",
  );
  assert.doesNotMatch(
    renderMatch,
    /\.candidate_features\s*\[/,
    "feature rendering must not index stale feature rows directly",
  );
});

test("dev container source guard stays scoped to owned production files", () => {
  assert.equal(sourcePath, "crates/dev_container/src/lib.rs");
  assert.doesNotMatch(sourcePath, /DX\.md|todo\.txt|changelog\.txt/);
});
