import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const promptEditor = readFileSync(
  "crates/agent_ui/src/inline_prompt_editor.rs",
  "utf8",
);
const inlineAssistant = readFileSync(
  "crates/agent_ui/src/inline_assistant.rs",
  "utf8",
);

test("inline prompt caps editor text before full prompt materialization", () => {
  assert.match(
    promptEditor,
    /const MAX_INLINE_PROMPT_BYTES: usize = 128 \* 1024;/,
  );
  assert.match(
    promptEditor,
    /snapshot\.display_snapshot\.buffer_snapshot\(\)\.len\(\)\.0/,
  );

  const boundedStart = promptEditor.indexOf("pub fn bounded_prompt");
  const boundedEnd = promptEditor.indexOf("\n    fn paste", boundedStart);
  assert.notEqual(boundedStart, -1, "expected bounded prompt accessor");
  assert.ok(boundedEnd > boundedStart, "expected paste after bounded prompt");

  const boundedPrompt = promptEditor.slice(boundedStart, boundedEnd);
  assert.match(boundedPrompt, /self\.prompt_size_error\(cx\)/);
  assert.match(boundedPrompt, /Ok\(self\.prompt\(cx\)\)/);
  assert.ok(
    boundedPrompt.indexOf("self.prompt_size_error(cx)") <
      boundedPrompt.indexOf("self.prompt(cx)"),
    "prompt size must be checked before cloning the full prompt",
  );

  const editedStart = promptEditor.indexOf("EditorEvent::Edited");
  const editedEnd = promptEditor.indexOf("EditorEvent::Blurred", editedStart);
  assert.notEqual(editedStart, -1, "expected edit-event handler");
  assert.ok(editedEnd > editedStart, "expected blurred branch after edited branch");

  const editedBranch = promptEditor.slice(editedStart, editedEnd);
  assert.match(editedBranch, /inline_prompt_snapshot_size_error\(&snapshot\)/);
  assert.match(editedBranch, /let prompt = snapshot\.text\(\);/);
  assert.ok(
    editedBranch.indexOf("inline_prompt_snapshot_size_error(&snapshot)") <
      editedBranch.indexOf("let prompt = snapshot.text();"),
    "snapshot size must be checked before cloning full text for prompt history",
  );
});

test("inline assistant request construction uses the bounded prompt path", () => {
  const startAssistStart = inlineAssistant.indexOf("pub fn start_assist");
  const startAssistEnd = inlineAssistant.indexOf("\n    pub fn stop_assist", startAssistStart);
  assert.notEqual(startAssistStart, -1, "expected start_assist");
  assert.ok(startAssistEnd > startAssistStart, "expected stop_assist after start_assist");

  const startAssist = inlineAssistant.slice(startAssistStart, startAssistEnd);
  assert.match(startAssist, /prompt_size_error_for_assist_group\(assist_group_id, cx\)/);
  assert.match(startAssist, /self\.unlink_assist_group\(assist_group_id, window, cx\)/);
  assert.ok(
    startAssist.indexOf("prompt_size_error_for_assist_group(assist_group_id, cx)") <
      startAssist.indexOf("self.unlink_assist_group(assist_group_id, window, cx)"),
    "linked prompt groups must be size-checked before unlink clones prompt text",
  );

  assert.match(startAssist, /let user_prompt = match assist\.user_prompt\(cx\)/);
  assert.match(startAssist, /codegen\.start\(model, user_prompt, context_task, cx\)/);
  assert.ok(
    startAssist.indexOf("let user_prompt = match assist.user_prompt(cx)") <
      startAssist.indexOf("codegen.start(model, user_prompt, context_task, cx)"),
    "assistant requests must receive prompt text only from the guarded accessor",
  );

  const userPromptStart = inlineAssistant.indexOf("fn user_prompt(&self");
  const userPromptEnd = inlineAssistant.indexOf("\n    fn mention_set", userPromptStart);
  assert.notEqual(userPromptStart, -1, "expected InlineAssist::user_prompt");
  assert.ok(userPromptEnd > userPromptStart, "expected mention_set after user_prompt");

  const userPrompt = inlineAssistant.slice(userPromptStart, userPromptEnd);
  assert.match(userPrompt, /Result<Option<String>, InlinePromptSizeError>/);
  assert.match(userPrompt, /bounded_prompt\(cx\)\s*\.map\(Some\)/);
  assert.doesNotMatch(userPrompt, /\.prompt\(cx\)/);
});

test("inline prompt history navigation uses checked history lookups", () => {
  const moveUpStart = promptEditor.indexOf("fn move_up(");
  const moveDownStart = promptEditor.indexOf("fn move_down(", moveUpStart);
  const renderButtonsStart = promptEditor.indexOf("\n    fn render_buttons", moveDownStart);
  assert.notEqual(moveUpStart, -1, "expected move_up handler");
  assert.ok(moveDownStart > moveUpStart, "expected move_down after move_up");
  assert.ok(
    renderButtonsStart > moveDownStart,
    "expected render_buttons after move_down",
  );

  const navigationHandlers = {
    move_up: promptEditor.slice(moveUpStart, moveDownStart),
    move_down: promptEditor.slice(moveDownStart, renderButtonsStart),
  };

  for (const [name, handler] of Object.entries(navigationHandlers)) {
    assert.doesNotMatch(
      handler,
      /self\.prompt_history\s*\[[^\]]+\]/,
      `${name} must not directly index prompt_history`,
    );
    assert.match(
      handler,
      /self\.prompt_history\.get\(/,
      `${name} should use checked prompt_history lookups`,
    );
  }
});

test("oversized inline prompt refusal surfaces through existing assistant toast UI", () => {
  const toastStart = inlineAssistant.indexOf("fn show_prompt_size_error");
  const toastEnd = inlineAssistant.indexOf("\n    pub fn start_assist", toastStart);
  assert.notEqual(toastStart, -1, "expected prompt-size error toast helper");
  assert.ok(toastEnd > toastStart, "expected start_assist after toast helper");

  const toastHelper = inlineAssistant.slice(toastStart, toastEnd);
  assert.match(toastHelper, /NotificationId::composite::<InlinePromptSizeLimit>/);
  assert.match(toastHelper, /workspace\.show_toast\(Toast::new\(id, error\.to_string\(\)\), cx\)/);
  assert.match(promptEditor, /Inline prompt is too large/);
});
