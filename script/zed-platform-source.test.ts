import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const sliceBetween = (source: string, startNeedle: string, endNeedle: string) => {
  const start = source.indexOf(startNeedle);
  assert.notEqual(start, -1, `missing ${startNeedle}`);
  const end = source.indexOf(endNeedle, start + startNeedle.length);
  assert.notEqual(end, -1, `missing ${endNeedle}`);
  return source.slice(start, end);
};

test("copy_app_bundle caps rsync stderr before showing copy failures", () => {
  const source = read("crates/zed/src/zed/move_to_applications.rs");
  const copyAppBundle = sliceBetween(
    source,
    "async fn copy_app_bundle",
    "fn restart_into",
  );
  const stderrHelper = sliceBetween(
    source,
    "fn copy_app_bundle_stderr_display",
    "fn restart_into",
  );

  assert.match(source, /const MAX_COPY_APP_BUNDLE_STDERR_BYTES: usize = 2048;/);
  assert.match(source, /const MAX_COPY_APP_BUNDLE_STDERR_CHARS: usize = 500;/);
  assert.match(
    copyAppBundle,
    /let stderr = copy_app_bundle_stderr_display\(&output\.stderr\);/,
  );
  assert.match(copyAppBundle, /"failed to copy app bundle: \{stderr\}"/);
  assert.doesNotMatch(source, /String::from_utf8_lossy\(&output\.stderr\)/);
  assert.match(stderrHelper, /stderr\.len\(\) > MAX_COPY_APP_BUNDLE_STDERR_BYTES/);
  assert.match(stderrHelper, /&stderr\[..visible_len\]/);
  assert.match(stderrHelper, /String::from_utf8_lossy\(&stderr\[..visible_len\]\)/);
  assert.match(stderrHelper, /split_whitespace\(\)\.collect::<Vec<_>>\(\)\.join\(" "\)/);
  assert.match(
    stderrHelper,
    /take\(MAX_COPY_APP_BUNDLE_STDERR_CHARS\.saturating_sub\(3\)\)/,
  );
  assert.match(stderrHelper, /display\.push_str\("\.\.\."\)/);
});
