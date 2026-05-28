import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/acp_tools/src/acp_tools.rs";
const source = readFileSync(sourcePath, "utf8");

const functionBody = (name: string): string => {
  const start = source.search(new RegExp(`fn\\s+${name}\\s*\\(`));
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
};

test("ACP request id chips are bounded before UI materialization", () => {
  assert.match(source, /const MAX_REQUEST_ID_CHIP_CHARS: usize = 96;/);

  const requestIdChipLabel = functionBody("request_id_chip_label");
  assert.match(requestIdChipLabel, /request_id\.to_string\(\)/);
  assert.match(requestIdChipLabel, /MAX_REQUEST_ID_CHIP_CHARS/);
  assert.match(requestIdChipLabel, /\.char_indices\(\)/);
  assert.match(requestIdChipLabel, /label\.truncate\(truncate_at\)/);
  assert.match(requestIdChipLabel, /label\.push_str\("..."\)/);

  const renderMessage = functionBody("render_message");
  assert.match(renderMessage, /ui::Chip::new\(request_id_chip_label\(req_id\)\)/);
  assert.doesNotMatch(renderMessage, /ui::Chip::new\(req_id\.to_string\(\)\)/);
});
