import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX launch prompts keep launch context formatting in a focused module", () => {
  const parent = read("crates/agent_ui/src/dx_launch_prompts.rs");
  const context = "crates/agent_ui/src/dx_launch_prompts/context.rs";

  assert.ok(existsSync(context), `expected focused launch context module ${context}`);
  assert.match(parent, /^mod context;$/m);
  assert.match(parent, /^use context::\{$/m);
  assert.match(parent, /launch_receipt_review_prompt_context,/);
  assert.doesNotMatch(parent, /fn launch_status_prompt_context/);
  assert.doesNotMatch(parent, /fn launch_contract_prompt_context/);
  assert.doesNotMatch(parent, /fn launch_receipt_review_prompt_context/);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_prompts.rs") < 230,
    "dx_launch_prompts.rs should stay a prompt coordinator",
  );
});

test("DX launch context module owns receipt and audit summaries", () => {
  const context = read("crates/agent_ui/src/dx_launch_prompts/context.rs");

  assert.match(context, /pub\(super\) fn launch_status_prompt_context/);
  assert.match(context, /pub\(super\) fn launch_contract_prompt_context/);
  assert.match(context, /pub\(super\) fn launch_readiness_prompt_context/);
  assert.match(context, /pub\(super\) fn launch_audit_prompt_context/);
  assert.match(context, /pub\(super\) fn launch_www_evidence_prompt_context/);
  assert.match(context, /pub\(super\) fn launch_source_audit_prompt_context/);
  assert.match(context, /pub\(super\) fn launch_receipt_review_prompt_context/);
  assert.match(context, /pub\(super\) fn bounded_join/);
  assert.match(context, /remaining_count/);
  assert.match(context, /\+\{\} more/);
  assert.match(context, /missing launch receipt directory/);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_prompts/context.rs") < 380,
    "launch context module should stay focused",
  );
});

test("bounded_join compacts and caps each item before preserving overflow count", () => {
  const context = read("crates/agent_ui/src/dx_launch_prompts/context.rs");

  assert.match(context, /const BOUNDED_JOIN_ITEM_CHAR_LIMIT: usize = 240;/);
  assert.match(context, /const BOUNDED_JOIN_TRUNCATION_MARKER: &str = "\.\.\.";/);
  assert.match(context, /fn compact_bounded_join_item\(value: &str\) -> String/);
  assert.match(
    context,
    /value\s*\.split_whitespace\(\)\s*\.collect::<Vec<_>>\(\)\s*\.join\(" "\)/s,
  );
  assert.match(context, /BOUNDED_JOIN_ITEM_CHAR_LIMIT\.saturating_sub/);
  assert.match(context, /\.chars\(\)\s*\.take\(keep\)\s*\.collect::<String>\(\)/s);
  assert.match(
    context,
    /\.iter\(\)\s*\.take\(limit\)\s*\.map\(\|value\| compact_bounded_join_item\(value\)\)\s*\.collect::<Vec<_>>\(\)\s*\.join\(", "\)/s,
  );
  assert.match(context, /if values\.is_empty\(\)\s*\{\s*return empty\.to_string\(\);/s);
  assert.match(context, /let remaining_count = values\.len\(\)\.saturating_sub\(limit\);/);
  assert.match(context, /rendered\.push_str\(&format!\(", \+\{\} more", remaining_count\)\);/);
});

test("DX launch prompts keep Forge proof wording in a focused module", () => {
  const parent = read("crates/agent_ui/src/dx_launch_prompts.rs");
  const forge = "crates/agent_ui/src/dx_launch_prompts/forge.rs";

  assert.ok(existsSync(forge), `expected focused Forge prompt module ${forge}`);
  assert.match(parent, /^mod forge;$/m);
  assert.match(parent, /^pub\(crate\) use forge::\{forge_proof_prompt, restore_approval_prompt\};$/m);
  assert.doesNotMatch(parent, /fn forge_proof_prompt/);
  assert.doesNotMatch(parent, /fn restore_approval_prompt/);
  assert.doesNotMatch(parent, /fn forge_history_summary_prompt/);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_prompts.rs") < 560,
    "dx_launch_prompts.rs should stay a prompt coordinator",
  );
});

