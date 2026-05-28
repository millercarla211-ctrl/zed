import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const sliceBetween = (contents: string, startNeedle: string, endNeedle: string) => {
  const start = contents.indexOf(startNeedle);
  assert.notEqual(start, -1, `missing ${startNeedle}`);
  const end = contents.indexOf(endNeedle, start + startNeedle.length);
  assert.notEqual(end, -1, `missing ${endNeedle}`);
  return contents.slice(start, end);
};

test("built-in JSON schema rendering is byte-capped before parse and pretty-print", () => {
  const main = read("crates/zed/src/main.rs");
  const builtinSchemaOpen = sliceBetween(
    main,
    "OpenRequestKind::BuiltinJsonSchema { schema_path } => {",
    "OpenRequestKind::Setting {",
  );

  assert.ok(
    /const MAX_BUILTIN_JSON_SCHEMA_BYTES: usize = 8 \* 1024 \* 1024;/.test(main),
    "missing 8 MiB built-in JSON schema byte cap",
  );
  assert.ok(
    /json_schema_content\.len\(\) > MAX_BUILTIN_JSON_SCHEMA_BYTES/.test(builtinSchemaOpen),
    "missing built-in schema content size check",
  );
  assert.ok(
    /anyhow::bail!\([^;]*schema_path[^;]*json_schema_content\.len\(\)[^;]*MAX_BUILTIN_JSON_SCHEMA_BYTES/s.test(
      builtinSchemaOpen,
    ),
    "oversized schema error should include path, actual size, and max size",
  );

  const sizeCheck = builtinSchemaOpen.indexOf(
    "json_schema_content.len() > MAX_BUILTIN_JSON_SCHEMA_BYTES",
  );
  const parse = builtinSchemaOpen.indexOf("serde_json::from_str(&json_schema_content)");
  const prettyPrint = builtinSchemaOpen.indexOf("serde_json::to_string_pretty(&json_schema_value)");
  const bufferInsert = builtinSchemaOpen.indexOf("buffer.edit([(0..0, json_schema_content)]");

  assert.ok(sizeCheck >= 0, "built-in schema content should be size checked");
  assert.ok(parse > sizeCheck, "schema parsing must happen after the byte cap");
  assert.ok(
    prettyPrint > sizeCheck,
    "schema pretty-printing must happen after the byte cap",
  );
  assert.ok(
    bufferInsert > prettyPrint,
    "buffer insertion should keep using pretty-printed schema content",
  );
});
