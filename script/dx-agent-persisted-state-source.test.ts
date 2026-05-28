import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const agentPanel = read("crates/agent_ui/src/agent_panel.rs");
const draftPromptStore = read("crates/agent_ui/src/draft_prompt_store.rs");

const functionBody = (source: string, name: string) => {
  const start = source.indexOf(`fn ${name}(`);
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

const assertAgentPanelReadBoundsBeforeParse = ({
  functionName,
  maxConst,
  parseType,
}: {
  functionName: string;
  maxConst: string;
  parseType: string;
}) => {
  const body = functionBody(agentPanel, functionName);
  const boundCall = "ensure_kvp_json_within_limit";
  const parseCall = `serde_json::from_str::<${parseType}>(&json)`;
  const boundIndex = body.indexOf(boundCall);
  const maxIndex = body.indexOf(maxConst, boundIndex);
  const parseIndex = body.indexOf(parseCall);

  assert.ok(
    boundIndex >= 0 && maxIndex > boundIndex,
    `${functionName} must check ${maxConst} before parsing`,
  );
  assert.ok(parseIndex >= 0, `${functionName} must parse ${parseType}`);
  assert.ok(
    boundIndex < parseIndex && maxIndex < parseIndex,
    `${functionName} must reject oversized KVP JSON before deserialization`,
  );
};

test("agent panel persisted KVP JSON limits are explicit", () => {
  assert.match(
    agentPanel,
    /const MAX_LAST_USED_AGENT_JSON_BYTES: usize = 16 \* 1024;/,
  );
  assert.match(
    agentPanel,
    /const MAX_LAST_CREATED_ENTRY_KIND_JSON_BYTES: usize = 4 \* 1024;/,
  );
  assert.match(
    agentPanel,
    /const MAX_SERIALIZED_AGENT_PANEL_JSON_BYTES: usize = 256 \* 1024;/,
  );

  const helper = functionBody(agentPanel, "ensure_kvp_json_within_limit");
  assert.match(helper, /json\.len\(\) > max_bytes/);
  assert.match(helper, /KVP payload is too large/);
  assert.match(helper, /log_err\(\)/);
});

test("agent panel persisted KVP JSON is bounded before deserialization", () => {
  assertAgentPanelReadBoundsBeforeParse({
    functionName: "read_global_last_used_agent",
    maxConst: "MAX_LAST_USED_AGENT_JSON_BYTES",
    parseType: "LastUsedAgent",
  });
  assertAgentPanelReadBoundsBeforeParse({
    functionName: "read_global_last_created_entry_kind",
    maxConst: "MAX_LAST_CREATED_ENTRY_KIND_JSON_BYTES",
    parseType: "LastCreatedEntryKind",
  });
  assertAgentPanelReadBoundsBeforeParse({
    functionName: "read_serialized_panel",
    maxConst: "MAX_SERIALIZED_AGENT_PANEL_JSON_BYTES",
    parseType: "SerializedAgentPanel",
  });
  assertAgentPanelReadBoundsBeforeParse({
    functionName: "read_legacy_serialized_panel",
    maxConst: "MAX_SERIALIZED_AGENT_PANEL_JSON_BYTES",
    parseType: "SerializedAgentPanel",
  });
});

test("draft prompt KVP JSON is bounded before deserialization", () => {
  assert.match(
    draftPromptStore,
    /const MAX_DRAFT_PROMPT_JSON_BYTES: usize = 1024 \* 1024;/,
  );

  const helper = functionBody(
    draftPromptStore,
    "ensure_draft_prompt_json_within_limit",
  );
  assert.match(helper, /raw\.len\(\) > MAX_DRAFT_PROMPT_JSON_BYTES/);
  assert.match(helper, /KVP payload is too large/);
  assert.match(helper, /log_err\(\)/);

  const readBody = functionBody(draftPromptStore, "read");
  const boundCall = "ensure_draft_prompt_json_within_limit(&raw)";
  const parseCall = "serde_json::from_str(&raw)";
  assert.ok(readBody.includes(boundCall), "draft prompt read must check size");
  assert.ok(readBody.includes(parseCall), "draft prompt read must parse JSON");
  assert.ok(
    readBody.indexOf(boundCall) < readBody.indexOf(parseCall),
    "draft prompt JSON must be rejected before deserialization",
  );
});
