import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

test("skill mention file ingestion uses a sentinel bounded UTF-8 read", () => {
  const source = productionSource(read("crates/agent_ui/src/mention_set.rs"));
  const helperStart = source.indexOf("fn read_skill_mention_file(");
  const helperEnd = source.indexOf("\nfn compute_disambiguated_labels");
  const confirmStart = source.indexOf("fn confirm_mention_for_skill(");
  const confirmEnd = source.indexOf("\n    fn confirm_mention_for_rule(");

  assert.ok(helperStart >= 0, "expected a focused skill mention read helper");
  assert.ok(helperEnd > helperStart, "expected helper before label disambiguation");
  assert.ok(confirmStart >= 0, "expected skill mention confirmation path");
  assert.ok(confirmEnd > confirmStart, "expected focused skill mention confirmation");

  const helper = source.slice(helperStart, helperEnd);
  const confirm = source.slice(confirmStart, confirmEnd);
  const maxBytes = "MAX_SKILL_MENTION_FILE_BYTES";
  const oversizeCheck = "bytes.len() as u64 > MAX_SKILL_MENTION_FILE_BYTES";
  const utf8Decode = "String::from_utf8(bytes)";

  assert.match(source, /const MAX_SKILL_MENTION_FILE_BYTES: u64 = \d+ \* 1024;/);
  assert.match(helper, /\.take\(MAX_SKILL_MENTION_FILE_BYTES \+ 1\)/);
  assert.match(helper, /\.read_to_end\(&mut bytes\)/);
  assert.match(helper, new RegExp(oversizeCheck.replace(/[().+]/g, "\\$&")));
  assert.match(helper, new RegExp(utf8Decode.replace(/[().+]/g, "\\$&")));
  assert.ok(
    helper.indexOf(oversizeCheck) < helper.indexOf(utf8Decode),
    "skill files must be rejected over the max byte limit before UTF-8 decoding",
  );
  assert.match(confirm, /read_skill_mention_file\(&skill_file_path\)/);
  assert.doesNotMatch(source, /\bstd::fs::read_to_string\s*\(/);
  assert.ok(source.includes(maxBytes), "expected max byte constant to remain source-visible");
});

test("external pasted image files are bounded before format guessing", () => {
  const source = read("crates/agent_ui/src/mention_set.rs");
  const helperStart = source.indexOf("fn read_external_raster_image_file_bytes(");
  const helperEnd = source.indexOf("\nfn image_format_from_external_content", helperStart);
  const loaderStart = source.indexOf("pub(crate) fn load_external_image_from_path(");
  const loaderEnd = source.indexOf("\npub(crate) fn paste_images_as_context", loaderStart);

  assert.ok(helperStart >= 0, "expected a focused external raster image read helper");
  assert.ok(helperEnd > helperStart, "expected image read helper before format mapping");
  assert.ok(loaderStart >= 0, "expected external image loader");
  assert.ok(loaderEnd > loaderStart, "expected focused external image loader slice");

  const helper = source.slice(helperStart, helperEnd);
  const loader = source.slice(loaderStart, loaderEnd);
  const maxBytes = "MAX_EXTERNAL_RASTER_IMAGE_FILE_BYTES";
  const metadataCheck = "metadata.len() > MAX_EXTERNAL_RASTER_IMAGE_FILE_BYTES";
  const sentinelCheck = "bytes.len() as u64 > MAX_EXTERNAL_RASTER_IMAGE_FILE_BYTES";
  const boundedRead = "read_external_raster_image_file_bytes(path)?";
  const formatGuess = "image::guess_format(&content)";
  const imageBuild = "Image::from_bytes(format, content)";

  assert.match(source, /const MAX_EXTERNAL_RASTER_IMAGE_FILE_BYTES: u64 = \d+ \* 1024 \* 1024;/);
  assert.match(helper, /std::fs::metadata\(path\)/);
  assert.match(helper, new RegExp(metadataCheck.replace(/[().+]/g, "\\$&")));
  assert.match(
    helper,
    /std::fs::File::open\(path\)\s*\.ok\(\)\?\s*\.take\(MAX_EXTERNAL_RASTER_IMAGE_FILE_BYTES \+ 1\)/,
  );
  assert.match(helper, /\.read_to_end\(&mut bytes\)\.ok\(\)\?/);
  assert.match(helper, new RegExp(sentinelCheck.replace(/[().+]/g, "\\$&")));
  assert.match(helper, /eq_ignore_ascii_case\("svg"\)/);
  assert.ok(
    helper.indexOf(metadataCheck) < helper.indexOf("std::fs::File::open(path)"),
    "image files must be rejected by metadata before opening for byte reads",
  );
  assert.ok(
    helper.indexOf(sentinelCheck) > helper.indexOf(".read_to_end(&mut bytes)"),
    "image file sentinel check must run after the capped read",
  );
  assert.ok(
    loader.indexOf(boundedRead) < loader.indexOf(formatGuess),
    "external image bytes must be bounded before image format guessing",
  );
  assert.ok(
    loader.indexOf(formatGuess) < loader.indexOf(imageBuild),
    "external image format guessing should still happen before Image::from_bytes",
  );
  assert.doesNotMatch(loader, /\bstd::fs::read\s*\(/);
  assert.ok(source.includes(maxBytes), "expected external image byte constant to remain source-visible");
});

test("fetch mention client errors render capped compact response text", () => {
  const source = read("crates/agent_ui/src/mention_set.rs");
  const displayHelperStart = source.indexOf("fn format_fetch_mention_error_body(");
  const bodyHelperStart = source.indexOf("async fn read_fetch_mention_body(");
  const fetchStart = source.indexOf("async fn fetch_url_content(");
  const contentTypeStart = source.indexOf("let Some(content_type)", fetchStart);

  assert.ok(displayHelperStart >= 0, "expected a focused fetch error display helper");
  assert.ok(bodyHelperStart > displayHelperStart, "expected display helper before body reads");
  assert.ok(fetchStart > bodyHelperStart, "expected fetch helper after bounded body read helper");
  assert.ok(contentTypeStart > fetchStart, "expected content-type handling after client-error branch");

  const displayHelper = source.slice(displayHelperStart, bodyHelperStart);
  const fetch = source.slice(fetchStart);
  const clientErrorBranch = source.slice(
    source.indexOf("if response.status().is_client_error()", fetchStart),
    contentTypeStart,
  );
  const maxDisplayBytes = "MAX_FETCH_MENTION_ERROR_BODY_DISPLAY_BYTES";
  const displayHelperCall = "format_fetch_mention_error_body(&body)";
  const escaped = (text: string) => text.replace(/[().+]/g, "\\$&");

  assert.match(source, /const MAX_FETCH_MENTION_ERROR_BODY_DISPLAY_BYTES: usize = \d+ \* 1024;/);
  assert.match(displayHelper, new RegExp(escaped(`body.len() > ${maxDisplayBytes}`)));
  assert.match(
    displayHelper,
    /body\s*\.len\(\)\s*\.min\(MAX_FETCH_MENTION_ERROR_BODY_DISPLAY_BYTES\)/,
  );
  assert.match(displayHelper, /String::from_utf8_lossy\(visible_body\)/);
  assert.match(displayHelper, /\.split_whitespace\(\)/);
  assert.match(displayHelper, /\[truncated\]/);
  assert.match(fetch, new RegExp(escaped(displayHelperCall)));
  assert.doesNotMatch(clientErrorBranch, /String::from_utf8_lossy/);
  assert.doesNotMatch(fetch, /String::from_utf8_lossy\(body\.as_slice\(\)\)/);
});

test("fetch mention bodies are bounded before text or JSON conversion", () => {
  const source = read("crates/agent_ui/src/mention_set.rs");
  const bodyHelperStart = source.indexOf("async fn read_fetch_mention_body(");
  const fetchStart = source.indexOf("async fn fetch_url_content(");
  const bodyHelperEnd = fetchStart;

  assert.ok(fetchStart >= 0, "expected fetch mention helper");

  assert.ok(bodyHelperStart >= 0, "expected bounded fetch body helper");
  assert.ok(bodyHelperEnd > bodyHelperStart, "expected fetch body helper before fetch parsing");

  const fetch = source.slice(fetchStart);
  const bodyHelper = source.slice(bodyHelperStart, bodyHelperEnd);
  const maxBytes = "MAX_FETCH_MENTION_BODY_BYTES";
  const helperCall = "read_fetch_mention_body(response.body_mut()).await";
  const displayHelperCall = "format_fetch_mention_error_body(&body)";
  const oversizeCheck = "body.len() as u64 > MAX_FETCH_MENTION_BODY_BYTES";

  assert.match(source, /const MAX_FETCH_MENTION_BODY_BYTES: u64 = \d+ \* 1024 \* 1024;/);
  assert.match(fetch, new RegExp(helperCall.replace(/[().+]/g, "\\$&")));
  assert.match(bodyHelper, /\.take\(MAX_FETCH_MENTION_BODY_BYTES \+ 1\)/);
  assert.match(bodyHelper, /\.read_to_end\(&mut body\)/);
  assert.match(bodyHelper, new RegExp(oversizeCheck.replace(/[().+]/g, "\\$&")));
  assert.ok(
    fetch.indexOf(helperCall) < fetch.indexOf("response.status().is_client_error()"),
    "fetch body cap must apply before client-error response text is rendered",
  );
  assert.ok(
    fetch.indexOf(helperCall) < fetch.indexOf(displayHelperCall),
    "fetch body cap must apply before client-error display text is rendered",
  );
  assert.ok(
    fetch.indexOf(helperCall) < fetch.indexOf("convert_html_to_markdown(&body[..]"),
    "fetch body cap must apply before HTML conversion",
  );
  assert.ok(
    fetch.indexOf(helperCall) < fetch.indexOf("std::str::from_utf8(&body)"),
    "fetch body cap must apply before plaintext conversion",
  );
  assert.ok(
    fetch.indexOf(helperCall) < fetch.indexOf("serde_json::from_slice(&body)"),
    "fetch body cap must apply before JSON parsing",
  );
  assert.ok(source.includes(maxBytes), "expected fetch body byte constant to remain source-visible");
});
