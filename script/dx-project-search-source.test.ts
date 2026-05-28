import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const sliceBetween = (source: string, start: string, end: string) => {
  const startIndex = source.indexOf(start);
  assert.notEqual(startIndex, -1, `missing start marker: ${start}`);
  const endIndex = source.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `missing end marker after ${start}: ${end}`);
  return source.slice(startIndex, endIndex);
};

test("project search multiline detection uses a sentinel-bounded text reader", () => {
  const source = read("crates/project/src/search.rs");

  assert.match(
    source,
    /const MAX_PROJECT_SEARCH_MULTILINE_FILE_BYTES: u64 = 8 \* 1024 \* 1024;/,
  );
  assert.match(
    source,
    /fn read_bounded_multiline_search_text\(\s+reader: &mut BufReader<Box<dyn Read \+ Send \+ Sync>>,\s+\) -> Result<String>/,
  );

  const helper = sliceBetween(
    source,
    "fn read_bounded_multiline_search_text(",
    "impl SearchQuery",
  );
  assert.match(helper, /\.take\(MAX_PROJECT_SEARCH_MULTILINE_FILE_BYTES \+ 1\)/);
  assert.match(helper, /\.read_to_end\(&mut bytes\)\?/);
  assert.match(helper, /bytes\.len\(\) as u64 > MAX_PROJECT_SEARCH_MULTILINE_FILE_BYTES/);
  assert.match(helper, /bail!\(/);

  const oversizedCheck = helper.indexOf(
    "bytes.len() as u64 > MAX_PROJECT_SEARCH_MULTILINE_FILE_BYTES",
  );
  const materialization = helper.indexOf("String::from_utf8(bytes)");
  assert.ok(oversizedCheck >= 0, "helper should reject the sentinel byte");
  assert.ok(
    materialization > oversizedCheck,
    "UTF-8 string materialization must happen after the size rejection",
  );
});

test("project search detect avoids direct read_to_string materialization", () => {
  const source = read("crates/project/src/search.rs");
  const detect = sliceBetween(
    source,
    "pub(crate) async fn detect(",
    "    /// Returns the replacement text",
  );

  assert.match(detect, /let text = read_bounded_multiline_search_text\(&mut reader\)\?;/);
  assert.equal(
    [...detect.matchAll(/read_bounded_multiline_search_text\(&mut reader\)\?/g)].length,
    2,
    "both multiline text and regex detection paths should use the bounded helper",
  );
  assert.doesNotMatch(detect, /read_to_string/);
});
