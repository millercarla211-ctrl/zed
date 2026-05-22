import { readFileSync, readdirSync } from "node:fs";
import test from "node:test";
import assert from "node:assert/strict";

const deploySourceDir = "crates/agent_ui/src";
const read = (path) => readFileSync(path, "utf8");
const lineCount = (path) => read(path).split(/\r?\n/).length;

const deploySourceFiles = () =>
  readdirSync(deploySourceDir)
    .filter((name) => name.startsWith("dx_deploy") && name.endsWith(".rs"))
    .sort();

test("agent_ui registers the focused deploy modules", () => {
  const source = read("crates/agent_ui/src/agent_ui.rs");
  const modules = [
    "dx_deploy_capabilities",
    "dx_deploy_check_roots",
    "dx_deploy_gate_rail",
    "dx_deploy_hub_roots",
    "dx_deploy_launch_actions",
    "dx_deploy_launch_action_labels",
    "dx_deploy_launch_actions_rail",
    "dx_deploy_launch_approval_evidence",
    "dx_deploy_launch_buckets",
    "dx_deploy_launch_evidence",
    "dx_deploy_launch_evidence_rail",
    "dx_deploy_launch_gate",
    "dx_deploy_launch_gate_rail",
    "dx_deploy_launch_outcome",
    "dx_deploy_launch_prompts",
    "dx_deploy_launch_score",
    "dx_deploy_launch_scope",
    "dx_deploy_local_files",
    "dx_deploy_matrix_rail",
    "dx_deploy_provider_gate_summary",
    "dx_deploy_prompts",
    "dx_deploy_rail",
    "dx_deploy_rail_ui",
    "dx_deploy_receipt_buckets",
    "dx_deploy_receipt_extract",
    "dx_deploy_receipt_fields",
    "dx_deploy_receipt_files",
    "dx_deploy_receipt_rank",
    "dx_deploy_receipt_roots",
    "dx_deploy_receipt_summary",
    "dx_deploy_root_key",
    "dx_deploy_target_detection",
    "dx_deploy_targets",
  ];

  for (const moduleName of modules) {
    assert.match(
      source,
      new RegExp(`^mod ${moduleName};$`, "m"),
      `${moduleName} should be registered from agent_ui.rs`,
    );
  }
});

