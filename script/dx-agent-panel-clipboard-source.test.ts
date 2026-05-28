import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const agentPanel = readFileSync("crates/agent_ui/src/agent_panel.rs", "utf8");
const sharedThread = readFileSync("crates/agent/src/db.rs", "utf8");

const functionBody = (name: string) => {
  const start = agentPanel.indexOf(`fn ${name}(`);
  assert.ok(start >= 0, `expected ${name} in agent_panel.rs`);

  const nextFunction = agentPanel.indexOf("\n    fn ", start + 1);
  assert.ok(nextFunction > start, `expected ${name} to end before the next function`);

  return agentPanel.slice(start, nextFunction);
};

test("native agent thread clipboard payload limits are explicit", () => {
  assert.match(
    agentPanel,
    /const MAX_THREAD_CLIPBOARD_DECODED_BYTES: usize = 16 \* 1024 \* 1024;/,
  );
  assert.match(
    agentPanel,
    /const MAX_THREAD_CLIPBOARD_ENCODED_BYTES: usize =\s*\(\(MAX_THREAD_CLIPBOARD_DECODED_BYTES \+ 2\) \/ 3\) \* 4;/,
  );
  assert.match(
    agentPanel,
    /const MAX_THREAD_CLIPBOARD_DECOMPRESSED_BYTES: usize = 64 \* 1024 \* 1024;/,
  );
});

test("copying a native agent thread refuses oversized payloads before clipboard write", () => {
  const copyThread = functionBody("copy_thread_to_clipboard");

  assert.match(copyThread, /thread_data\.len\(\) > MAX_THREAD_CLIPBOARD_DECODED_BYTES/);
  assert.match(copyThread, /encoded\.len\(\) > MAX_THREAD_CLIPBOARD_ENCODED_BYTES/);
  assert.ok(
    copyThread.indexOf("thread_data.len() > MAX_THREAD_CLIPBOARD_DECODED_BYTES") <
      copyThread.indexOf("base64::Engine::encode"),
    "decoded export size must be checked before base64 encoding",
  );
  assert.ok(
    copyThread.indexOf("encoded.len() > MAX_THREAD_CLIPBOARD_ENCODED_BYTES") <
      copyThread.indexOf("cx.write_to_clipboard"),
    "encoded export size must be checked before writing clipboard text",
  );
  assert.match(copyThread, /Thread is too large to copy to clipboard/);
});

test("loading a native agent thread rejects oversized clipboard text before parsing", () => {
  const loadThread = functionBody("load_thread_from_clipboard");

  assert.match(
    loadThread,
    /Self::clipboard_text_with_size_limit\(\s*&clipboard,\s*MAX_THREAD_CLIPBOARD_ENCODED_BYTES,\s*\)/,
  );
  assert.doesNotMatch(loadThread, /clipboard\.text\(\)/);
  assert.match(loadThread, /encoded\.len\(\) > MAX_THREAD_CLIPBOARD_ENCODED_BYTES/);
  assert.match(loadThread, /thread_data\.len\(\) > MAX_THREAD_CLIPBOARD_DECODED_BYTES/);
  assert.ok(
    loadThread.indexOf("Self::clipboard_text_with_size_limit") <
      loadThread.indexOf("base64::Engine::decode"),
    "encoded import size must be checked while reading clipboard text before base64 decode",
  );
  assert.ok(
    loadThread.indexOf("thread_data.len() > MAX_THREAD_CLIPBOARD_DECODED_BYTES") <
      loadThread.indexOf("SharedThread::from_bytes_with_decompressed_size_limit"),
    "decoded import size must be checked before SharedThread deserialization",
  );
  assert.match(
    loadThread,
    /SharedThread::from_bytes_with_decompressed_size_limit\(\s*&thread_data,\s*MAX_THREAD_CLIPBOARD_DECOMPRESSED_BYTES,\s*\)/,
  );
  assert.match(loadThread, /Clipboard thread data is too large/);
});

test("creating a skill from a URL bounds clipboard text before trimming or filtering", () => {
  const createSkillFromUrl = functionBody("deploy_skill_creator_from_url");

  assert.match(agentPanel, /const MAX_SKILL_URL_CLIPBOARD_BYTES: usize = 64 \* 1024;/);
  assert.match(
    createSkillFromUrl,
    /Self::clipboard_text_with_size_limit\(\s*&clipboard,\s*MAX_SKILL_URL_CLIPBOARD_BYTES,\s*\)/,
  );
  assert.doesNotMatch(createSkillFromUrl, /clipboard\.text\(\)/);
  assert.match(createSkillFromUrl, /Err\(\(\)\) => None/);
  assert.ok(
    createSkillFromUrl.indexOf("Self::clipboard_text_with_size_limit") <
      createSkillFromUrl.indexOf(".trim()"),
    "URL clipboard text must be bounded before trimming",
  );
  assert.ok(
    createSkillFromUrl.indexOf("Self::clipboard_text_with_size_limit") <
      createSkillFromUrl.indexOf("is_supported_skill_url"),
    "URL clipboard text must be bounded before supported-URL filtering",
  );
});

test("loading a native agent thread bounds decompression before JSON parse", () => {
  const sharedThreadImplStart = sharedThread.indexOf("impl SharedThread");
  const sharedThreadImplEnd = sharedThread.indexOf("impl DbThread", sharedThreadImplStart);
  assert.ok(sharedThreadImplStart >= 0, "expected SharedThread impl");
  assert.ok(sharedThreadImplEnd > sharedThreadImplStart, "expected SharedThread impl to end");
  const sharedThreadImpl = sharedThread.slice(sharedThreadImplStart, sharedThreadImplEnd);

  assert.match(sharedThreadImpl, /const MAX_DECOMPRESSED_BYTES: usize = 64 \* 1024 \* 1024;/);
  assert.match(
    sharedThreadImpl,
    /pub fn from_bytes_with_decompressed_size_limit\(\s*data: &\[u8\],\s*max_decompressed_bytes: usize,\s*\) -> Result<Self>/,
  );
  assert.match(sharedThreadImpl, /zstd::stream::read::Decoder::new\(data\)\?/);
  assert.match(sharedThreadImpl, /\.take\(read_limit\)\s*\.read_to_end\(&mut decompressed\)\?/);
  assert.match(sharedThreadImpl, /decompressed\.len\(\) > max_decompressed_bytes/);
  assert.doesNotMatch(sharedThreadImpl, /zstd::decode_all\(data\)/);

  assert.ok(
    sharedThreadImpl.indexOf("decompressed.len() > max_decompressed_bytes") <
      sharedThreadImpl.indexOf("serde_json::from_slice(&decompressed)"),
    "shared thread JSON parsing must happen only after bounded decompression succeeds",
  );
});
