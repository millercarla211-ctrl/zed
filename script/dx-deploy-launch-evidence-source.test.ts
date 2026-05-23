import test from "node:test";
import assert from "node:assert/strict";

import { read } from "./dx-deploy-source-guard.ts";

test("launch gate surfaces dx-check outcome counts and duration", () => {
  const agentUi = read("crates/agent_ui/src/agent_ui.rs");
  const outcome = read("crates/agent_ui/src/dx_deploy_launch_outcome.rs");
  const reader = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");
  const rail = read("crates/agent_ui/src/dx_deploy_launch_gate_rail.rs");
  const prompts = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(agentUi, /^mod dx_deploy_launch_outcome;$/m);
  assert.match(outcome, /pub\(crate\) struct DxDeployLaunchOutcome/);
  assert.match(outcome, /pub pass_count: Option<usize>/);
  assert.match(outcome, /pub fail_count: Option<usize>/);
  assert.match(outcome, /pub warn_count: Option<usize>/);
  assert.match(outcome, /pub skipped_count: Option<usize>/);
  assert.match(outcome, /pub duration_ms: Option<usize>/);
  assert.match(outcome, /pub skipped_expensive_checks: Vec<String>/);
  assert.match(outcome, /pub\(crate\) fn launch_outcome\(/);
  assert.match(outcome, /pass_count: usize_field\(receipt, "pass_count"\)/);
  assert.match(outcome, /skipped_expensive_checks: string_array\(receipt, "skipped_expensive_checks", 4\)/);
  assert.match(outcome, /pub\(crate\) fn launch_outcome_summary/);
  assert.match(outcome, /pub\(crate\) fn launch_duration_label/);
  assert.match(outcome, /pub\(crate\) fn launch_outcome_prompt/);
  assert.match(outcome, /pub\(crate\) fn skipped_checks_prompt/);
  assert.match(reader, /use crate::dx_deploy_launch_outcome::\{DxDeployLaunchOutcome, launch_outcome\};/);
  assert.match(reader, /pub outcome: DxDeployLaunchOutcome/);
  assert.match(reader, /outcome: launch_outcome\(&receipt\)/);
  assert.doesNotMatch(reader, /pub pass_count: Option<usize>/);
  assert.match(rail, /launch_outcome_summary\(&snapshot\.outcome\)/);
  assert.match(rail, /launch_duration_label\(&snapshot\.outcome\)/);
  assert.match(rail, /skipped_checks_prompt\(&snapshot\.outcome\)/);
  assert.match(rail, /metric_row\("Outcome"/);
  assert.match(rail, /metric_row\("Duration"/);
  assert.match(rail, /metric_row\("Skipped"/);
  assert.match(prompts, /launch_outcome=/);
  assert.match(prompts, /duration_ms=/);
  assert.match(prompts, /skipped_checks=/);
  assert.match(prompts, /launch_outcome_prompt\(&gate\.outcome\)/);
  assert.match(prompts, /skipped_checks_prompt\(&gate\.outcome\)/);
});

test("launch gate surfaces dx-check scope and checked paths", () => {
  const agentUi = read("crates/agent_ui/src/agent_ui.rs");
  const scope = read("crates/agent_ui/src/dx_deploy_launch_scope.rs");
  const reader = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");
  const rail = read("crates/agent_ui/src/dx_deploy_launch_gate_rail.rs");
  const prompts = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(agentUi, /^mod dx_deploy_launch_scope;$/m);
  assert.match(scope, /pub\(crate\) struct DxDeployLaunchScope/);
  assert.match(scope, /pub weight_profile: Option<String>/);
  assert.match(scope, /pub active_profile: Option<String>/);
  assert.match(scope, /pub scoring_status: Option<String>/);
  assert.match(scope, /pub scoring_config_present: Option<bool>/);
  assert.match(scope, /pub checked_paths: Vec<String>/);
  assert.match(scope, /pub next_action: Option<String>/);
  assert.match(scope, /pub\(crate\) fn launch_scope\(receipt: &Value\)/);
  assert.match(scope, /string_field\(receipt, "weight_profile"\)/);
  assert.match(scope, /string_field\(scoring_config, "active_profile"\)/);
  assert.match(scope, /bool_field\(scoring_config, "config_present"\)/);
  assert.match(scope, /string_array\(receipt, "checked_paths", 4\)/);
  assert.match(scope, /pub\(crate\) fn launch_scope_summary/);
  assert.match(scope, /pub\(crate\) fn checked_paths_prompt/);
  assert.match(scope, /pub\(crate\) fn launch_scope_prompt/);
  assert.match(reader, /use crate::dx_deploy_launch_scope::\{DxDeployLaunchScope, launch_scope\};/);
  assert.match(reader, /pub scope: DxDeployLaunchScope/);
  assert.match(reader, /scope: launch_scope\(&receipt\)/);
  assert.match(rail, /launch_scope_summary\(&snapshot\.scope\)/);
  assert.match(rail, /checked_paths_prompt\(&snapshot\.scope\)/);
  assert.match(rail, /metric_row\("Scope"/);
  assert.match(rail, /metric_row\("Checked"/);
  assert.match(prompts, /launch_scope=/);
  assert.match(prompts, /checked_paths=/);
  assert.match(prompts, /launch_scope_prompt\(&gate\.scope\)/);
  assert.match(prompts, /checked_paths_prompt\(&gate\.scope\)/);
});

test("launch gate surfaces dx-check bucket scores", () => {
  const agentUi = read("crates/agent_ui/src/agent_ui.rs");
  const buckets = read("crates/agent_ui/src/dx_deploy_launch_buckets.rs");
  const reader = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");
  const rail = read("crates/agent_ui/src/dx_deploy_launch_gate_rail.rs");
  const prompts = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(agentUi, /^mod dx_deploy_launch_buckets;$/m);
  assert.match(buckets, /pub\(crate\) struct DxDeployLaunchBucket/);
  assert.match(buckets, /pub id: Option<String>/);
  assert.match(buckets, /pub label: String/);
  assert.match(buckets, /pub status: Option<String>/);
  assert.match(buckets, /pub score: Option<usize>/);
  assert.match(buckets, /pub max_score: Option<usize>/);
  assert.match(buckets, /pub estimated: Option<bool>/);
  assert.match(buckets, /pub summary: Option<String>/);
  assert.match(buckets, /pub\(crate\) fn launch_buckets\(receipt: &Value\)/);
  assert.match(buckets, /receipt\s*\.get\("bucket_scores"\)/);
  assert.match(buckets, /take\(5\)/);
  assert.match(buckets, /string_field\(row, "id"\)/);
  assert.match(buckets, /usize_field\(row, "score"\)/);
  assert.match(buckets, /bool_field\(row, "estimated"\)/);
  assert.match(buckets, /pub\(crate\) fn launch_bucket_summary_rows/);
  assert.match(buckets, /pub\(crate\) fn launch_buckets_prompt/);
  assert.match(reader, /use crate::dx_deploy_launch_buckets::\{DxDeployLaunchBucket, launch_buckets\};/);
  assert.match(reader, /pub buckets: Vec<DxDeployLaunchBucket>/);
  assert.match(reader, /buckets: launch_buckets\(&receipt\)/);
  assert.match(rail, /launch_bucket_summary_rows\(&snapshot\.buckets\)/);
  assert.match(rail, /metric_row\("Bucket"/);
  assert.match(prompts, /launch_buckets=/);
  assert.match(prompts, /launch_buckets_prompt\(&gate\.buckets\)/);
});

test("launch gate surfaces source runtime and launch approval evidence", () => {
  const agentUi = read("crates/agent_ui/src/agent_ui.rs");
  const approval = read("crates/agent_ui/src/dx_deploy_launch_approval_evidence.rs");
  const reader = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");
  const rail = read("crates/agent_ui/src/dx_deploy_launch_gate_rail.rs");
  const prompts = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(agentUi, /^mod dx_deploy_launch_approval_evidence;$/m);
  assert.match(approval, /pub\(crate\) struct DxDeployLaunchApprovalEvidence/);
  assert.match(approval, /pub source: Vec<String>/);
  assert.match(approval, /pub runtime: Vec<String>/);
  assert.match(approval, /pub launch: Vec<String>/);
  assert.match(approval, /pub\(crate\) fn launch_approval_evidence\(receipt: &Value\)/);
  assert.match(approval, /nested_string_array\(receipt, "source_ready", "evidence", 4\)/);
  assert.match(approval, /nested_string_array\(receipt, "runtime_approved", "evidence", 4\)/);
  assert.match(approval, /nested_string_array\(receipt, "launch_approved", "evidence", 6\)/);
  assert.match(approval, /pub\(crate\) fn approval_evidence_rows/);
  assert.match(approval, /pub\(crate\) fn approval_evidence_prompt/);
  assert.match(
    reader,
    /use crate::dx_deploy_launch_approval_evidence::\{\s*DxDeployLaunchApprovalEvidence,\s*launch_approval_evidence,\s*\};/,
  );
  assert.match(reader, /pub approval_evidence: DxDeployLaunchApprovalEvidence/);
  assert.match(reader, /approval_evidence: launch_approval_evidence\(&receipt\)/);
  assert.match(rail, /approval_evidence_rows\(&snapshot\.approval_evidence\)/);
  assert.match(rail, /metric_row\("Evidence"/);
  assert.match(prompts, /approval_evidence=/);
  assert.match(prompts, /approval_evidence_prompt\(&gate\.approval_evidence\)/);
});

test("launch gate exposes dx-check evidence-source rows", () => {
  const parser = read("crates/agent_ui/src/dx_deploy_launch_evidence.rs");
  const reader = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");
  const rail = read("crates/agent_ui/src/dx_deploy_launch_gate_rail.rs");
  const evidenceRail = read("crates/agent_ui/src/dx_deploy_launch_evidence_rail.rs");
  const prompts = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(parser, /DxDeployLaunchEvidenceSource/);
  assert.match(parser, /DxDeployLaunchChain/);
  assert.match(parser, /pub id: Option<String>/);
  assert.match(parser, /pub generated_at_unix_ms: Option<usize>/);
  assert.match(parser, /launch_evidence_sources/);
  assert.match(parser, /launch_chain/);
  assert.match(parser, /required_source_count/);
  assert.match(
    parser,
    /pub receipt_path: Option<String>,\s*pub generated_at_unix_ms: Option<usize>,\s*pub blocker_count: usize,\s*pub blockers: Vec<String>,\s*pub next_action: Option<String>,/s,
  );
  assert.match(parser, /blocker_count/);
  assert.match(parser, /generated_at_unix_ms/);
  assert.match(parser, /blockers: string_array\(row, "blockers", 3\)/);
  assert.match(parser, /string_array\(chain, "blockers", 5\)/);
  assert.match(reader, /launch_evidence_sources\(&receipt\)/);
  assert.match(reader, /launch_chain\(&receipt\)/);
  assert.match(rail, /deploy_launch_evidence_state/);
  assert.match(evidenceRail, /sources\.iter\(\)\.take\(5\)/);
  assert.match(evidenceRail, /source\.id/);
  assert.match(evidenceRail, /source\.generated_at_unix_ms/);
  assert.match(evidenceRail, /launch_evidence_summary/);
  assert.match(evidenceRail, /launch_evidence_row/);
  assert.match(evidenceRail, /launch_chain_summary/);
  assert.match(evidenceRail, /launch_chain_blocker_rows/);
  assert.match(evidenceRail, /chain\.blockers\.iter\(\)\.take\(5\)/);
  assert.match(evidenceRail, /launch_evidence_source_blocker_rows/);
  assert.match(evidenceRail, /source\.blockers\.iter\(\)\.take\(3\)/);
  assert.match(prompts, /deploy_launch_evidence_prompt/);
  assert.match(prompts, /evidence_sources=/);
  assert.match(prompts, /evidence_id=/);
  assert.match(prompts, /source_blockers=/);
  assert.match(prompts, /generated_at_unix_ms=/);
  assert.match(prompts, /chain_blockers=/);
});
