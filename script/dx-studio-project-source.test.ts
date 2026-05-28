import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (source: string) => source.split(/\r?\n/).length;
const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const directReadToString = /\b(?:fs|std::fs)::read_to_string\s*\(/;

test("DX Studio project detection keeps marker and Cargo.toml reads bounded", () => {
  const source = read("crates/web_preview/src/dx_studio/project.rs");

  assert.ok(lineCount(source) < 240, "project detection should stay compact");
  assert.doesNotMatch(source, directReadToString);
  assert.match(source, /const MAX_DX_CARGO_TOML_SCAN_BYTES: u64 = 256 \* 1024;/);
  assert.match(
    source,
    /fn read_bounded_utf8_file\(path: &Path, max_bytes: u64\) -> Option<String>/,
  );
  assert.match(source, /\.take\(max_bytes \+ 1\)/);
  assert.match(source, /\.read_to_end\(&mut bytes\)/);
  assert.match(source, /bytes\.len\(\) as u64 > max_bytes/);
  assert.match(source, /cargo_toml_contains_dx_www_marker\(&cargo_toml\)/);
  assert.match(
    source,
    /read_bounded_utf8_file\(path, MAX_DX_MARKER_SCAN_BYTES\)/,
  );
  assert.match(
    source,
    /read_bounded_utf8_file\(path, MAX_DX_CARGO_TOML_SCAN_BYTES\)/,
  );
});

test("DX Studio source edits bound source file reads before UTF-8 decoding", () => {
  const fullSource = read("crates/web_preview/src/dx_studio_source_edit.rs");
  const source = productionSource(fullSource);

  assert.ok(lineCount(fullSource) < 580, "source edit root should stay compact");
  assert.doesNotMatch(source, directReadToString);
  assert.match(source, /fn read_source_file_for_edit\(source: &Path\) -> Result<String>/);
  assert.match(source, /\.take\(DX_STUDIO_MAX_SOURCE_FILE_BYTES \+ 1\)/);
  assert.match(source, /\.read_to_end\(&mut bytes\)/);
  assert.match(source, /bytes\.len\(\) as u64 > DX_STUDIO_MAX_SOURCE_FILE_BYTES/);
  assert.match(source, /String::from_utf8\(bytes\)/);
  assert.match(source, /read_source_file_for_edit\(&source\)\?/);

  const sizeValidation = source.indexOf(
    "ensure_source_file_size_allows_edit(&source, metadata.len())?",
  );
  const boundedRead = source.indexOf("read_source_file_for_edit(&source)?");
  const overLimitCheck = source.indexOf(
    "bytes.len() as u64 > DX_STUDIO_MAX_SOURCE_FILE_BYTES",
  );
  const utf8Decode = source.indexOf("String::from_utf8(bytes)");

  assert.ok(sizeValidation >= 0, "source edit should validate metadata size first");
  assert.ok(
    boundedRead > sizeValidation,
    "source edit should read the source only after metadata size validation",
  );
  assert.ok(overLimitCheck >= 0, "source edit should reject sentinel over-limit reads");
  assert.ok(
    utf8Decode > overLimitCheck,
    "source edit should reject over-limit reads before UTF-8 decoding",
  );
});
