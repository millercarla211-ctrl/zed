import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX launch workspace UI stays split by rail ownership", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const expectedModules = [
    "crates/agent_ui/src/dx_launch_workspace/agents.rs",
    "crates/agent_ui/src/dx_launch_workspace/binary_cache.rs",
    "crates/agent_ui/src/dx_launch_workspace/binary_cache_labels.rs",
    "crates/agent_ui/src/dx_launch_workspace/check.rs",
    "crates/agent_ui/src/dx_launch_workspace/check_labels.rs",
    "crates/agent_ui/src/dx_launch_workspace/contracts.rs",
    "crates/agent_ui/src/dx_launch_workspace/launch_status.rs",
    "crates/agent_ui/src/dx_launch_workspace/launch_status_labels.rs",
    "crates/agent_ui/src/dx_launch_workspace/list_labels.rs",
    "crates/agent_ui/src/dx_launch_workspace/proof.rs",
    "crates/agent_ui/src/dx_launch_workspace/proof_labels.rs",
    "crates/agent_ui/src/dx_launch_workspace/readiness.rs",
    "crates/agent_ui/src/dx_launch_workspace/sources.rs",
    "crates/agent_ui/src/dx_launch_workspace/tool_history.rs",
  ];

  for (const module of expectedModules) {
    assert.ok(existsSync(module), `expected focused launch workspace module ${module}`);
  }

  assert.match(parent, /^mod agents;$/m);
  assert.match(parent, /^mod binary_cache;$/m);
  assert.match(parent, /^mod binary_cache_labels;$/m);
  assert.match(parent, /^mod check;$/m);
  assert.match(parent, /^mod check_labels;$/m);
  assert.match(parent, /^mod contracts;$/m);
  assert.match(parent, /^mod launch_status;$/m);
  assert.match(parent, /^mod launch_status_labels;$/m);
  assert.match(parent, /^mod list_labels;$/m);
  assert.match(parent, /^mod proof;$/m);
  assert.match(parent, /^mod proof_labels;$/m);
  assert.match(parent, /^mod readiness;$/m);
  assert.match(parent, /^mod sources;$/m);
  assert.match(parent, /^mod tool_history;$/m);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace.rs") < 1000,
    "dx_launch_workspace.rs should stay a coordinator instead of owning every rail",
  );
});

test("DX launch workspace delegates Launch Handoff rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const contracts = read("crates/agent_ui/src/dx_launch_workspace/contracts.rs");

  assert.match(parent, /contracts::launch_contract_state/);
  assert.doesNotMatch(parent, /fn launch_contract_state/);
  assert.match(contracts, /pub\(super\) fn launch_contract_state/);
  assert.match(contracts, /DxLaunchContractSnapshot/);
  assert.match(contracts, /dx-launch-contract-fanout-review/);
  assert.match(contracts, /use super::\{bounded_items, metric_row, muted_card, signal_row\}/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/contracts.rs") < 150);
});

test("DX launch workspace delegates Launch Status rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const launchStatus = read("crates/agent_ui/src/dx_launch_workspace/launch_status.rs");
  const launchStatusLabels = read(
    "crates/agent_ui/src/dx_launch_workspace/launch_status_labels.rs",
  );

  assert.match(parent, /launch_status::launch_status_state/);
  assert.doesNotMatch(parent, /fn launch_status_state/);
  assert.match(launchStatus, /pub\(super\) fn launch_status_state/);
  assert.match(launchStatus, /DxLaunchStatusSnapshot/);
  assert.match(launchStatus, /fn launch_status_warning/);
  assert.match(launchStatus, /dx-launch-status-redaction-review/);
  assert.match(launchStatus, /use super::launch_status_labels::\{/);
  assert.match(launchStatus, /use super::\{metric_row, muted_card, signal_row, yes_no\}/);
  assert.doesNotMatch(launchStatus, /snapshot\.operator_summary\.clone\(\)/);
  assert.doesNotMatch(launchStatus, /snapshot\.redaction_summary\.is_empty\(\)/);
  assert.match(launchStatusLabels, /pub\(crate\) fn launch_status_summary_label/);
  assert.match(launchStatusLabels, /pub\(crate\) fn launch_status_next_action_label/);
  assert.match(launchStatusLabels, /pub\(crate\) fn launch_status_command_label/);
  assert.match(launchStatusLabels, /pub\(crate\) fn launch_status_optional_summary/);
  assert.match(launchStatusLabels, /launch_status_labels_trim_nonblank_receipt_text/);
  assert.match(launchStatusLabels, /launch_status_labels_fall_back_for_blank_receipt_text/);
  assert.match(launchStatusLabels, /launch_status_optional_summary_ignores_blank_text/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/launch_status.rs") < 170);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/launch_status_labels.rs") < 110);
});

