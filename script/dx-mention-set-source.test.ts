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