test("DX launch Forge prompt module owns restore proof context", () => {
  const forge = read("crates/agent_ui/src/dx_launch_prompts/forge.rs");

  assert.match(forge, /pub\(crate\) fn forge_proof_prompt/);
  assert.match(forge, /pub\(crate\) fn restore_approval_prompt/);
  assert.match(forge, /pub\(super\) fn forge_history_prompt_context/);
  assert.match(forge, /fn forge_history_summary_prompt/);
  assert.match(forge, /inspect_dx_forge_history/);
  assert.match(forge, /Do not mutate target paths/);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_prompts/forge.rs") < 130,
    "Forge prompt module should stay focused",
  );
});

test("DX launch prompts keep source-action wording in a focused module", () => {
  const parent = read("crates/agent_ui/src/dx_launch_prompts.rs");
  const source = "crates/agent_ui/src/dx_launch_prompts/source.rs";

  assert.ok(existsSync(source), `expected focused source prompt module ${source}`);
  assert.match(parent, /^mod source;$/m);
  assert.match(parent, /^pub\(crate\) use source::\{$/m);
  assert.match(parent, /source_receipt_review_prompt,/);
  assert.doesNotMatch(parent, /fn source_action_prompt/);
  assert.doesNotMatch(parent, /fn source_receipt_review_prompt/);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_prompts.rs") < 620,
    "dx_launch_prompts.rs should stay a prompt coordinator",
  );
});

test("DX launch source prompt module owns attachment prompt text", () => {
  const source = read("crates/agent_ui/src/dx_launch_prompts/source.rs");

  assert.match(source, /pub\(crate\) fn source_action_icon/);
  assert.match(source, /pub\(crate\) fn source_action_title/);
  assert.match(source, /pub\(crate\) fn source_action_label/);
  assert.match(source, /pub\(crate\) fn source_receipt_review_prompt/);
  assert.match(source, /pub\(crate\) fn source_action_prompt/);
  assert.match(source, /prepare_dx_source_attachment/);
  assert.match(source, /Do not run builds, local servers/);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_prompts/source.rs") < 140,
    "source prompt module should stay focused",
  );
});

test("DX launch prompts keep runtime-proof wording in a focused module", () => {
  const parent = read("crates/agent_ui/src/dx_launch_prompts.rs");
  const runtimeProof = "crates/agent_ui/src/dx_launch_prompts/runtime_proof.rs";

  assert.ok(existsSync(runtimeProof), `expected focused runtime proof prompt module ${runtimeProof}`);
  assert.match(parent, /^mod runtime_proof;$/m);
  assert.match(parent, /^pub\(crate\) use runtime_proof::\{$/m);
  assert.match(parent, /runtime_proof_evidence_template_prompt,/);
  assert.doesNotMatch(parent, /fn runtime_proof_status_prompt_context/);
  assert.doesNotMatch(parent, /fn runtime_proof_evidence_template\(/);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_prompts.rs") < 820,
    "dx_launch_prompts.rs should stay a prompt coordinator",
  );
});

test("DX launch runtime-proof module owns guarded proof handoff text", () => {
  const runtimeProof = read("crates/agent_ui/src/dx_launch_prompts/runtime_proof.rs");

  assert.match(runtimeProof, /pub\(crate\) fn runtime_proof_prompt/);
  assert.match(runtimeProof, /pub\(crate\) fn runtime_proof_import_prompt/);
  assert.match(runtimeProof, /pub\(crate\) fn runtime_proof_evidence_template_prompt/);
  assert.match(runtimeProof, /fn runtime_proof_status_prompt_context/);
  assert.match(runtimeProof, /fn runtime_proof_plan_requirements/);
  assert.match(runtimeProof, /Do not run just run, cargo, builds, local servers/);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_prompts/runtime_proof.rs") < 340,
    "runtime proof prompt module should stay focused",
  );
});