test("deploy receipt parsing stays split by receipt ownership", () => {
  const agentUi = read("crates/agent_ui/src/agent_ui.rs");
  const capabilities = read("crates/agent_ui/src/dx_deploy_capabilities.rs");
  const summary = read("crates/agent_ui/src/dx_deploy_receipt_summary.rs");
  const providerGate = read("crates/agent_ui/src/dx_deploy_provider_gate_summary.rs");
  const fields = read("crates/agent_ui/src/dx_deploy_receipt_fields.rs");

  assert.match(agentUi, /^mod dx_deploy_provider_gate_summary;$/m);
  assert.match(agentUi, /^mod dx_deploy_receipt_fields;$/m);
  assert.match(
    capabilities,
    /pub\(crate\) use crate::dx_deploy_provider_gate_summary::\{/,
  );
  assert.match(
    capabilities,
    /use crate::dx_deploy_provider_gate_summary::parse_deploy_provider_gate_receipt;/,
  );
  assert.match(summary, /use crate::dx_deploy_receipt_fields::\{/);
  assert.match(providerGate, /use crate::dx_deploy_receipt_fields::\{/);
  assert.match(providerGate, /pub\(crate\) struct DxDeployProviderGateReceiptSummary/);
  assert.match(providerGate, /pub\(crate\) fn parse_deploy_provider_gate_receipt/);
  assert.match(providerGate, /fn provider_gate_quick_fixes/);
  assert.match(fields, /pub\(crate\) fn string_field/);
  assert.match(fields, /pub\(crate\) fn usize_field/);
  assert.match(fields, /pub\(crate\) fn string_array/);
  assert.doesNotMatch(summary, /struct DxDeployProviderGateReceiptSummary/);
  assert.doesNotMatch(summary, /fn provider_gate_rows/);
  assert.doesNotMatch(summary, /fn provider_gate_quick_fixes/);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_deploy_receipt_summary.rs") < 190,
    "dx_deploy_receipt_summary.rs should keep provider-gate parsing out",
  );
  assert.ok(
    lineCount("crates/agent_ui/src/dx_deploy_provider_gate_summary.rs") < 120,
    "provider gate summary parsing should stay focused",
  );
  assert.ok(
    lineCount("crates/agent_ui/src/dx_deploy_receipt_fields.rs") < 80,
    "shared receipt field helpers should stay tiny",
  );
});

test("deploy prompt ownership stays out of the launch prompt module", () => {
  const agentPanel = read("crates/agent_ui/src/agent_panel.rs");
  const launchPrompts = read("crates/agent_ui/src/dx_launch_prompts.rs");

  assert.match(
    agentPanel,
    /use crate::dx_deploy_prompts::deploy_readiness_prompt;/,
  );
  assert.match(
    launchPrompts,
    /use crate::dx_deploy_prompts::\{deploy_launch_gate_prompt, deploy_receipt_bucket_prompt\};/,
  );
  assert.doesNotMatch(launchPrompts, /fn\s+deploy_readiness_prompt/);
  assert.doesNotMatch(launchPrompts, /fn\s+deploy_capability_matrix_prompt/);
});

test("deploy prompts keep dry-run and launch approval language explicit", () => {
  const source = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(source, /source\/runtime\/launch approval/);
  assert.match(source, /no live deploy should be inferred from dry-run receipts/);
  assert.match(source, /dry-run receipts are not live deploy approval/);
  assert.match(source, /fn deploy_launch_gate_prompt\(/);
});

test("deploy launch prompt details stay in a focused module", () => {
  const prompts = read("crates/agent_ui/src/dx_deploy_prompts.rs");
  const launchPrompts = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(
    prompts,
    /pub\(crate\) use crate::dx_deploy_launch_prompts::deploy_launch_gate_prompt;/,
  );
  assert.match(
    prompts,
    /use crate::dx_deploy_launch_prompts::deploy_launch_evidence_prompt;/,
  );
  assert.match(launchPrompts, /pub\(crate\) fn deploy_launch_gate_prompt/);
  assert.match(launchPrompts, /pub\(crate\) fn deploy_launch_evidence_prompt/);
  assert.match(launchPrompts, /fn launch_actions_prompt/);
  assert.match(launchPrompts, /fn approval_state_label/);
  assert.match(launchPrompts, /status_score=/);
  assert.match(launchPrompts, /launch_actions=/);
  assert.doesNotMatch(prompts, /fn\s+deploy_launch_gate_prompt/);
  assert.doesNotMatch(prompts, /fn\s+launch_actions_prompt/);
  assert.doesNotMatch(prompts, /fn\s+launch_evidence_prompt/);
});

test("launch gate reader prefers launch-specific check receipts", () => {
  const agentUi = read("crates/agent_ui/src/agent_ui.rs");
  const roots = read("crates/agent_ui/src/dx_deploy_check_roots.rs");
  const rootKey = read("crates/agent_ui/src/dx_deploy_root_key.rs");
  const source = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");

  assert.match(agentUi, /^mod dx_deploy_check_roots;$/m);
  assert.match(agentUi, /^mod dx_deploy_root_key;$/m);
  assert.match(roots, /pub\(crate\) struct DxDeployCheckReceiptRoot/);
  assert.match(roots, /pub root_rank: u8/);
  assert.match(roots, /pub\(crate\) fn check_receipt_roots/);
  assert.match(roots, /workspace_roots\.iter\(\)\.take\(4\)/);
  assert.match(roots, /dx_hub_root\(\)/);
  assert.match(roots, /\.join\("www"\)/);
  assert.match(roots, /\.join\("receipts"\)\.join\("check"\)/);
  assert.match(roots, /use crate::dx_deploy_root_key::deploy_root_key;/);
  assert.match(roots, /let path_key = deploy_root_key\(&path\);/);
  assert.match(roots, /deploy_root_key\(&root\.path\) == path_key/);
  assert.match(rootKey, /pub\(crate\) fn deploy_root_key\(path: &Path\) -> String/);
  assert.match(rootKey, /#\[cfg\(windows\)\]/);
  assert.match(rootKey, /#\[cfg\(not\(windows\)\)\]/);
  assert.match(rootKey, /replace\('\/', "\\\\"\)/);
  assert.match(rootKey, /key\.ends_with\('\/'\)/);
  assert.match(rootKey, /to_ascii_lowercase\(\)/);
  assert.match(source, /use crate::dx_deploy_check_roots::check_receipt_roots;/);
  assert.match(source, /for root in check_receipt_roots\(workspace_roots\)/);
  assert.match(source, /\["check-launch-latest\.json", "check-latest\.json"\]/);
  assert.match(source, /file_rank/);
  assert.match(source, /root_rank/);
  assert.doesNotMatch(source, /const DX_HUB_CHECK_RECEIPT_ROOT/);
  assert.doesNotMatch(source, /const DX_WWW_CHECK_RECEIPT_ROOT/);
});

test("deploy capability receipt roots stay in a focused module", () => {
  const agentUi = read("crates/agent_ui/src/agent_ui.rs");
  const roots = read("crates/agent_ui/src/dx_deploy_receipt_roots.rs");
  const hubRoots = read("crates/agent_ui/src/dx_deploy_hub_roots.rs");
  const capabilities = read("crates/agent_ui/src/dx_deploy_capabilities.rs");

  assert.match(agentUi, /^mod dx_deploy_hub_roots;$/m);
  assert.match(agentUi, /^mod dx_deploy_receipt_roots;$/m);
  assert.match(agentUi, /^mod dx_deploy_root_key;$/m);
  assert.match(hubRoots, /DX_HOME_ENV/);
  assert.match(hubRoots, /DX_ROOT_ENV/);
  assert.match(hubRoots, /DX_HUB_ROOT_CANDIDATES/);
  assert.match(hubRoots, /r"D:\\Dx"/);
  assert.match(hubRoots, /r"G:\\Dx"/);
  assert.match(hubRoots, /pub\(crate\) fn deploy_hub_receipt_roots/);
  assert.match(hubRoots, /fn configured_dx_hub_root/);
  assert.match(hubRoots, /std::env::var_os/);
  assert.match(hubRoots, /\.exists\(\)/);
  assert.match(hubRoots, /DxDeployReceiptSourceKind::DxHub/);
  assert.match(hubRoots, /DxDeployReceiptSourceKind::DxCli/);
  assert.match(hubRoots, /DxDeployReceiptSourceKind::DxWww/);
  assert.match(roots, /pub\(crate\) struct DxDeployReceiptRoot/);
  assert.match(roots, /pub path: PathBuf/);
  assert.match(roots, /pub label: String/);
  assert.match(roots, /pub source_kind: DxDeployReceiptSourceKind/);
  assert.match(
    roots,
    /pub\(crate\) fn deploy_receipt_roots\(workspace_roots: &\[PathBuf\]\) -> Vec<DxDeployReceiptRoot>/,
  );
  assert.match(roots, /workspace_roots\.iter\(\)\.take\(4\)/);
  assert.match(roots, /deploy_hub_receipt_roots\(\)/);
  assert.match(roots, /use crate::dx_deploy_root_key::deploy_root_key;/);
  assert.match(roots, /path\.as_os_str\(\)\.is_empty\(\)/);
  assert.match(roots, /let path_key = deploy_root_key\(&path\);/);
  assert.match(roots, /deploy_root_key\(&root\.path\) == path_key/);
  assert.match(
    capabilities,
    /use crate::dx_deploy_receipt_roots::\{DxDeployReceiptRoot, deploy_receipt_roots\};/,
  );
  assert.doesNotMatch(capabilities, /fn deploy_receipt_roots/);
  assert.doesNotMatch(capabilities, /const DX_HUB_DEPLOY_RECEIPT_ROOT/);
  assert.doesNotMatch(roots, /const DX_HUB_DEPLOY_RECEIPT_ROOT/);
});

test("launch gate keeps source-owned blocker provenance", () => {
  const reader = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");
  const rail = read("crates/agent_ui/src/dx_deploy_launch_gate_rail.rs");
  const prompts = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(reader, /pub severity: Option<String>/);
  assert.match(reader, /pub evidence_path: Option<String>/);
  assert.match(reader, /severity: string_field\(row, "severity"\)/);
  assert.match(reader, /evidence_path: string_field\(row, "evidence_path"\)/);
  assert.match(rail, /notice\.severity/);
  assert.match(rail, /notice\.evidence_path/);
  assert.match(rail, /evidence_path/);
  assert.match(prompts, /blocker\.severity/);
  assert.match(prompts, /blocker\.evidence_path/);
  assert.match(prompts, /severity=/);
  assert.match(prompts, /evidence_path=/);
});

test("provider gate surfaces dx-deploy quick-fix risk metadata", () => {
  const summary = read("crates/agent_ui/src/dx_deploy_provider_gate_summary.rs");
  const rail = read("crates/agent_ui/src/dx_deploy_gate_rail.rs");

  assert.match(summary, /pub\(crate\) struct DxDeployProviderGateQuickFix/);
  assert.match(summary, /pub quick_fixes: Vec<DxDeployProviderGateQuickFix>/);
  assert.match(summary, /pub command: String/);
  assert.match(summary, /pub risk_level: String/);
  assert.match(summary, /pub requires_user_approval: bool/);
  assert.match(summary, /pub writes_receipts: bool/);
  assert.match(summary, /let quick_fixes = provider_gate_quick_fixes\(zed\);/);
  assert.match(summary, /quick_fix_count: quick_fixes\.len\(\)/);
  assert.match(summary, /quick_fixes,/);
  assert.match(summary, /fn provider_gate_quick_fixes\(zed: Option<&Value>\) -> Vec<DxDeployProviderGateQuickFix>/);
  assert.match(summary, /string_field\(value, "risk_level"\)/);
  assert.match(summary, /value\s*\.get\("requires_user_approval"\)\s*\.and_then\(Value::as_bool\)/);
  assert.match(summary, /value\s*\.get\("writes_receipts"\)\s*\.and_then\(Value::as_bool\)/);
  assert.match(rail, /receipt\.quick_fixes\.iter\(\)\.take\(3\)/);
  assert.match(rail, /dx-deploy-gate-quick-fix-\{\}/);
  assert.match(rail, /quick_fix\.risk_level/);
  assert.match(rail, /requires approval/);
  assert.match(rail, /writes receipt/);
});

test("launch gate keeps warning provenance visible", () => {
  const reader = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");
  const rail = read("crates/agent_ui/src/dx_deploy_launch_gate_rail.rs");
  const prompts = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(reader, /pub warnings: Vec<DxDeployLaunchGateNotice>/);
  assert.match(reader, /warnings: notice_rows\(zed\.and_then\(\|value\| value\.get\("warnings"\)\)\)/);
  assert.match(rail, /snapshot\.warnings\.iter\(\)\.take\(2\)/);
  assert.match(rail, /dx-deploy-launch-gate-warning-\{ix\}/);
  assert.match(rail, /Launch warning/);
  assert.match(prompts, /launch_warnings_prompt/);
  assert.match(prompts, /launch_warnings=/);
  assert.match(prompts, /warning\.severity/);
  assert.match(prompts, /warning\.evidence_path/);
});

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

test("launch gate normalizes dx-check score to a 100-point deploy status", () => {
  const reader = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");
  const score = read("crates/agent_ui/src/dx_deploy_launch_score.rs");
  const rail = read("crates/agent_ui/src/dx_deploy_launch_gate_rail.rs");
  const prompts = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(reader, /pub score_estimated: Option<bool>/);
  assert.match(reader, /bool_field\(value, "score_estimated"\)/);
  assert.match(reader, /bool_field\(&receipt, "score_estimated"\)/);
  assert.match(score, /pub\(crate\) fn launch_status_score\(/);
  assert.match(score, /pub\(crate\) fn launch_status_score_label\(/);
  assert.match(score, /score\.saturating_mul\(100\) \/ max_score/);
  assert.match(score, /\.min\(100\)/);
  assert.match(score, /filter\(\|max_score\| \*max_score > 0\)/);
  assert.match(score, /score_estimated == Some\(true\)/);
  assert.match(score, /label\.push_str\(" estimated"\)/);
  assert.match(rail, /launch_status_score_label\(snapshot\)/);
  assert.match(rail, /metric_row\("Status score"/);
  assert.match(prompts, /launch_status_score_label\(snapshot\)/);
  assert.match(prompts, /status_score=/);
});

test("launch gate exposes bounded dx-check quick actions with risk metadata", () => {
  const actions = read("crates/agent_ui/src/dx_deploy_launch_actions.rs");
  const actionLabels = read("crates/agent_ui/src/dx_deploy_launch_action_labels.rs");
  const reader = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");
  const rail = read("crates/agent_ui/src/dx_deploy_launch_gate_rail.rs");
  const actionRail = read("crates/agent_ui/src/dx_deploy_launch_actions_rail.rs");
  const prompts = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(actions, /pub\(crate\) struct DxDeployLaunchAction/);
  assert.match(actions, /pub id: Option<String>/);
  assert.match(actions, /pub command: Option<String>/);
  assert.match(actions, /pub risk_level: Option<String>/);
  assert.match(actions, /pub requires_user_approval: Option<bool>/);
  assert.match(actions, /pub writes_receipts: Option<bool>/);
  assert.match(actions, /pub\(crate\) fn launch_actions/);
  assert.match(actions, /take\(5\)/);
  assert.match(reader, /quick_actions: Vec<DxDeployLaunchAction>/);
  assert.match(reader, /pub quick_action_count: usize/);
  assert.match(reader, /launch_actions\(zed\.and_then\(\|value\| value\.get\("quick_fixes"\)\)\)/);
  assert.match(reader, /usize_field\(value, "quick_fix_count"\)/);
  assert.match(
    rail,
    /deploy_launch_action_state\(\s*&snapshot\.quick_actions,\s*snapshot\.quick_action_count,/,
  );
  assert.match(actionRail, /total_count: usize/);
  assert.match(actionRail, /shown of .*available/);
  assert.match(actionRail, /metric_row\(\s*"Actions"/);
  assert.match(actionRail, /use crate::dx_deploy_launch_action_labels::launch_action_detail_parts/);
  assert.match(actionRail, /launch_action_detail_parts\(/);
  assert.doesNotMatch(actionRail, /let mut detail = Vec::new\(\);/);
  assert.match(actionLabels, /pub\(crate\) fn launch_action_detail_parts/);
  assert.match(actionLabels, /fn approval_state_label/);
  assert.match(actionLabels, /fn receipt_write_state_label/);
  assert.match(actionLabels, /no approval required/);
  assert.match(actionLabels, /approval unknown/);
  assert.match(actionLabels, /read-only/);
  assert.match(actionLabels, /receipt write unknown/);
  assert.match(actionLabels, /metadata only/);
  assert.match(prompts, /launch_actions=/);
  assert.match(prompts, /quick_action_count=/);
  assert.match(prompts, /action_id=/);
  assert.match(prompts, /risk=/);
  assert.match(prompts, /requires_approval=/);
  assert.match(prompts, /writes_receipts=/);
  assert.match(prompts, /next_action=/);
});

test("deploy rail renders launch approval before provider dry-run rows", () => {
  const source = read("crates/agent_ui/src/dx_deploy_rail.rs");
  const launchGateIndex = source.indexOf(
    ".child(deploy_launch_gate_state(&snapshot.launch_gate, cx))",
  );
  const matrixIndex = source.indexOf(".child(deploy_capability_matrix_state(");

  assert.notEqual(launchGateIndex, -1);
  assert.notEqual(matrixIndex, -1);
  assert.ok(
    launchGateIndex < matrixIndex,
    "launch approval should be visible before plan/status/provider rows",
  );
});

test("deploy source files stay small enough for maintenance", () => {
  const files = deploySourceFiles();

  assert.ok(files.length >= 12, "expected the deploy lane to stay split by ownership");
  for (const fileName of files) {
    const lineCount = read(`${deploySourceDir}/${fileName}`).split(/\r?\n/).length;
    assert.ok(lineCount < 300, `${fileName} has ${lineCount} lines`);
  }
});

test("deploy status docs name the repeatable source guard", () => {
  const docs = [read("DX.md"), read("todo.txt"), read("changelog.txt")].join("\n");

  assert.match(docs, /DX Deploy panel source guard/);
  assert.match(docs, /DX Deploy launch evidence-source rows/);
  assert.match(docs, /five-source launch evidence/);
  assert.match(docs, /stable evidence id/);
  assert.match(docs, /receipt freshness/);
  assert.match(docs, /100-point deploy status/);
  assert.match(docs, /49\/100/);
  assert.match(docs, /49\/100 estimated/);
  assert.match(docs, /launch quick actions/);
  assert.match(docs, /quick-action identities/);
  assert.match(docs, /source blocker evidence paths/);
  assert.match(docs, /warning evidence paths/);
  assert.match(docs, /evidence-source blocker text/);
  assert.match(docs, /launch outcome counts/);
  assert.match(docs, /skipped expensive checks/);
  assert.match(docs, /launch scope and checked paths/);
  assert.match(docs, /scoring profile/);
  assert.match(docs, /bucket-score breakdown/);
  assert.match(docs, /launch buckets/);
  assert.match(docs, /approval evidence/);
  assert.match(docs, /source runtime and launch evidence/);
  assert.match(docs, /deploy receipt roots/);
  assert.match(docs, /dx_deploy_hub_roots\.rs/);
  assert.match(docs, /dx_deploy_check_roots\.rs/);
  assert.match(docs, /launch-check receipt roots/);
  assert.match(docs, /DX_HOME/);
  assert.match(docs, /D:\\Dx/);
  assert.match(docs, /workspace plus DX hub\/cli\/www receipt roots/);
  assert.match(docs, /receipt-write/);
  assert.match(docs, /script\/dx-deploy-panel-source\.test\.ts/);
});
