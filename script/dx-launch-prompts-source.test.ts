import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

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
