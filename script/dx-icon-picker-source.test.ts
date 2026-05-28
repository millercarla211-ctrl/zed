import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/icon_picker/src/icon_picker.rs";

const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const source = productionSource(readFileSync(sourcePath, "utf8"));

function functionBody(source: string, name: string): string {
  const start = source.search(new RegExp(`fn\\s+${name}(?:\\s*<[^>]+>)?\\s*\\(`));
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
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("icon picker bounds TSV sample columns before pack summaries store them", () => {
  assert.match(source, /const MAX_ICON_PACK_SAMPLE_NAMES: usize = 2;/);

  const sampleHelper = functionBody(source, "icon_pack_sample_names");
  assert.match(sampleHelper, /Vec::with_capacity\(MAX_ICON_PACK_SAMPLE_NAMES\)/);
  assertBefore(
    sampleHelper,
    ".filter(|name| !name.is_empty())",
    ".take(MAX_ICON_PACK_SAMPLE_NAMES)",
    "empty sample columns should be discarded before the sample cap is applied",
  );
  assert.match(sampleHelper, /\.map\(SharedString::from\)/);

  const staticSummaries = functionBody(source, "static_icon_pack_summaries");
  assert.match(staticSummaries, /let sample_names = icon_pack_sample_names\(columns\);/);
  assert.doesNotMatch(
    staticSummaries,
    /Vec::with_capacity\(columns\.size_hint\(\)\.0\)/,
    "TSV row width must not drive sample-name allocation",
  );
});

test("icon picker representative loops share the sample-column cap", () => {
  const representativeSummaries = functionBody(
    source,
    "representative_icons_from_pack_summaries",
  );
  const externalCatalog = functionBody(source, "load_external_icon_catalog");

  assert.match(representativeSummaries, /for index in 0\.\.MAX_ICON_PACK_SAMPLE_NAMES/);
  assert.match(externalCatalog, /for index in 0\.\.MAX_ICON_PACK_SAMPLE_NAMES/);
  assert.doesNotMatch(
    `${representativeSummaries}\n${externalCatalog}`,
    /for index in 0\.\.2/,
    "representative rendering should stay tied to the bounded TSV sample count",
  );
});

test("icon picker source guard is focused on production icon picker code", () => {
  assert.equal(sourcePath, "crates/icon_picker/src/icon_picker.rs");
  assert.doesNotMatch(sourcePath, /test/i);
  assert.doesNotMatch(
    source,
    /#\[cfg\(test\)\]/,
    "source guard should only inspect production icon picker code",
  );
});
