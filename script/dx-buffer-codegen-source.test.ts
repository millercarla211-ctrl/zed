import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const source = productionSource(read("crates/agent_ui/src/buffer_codegen.rs"));

const sliceBetween = (startNeedle: string, endNeedle: string, from = 0) => {
  const start = source.indexOf(startNeedle, from);
  assert.notEqual(start, -1, `expected ${startNeedle}`);
  const end = source.indexOf(endNeedle, start + startNeedle.length);
  assert.notEqual(end, -1, `expected ${endNeedle} after ${startNeedle}`);
  return source.slice(start, end);
};

test("buffer codegen rejects oversized selected source before building prompts", () => {
  assert.match(
    source,
    /const MAX_INLINE_CODEGEN_SOURCE_BYTES: usize = 512 \* 1024;/,
  );

  const guard = sliceBetween(
    "fn ensure_inline_codegen_source_within_limit(",
    "\nimpl BufferCodegen {",
  );
  assert.match(guard, /source_byte_len > MAX_INLINE_CODEGEN_SOURCE_BYTES/);
  assert.match(guard, /anyhow::bail!\(/);

  const implStart = source.indexOf("impl CodegenAlternative {");
  assert.notEqual(implStart, -1, "expected CodegenAlternative impl");
  const start = sliceBetween("    pub fn start(", "    fn build_request_tools(", implStart);
  const sourceLen = "Self::source_range_byte_len(&self.buffer.read(cx).snapshot(cx), &self.range)";
  const guardCall = "ensure_inline_codegen_source_within_limit(source_byte_len)";

  assert.match(start, new RegExp(sourceLen.replace(/[().+]/g, "\\$&")));
  assert.ok(
    start.indexOf(sourceLen) < start.indexOf(guardCall),
    "selected source length must be measured before the byte guard",
  );
  assert.ok(
    start.indexOf(guardCall) < start.indexOf("if Self::use_streaming_tools"),
    "selected source guard must run before the streaming-tools prompt branch",
  );
  assert.ok(
    start.indexOf(guardCall) < start.indexOf("self.build_request(&model"),
    "selected source guard must run before inline prompt construction",
  );
  assert.match(start, /self\.finish_with_error\(error, cx\);\s*return Ok\(\(\)\);/);
});

test("buffer codegen checks selected source before streaming text materialization", () => {
  const handleStream = sliceBetween(
    "    pub fn handle_stream(",
    "    pub fn current_completion(",
  );
  const guardCall = "ensure_inline_codegen_source_within_limit(source_byte_len)";
  const materializeSelection = ".text_for_range(self.range.start..self.range.end)";

  assert.ok(
    handleStream.indexOf(guardCall) < handleStream.indexOf(materializeSelection),
    "selected source guard must run before collecting selected text",
  );
  assert.ok(
    handleStream.indexOf(guardCall) < handleStream.indexOf("selected_text.to_string()"),
    "selected source guard must run before cloning selected text into a String",
  );
  assert.match(
    handleStream,
    /self\.finish_with_error\(error, cx\);\s*return Task::ready\(\(\)\);/,
  );
});

test("buffer codegen skips batch diff materialization for empty rejected alternatives", () => {
  const setActive = sliceBetween(
    "    pub fn set_active(",
    "    fn handle_buffer_event(",
  );
  const emptyAlternativeGuard =
    "self.edits.is_empty() && self.line_operations.is_empty()";

  assert.ok(
    setActive.indexOf(emptyAlternativeGuard) < setActive.indexOf("self.reapply_batch_diff(cx)"),
    "empty rejected alternatives should not reapply a batch diff over the selected range",
  );
  assert.match(setActive, /self\.diff = Diff::default\(\);\s*cx\.notify\(\);/);
});

test("buffer codegen source-length helper does not materialize buffer text", () => {
  const helper = sliceBetween(
    "    fn source_range_byte_len(",
    "    fn finish_with_error(",
  );

  assert.match(helper, /range\.start\.to_offset\(snapshot\)\.0/);
  assert.match(helper, /range\.end\.to_offset\(snapshot\)\.0/);
  assert.match(helper, /end\.saturating_sub\(start\)/);
  assert.doesNotMatch(helper, /text_for_range|\.text\(\)/);
  assert.doesNotMatch(source, /snapshot\(cx\)\.text\(\)/);
});