test("DX launch workspace delegates Launch Gate rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const readiness = read("crates/agent_ui/src/dx_launch_workspace/readiness.rs");

  assert.match(parent, /readiness::launch_readiness_state/);
  assert.doesNotMatch(parent, /fn launch_readiness_state/);
  assert.match(readiness, /pub\(super\) fn launch_readiness_state/);
  assert.match(readiness, /DxLaunchReadinessSnapshot/);
  assert.match(readiness, /dx-launch-readiness-fanout-review/);
  assert.match(readiness, /snapshot\.examples\.iter\(\)\.take\(3\)/);
  assert.match(readiness, /use super::\{bounded_items, metric_row, muted_card, signal_row\}/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/readiness.rs") < 150);
});

test("DX launch workspace delegates Binary Cache rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const binaryCache = read("crates/agent_ui/src/dx_launch_workspace/binary_cache.rs");
  const binaryCacheLabels = read(
    "crates/agent_ui/src/dx_launch_workspace/binary_cache_labels.rs",
  );

  assert.match(parent, /binary_cache::binary_cache_state/);
  assert.doesNotMatch(parent, /fn binary_cache_state/);
  assert.doesNotMatch(parent, /fn binary_cache_row/);
  assert.match(binaryCache, /use super::binary_cache_labels::\{/);
  assert.match(binaryCache, /pub\(super\) fn binary_cache_state/);
  assert.match(binaryCache, /fn binary_cache_row/);
  assert.match(binaryCache, /DxBinaryCacheSnapshot/);
  assert.match(binaryCache, /DxBinaryCacheRow/);
  assert.match(binaryCache, /dx-binary-cache-row-\{ix\}/);
  assert.match(binaryCacheLabels, /pub\(crate\) fn binary_cache_summary_label/);
  assert.match(binaryCacheLabels, /pub\(crate\) fn binary_cache_next_action_label/);
  assert.match(binaryCacheLabels, /pub\(crate\) fn binary_cache_row_detail_label/);
  assert.match(binaryCacheLabels, /pub\(crate\) fn binary_cache_row_path_label/);
  assert.match(binaryCacheLabels, /labels_trim_nonblank_receipt_text/);
  assert.match(binaryCacheLabels, /row_labels_fall_back_for_blank_receipt_fields/);
  assert.match(binaryCacheLabels, /labels_preserve_nonblank_row_fields/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/binary_cache.rs") < 90);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/binary_cache_labels.rs") < 100);
});

test("DX launch workspace delegates agents and source rails", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const agents = read("crates/agent_ui/src/dx_launch_workspace/agents.rs");
  const agentProviderLabels = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/provider_labels.rs",
  );
  const agentProviders = read("crates/agent_ui/src/dx_launch_workspace/agents/providers.rs");
  const sources = read("crates/agent_ui/src/dx_launch_workspace/sources.rs");

  assert.match(parent, /agents::dx_agent_bridge_state/);
  assert.match(parent, /sources::source_set_stack/);
  assert.doesNotMatch(parent, /fn dx_agent_bridge_state/);
  assert.doesNotMatch(parent, /fn source_set_stack/);
  assert.match(agents, /pub\(super\) fn dx_agent_bridge_state/);
  assert.match(agents, /^mod provider_labels;$/m);
  assert.match(agents, /^mod providers;$/m);
  assert.match(agents, /pub\(super\) use providers::dx_agent_provider_state/);
  assert.doesNotMatch(agents, /fn dx_agent_provider_row/);
  assert.doesNotMatch(agents, /fn dx_agent_model_row/);
  assert.match(agentProviders, /pub\(in super::super\) fn dx_agent_provider_state/);
  assert.match(agentProviders, /fn dx_agent_provider_row/);
  assert.match(agentProviders, /fn dx_agent_model_row/);
  assert.match(agentProviders, /DxAgentProvider/);
  assert.match(agentProviders, /DxAgentModel/);
  assert.match(agentProviders, /use super::provider_labels::\{/);
  assert.doesNotMatch(agentProviders, /provider\.compatibility\.join/);
  assert.doesNotMatch(agentProviders, /model\.compatibility\.join/);
  assert.match(agentProviderLabels, /pub\(crate\) fn provider_state_label/);
  assert.match(agentProviderLabels, /pub\(crate\) fn model_state_label/);
  assert.match(agentProviderLabels, /pub\(crate\) fn provider_detail_label/);
  assert.match(agentProviderLabels, /pub\(crate\) fn model_detail_label/);
  assert.match(agentProviderLabels, /provider_detail_label_trims_blank_compatibility/);
  assert.match(agentProviderLabels, /model_detail_label_falls_back_for_blank_ids/);
  assert.match(agentProviderLabels, /detail_labels_disclose_compatibility_overflow/);
  assert.match(sources, /pub\(super\) fn source_set_stack/);
  assert.match(sources, /pub\(super\) fn receipt_source_state/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents.rs") < 900);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/provider_labels.rs") < 150);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/providers.rs") < 160);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/sources.rs") < 420);
});

