import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const agentPanel = readFileSync("crates/agent_ui/src/agent_panel.rs", "utf8");
const productionAgentPanel = agentPanel.slice(0, agentPanel.indexOf("\n#[cfg(test)]"));

const functionBody = (name: string) => {
  const signature = new RegExp(`\\n    (?:pub(?:\\([^)]*\\))?\\s+)?fn ${name}\\(`);
  const match = signature.exec(agentPanel);
  assert.ok(match?.index, `expected ${name} in agent_panel.rs`);

  const start = match.index + 1;
  const openBrace = agentPanel.indexOf("{", start);
  assert.ok(openBrace > start, `expected ${name} to have a body`);

  let depth = 0;
  for (let index = openBrace; index < agentPanel.length; index++) {
    const char = agentPanel[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return agentPanel.slice(start, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
};

test("agent panel editor text materialization has explicit byte limits", () => {
  assert.match(
    agentPanel,
    /const MAX_AGENT_PANEL_MESSAGE_EDITOR_TEXT_BYTES: usize = 16 \* 1024 \* 1024;/,
  );
  assert.match(
    agentPanel,
    /const MAX_AGENT_PANEL_TITLE_EDITOR_TEXT_BYTES: usize = 16 \* 1024;/,
  );
});

test("message editor text is materialized only through bounded agent panel helpers", () => {
  const helper = functionBody("message_editor_text_with_size_limit");
  assert.match(helper, /message_editor_within_size_limit/);
  assert.match(helper, /message_editor\.read\(cx\)\.text\(cx\)/);
  assert.ok(
    helper.indexOf("message_editor_within_size_limit") <
      helper.indexOf("message_editor.read(cx).text(cx)"),
    "message editor byte length must be checked before cloning text",
  );

  const sizeHelper = functionBody("message_editor_within_size_limit");
  assert.match(sizeHelper, /message_editor\.read\(cx\)\.text_byte_len\(cx\) > max_bytes/);
  assert.doesNotMatch(sizeHelper, /\.text\(cx\)/);

  const directTextCalls = [...productionAgentPanel.matchAll(/\.text\(cx\)/g)].map(
    (match) => match.index ?? -1,
  );
  assert.equal(
    directTextCalls.length,
    1,
    "production AgentPanel should only call .text(cx) inside message_editor_text_with_size_limit",
  );
  const helperStart = productionAgentPanel.indexOf("fn message_editor_text_with_size_limit(");
  const helperEnd = helperStart + helper.length;
  assert.ok(
    directTextCalls[0] >= helperStart && directTextCalls[0] < helperEnd,
    "the sole production .text(cx) call should be in the bounded helper",
  );
});

test("draft content checks use bounded text access before deciding editor state", () => {
  for (const name of [
    "draft_has_content",
    "ensure_draft",
    "observe_active_draft_for_empty_editor",
    "try_make_empty_draft_ephemeral",
    "destination_has_meaningful_state",
  ]) {
    const body = functionBody(name);
    assert.match(body, /message_editor_has_trimmed_text_with_size_limit/);
    assert.doesNotMatch(body, /\.text\(cx\)/);
  }
});

test("draft prompt materialization is guarded before snapshotting content blocks", () => {
  const helper = functionBody("message_editor_draft_content_blocks_snapshot_with_size_limit");

  assert.match(helper, /message_editor_within_size_limit/);
  assert.match(helper, /draft_content_blocks_snapshot\(cx\)/);
  assert.ok(
    helper.indexOf("message_editor_within_size_limit") <
      helper.indexOf("draft_content_blocks_snapshot(cx)"),
    "draft text must be size-checked before content block snapshot materialization",
  );

  for (const name of ["draft_prompt_blocks_if_in_memory", "active_initial_content"]) {
    const body = functionBody(name);
    assert.match(body, /message_editor_draft_content_blocks_snapshot_with_size_limit/);
    assert.doesNotMatch(body, /\.draft_content_blocks_snapshot\(cx\)/);
  }
});

test("draft label text and terminal title commits use bounded accessors", () => {
  const editorText = functionBody("editor_text_if_in_memory");
  assert.match(editorText, /message_editor_text_with_size_limit/);
  assert.doesNotMatch(editorText, /\.text\(cx\)/);

  const titleCommit = functionBody("handle_terminal_title_editor_event");
  assert.match(titleCommit, /title_editor_text_with_size_limit/);
  assert.doesNotMatch(titleCommit, /\.text\(cx\)/);
  assert.ok(
    titleCommit.indexOf("title_editor_text_with_size_limit") < titleCommit.indexOf(".trim()"),
    "terminal title editor text must be bounded before trimming",
  );
});

test("native thread clipboard paths continue to use bounded clipboard access", () => {
  const loadThread = functionBody("load_thread_from_clipboard");
  assert.match(loadThread, /clipboard_text_with_size_limit/);
  assert.doesNotMatch(loadThread, /clipboard\.text\(\)/);
});