test("DX launch workspace delegates bounded list labels", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const listLabels = read("crates/agent_ui/src/dx_launch_workspace/list_labels.rs");

  assert.match(parent, /use (?:self::)?list_labels::\{bounded_items, yes_no\}/);
  assert.doesNotMatch(parent, /fn bounded_items/);
  assert.doesNotMatch(parent, /fn yes_no/);
  assert.match(listLabels, /pub\(crate\) fn bounded_items/);
  assert.match(listLabels, /pub\(crate\) fn yes_no/);
  assert.match(listLabels, /bounded_items_ignores_blank_values/);
  assert.match(listLabels, /bounded_items_counts_overflow_after_blank_values_are_removed/);
  assert.match(listLabels, /yes_no_labels_boolean_values/);
  assert.match(listLabels, /filter\(\|value\| !value\.trim\(\)\.is_empty\(\)\)/);
  assert.match(listLabels, /map\(\|value\| value\.trim\(\)\)/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/list_labels.rs") < 100);
});

test("DX launch workspace delegates Check rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const check = read("crates/agent_ui/src/dx_launch_workspace/check.rs");
  const labels = read("crates/agent_ui/src/dx_launch_workspace/check_labels.rs");

  assert.match(parent, /check::check_score_state/);
  assert.doesNotMatch(parent, /fn check_score_state/);
  assert.doesNotMatch(parent, /fn check_outcome_label/);
  assert.match(check, /use super::check_labels::\{/);
  assert.match(check, /pub\(super\) fn check_score_state/);
  assert.doesNotMatch(check, /fn check_outcome_label/);
  assert.doesNotMatch(check, /fn checked_paths_label/);
  assert.doesNotMatch(check, /fn skipped_checks_label/);
  assert.match(labels, /pub\(crate\) fn check_outcome_label/);
  assert.match(labels, /pub\(crate\) fn checked_paths_label/);
  assert.match(labels, /pub\(crate\) fn skipped_checks_label/);
  assert.match(labels, /pub\(crate\) fn last_run_label_with_generated_at/);
  assert.match(labels, /fn nonblank_count/);
  assert.match(labels, /filter\(\|value\| !value\.trim\(\)\.is_empty\(\)\)/);
  assert.match(labels, /last_run_label_uses_generated_timestamp_when_label_is_blank/);
  assert.match(labels, /last_run_label_trims_nonblank_receipt_labels/);
  assert.match(labels, /Last run Unix ms: \{generated_at\}/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/check.rs") < 190);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/check_labels.rs") < 170);
});

test("DX launch workspace delegates Tool History rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const toolHistory = read("crates/agent_ui/src/dx_launch_workspace/tool_history.rs");

  assert.match(parent, /tool_history::tool_history_state/);
  assert.doesNotMatch(parent, /fn tool_history_state/);
  assert.doesNotMatch(parent, /fn tool_history_bucket/);
  assert.doesNotMatch(parent, /fn tool_history_summary_row/);
  assert.match(toolHistory, /pub\(super\) fn tool_history_state/);
  assert.match(toolHistory, /fn tool_history_bucket/);
  assert.match(toolHistory, /fn tool_history_summary_row/);
  assert.match(toolHistory, /DxToolHistoryReceiptSummary/);
  assert.match(toolHistory, /dx-tool-history-\{ix\}/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/tool_history.rs") < 150);
});

test("DX launch workspace delegates Proof rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const proof = read("crates/agent_ui/src/dx_launch_workspace/proof.rs");
  const proofLabels = read("crates/agent_ui/src/dx_launch_workspace/proof_labels.rs");

  assert.match(parent, /proof::proof_freshness_state/);
  assert.match(parent, /proof::runtime_proof_status_state/);
  assert.doesNotMatch(parent, /fn proof_freshness_state/);
  assert.doesNotMatch(parent, /fn runtime_proof_status_state/);
  assert.doesNotMatch(parent, /fn runtime_proof_plan_row/);
  assert.doesNotMatch(parent, /fn runtime_proof_receipt_row/);
  assert.match(proof, /pub\(super\) fn proof_freshness_state/);
  assert.match(proof, /pub\(super\) fn runtime_proof_status_state/);
  assert.match(proof, /fn proof_freshness_bucket_row/);
  assert.match(proof, /fn runtime_proof_plan_row/);
  assert.match(proof, /fn runtime_proof_receipt_row/);
  assert.match(proof, /DxRuntimeProofPlanSummary/);
  assert.match(proof, /DxRuntimeProofReceiptSummary/);
  assert.match(proof, /dx-runtime-proof-latest-plan/);
  assert.match(proof, /use super::proof_labels::\{/);
  assert.doesNotMatch(proof, /fn runtime_proof_plan_evidence_detail/);
  assert.doesNotMatch(proof, /fn runtime_proof_plan_requirements/);
  assert.match(proofLabels, /pub\(crate\) fn runtime_proof_evidence_detail/);
  assert.match(proofLabels, /pub\(crate\) fn runtime_proof_requirements_label/);
  assert.match(proofLabels, /pub\(crate\) fn runtime_proof_receipt_state_label/);
  assert.match(proofLabels, /runtime_proof_evidence_detail_ignores_blank_examples/);
  assert.match(proofLabels, /runtime_proof_receipt_state_label_handles_blank_validation_status/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/proof.rs") < 340);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/proof_labels.rs") < 120);
});
