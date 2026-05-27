import test from "node:test";
import assert from "node:assert/strict";

import { lineCount, read } from "./dx-deploy-source-guard.ts";

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

test("launch gate notice parsing stays in a focused module", () => {
  const agentUi = read("crates/agent_ui/src/agent_ui.rs");
  const reader = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");
  const notices = read("crates/agent_ui/src/dx_deploy_launch_notices.rs");
  const fields = read("crates/agent_ui/src/dx_deploy_receipt_fields.rs");

  assert.match(agentUi, /^mod dx_deploy_launch_notices;$/m);
  assert.match(reader, /use crate::dx_deploy_launch_notices::\{/);
  assert.match(reader, /DxDeployLaunchGateNotice/);
  assert.match(reader, /notice_rows/);
  assert.match(
    reader,
    /use crate::dx_deploy_receipt_fields::\{\s*array_len, bool_field, first_string_array_item, string_field, usize_field,\s*\};/s,
  );
  assert.match(notices, /pub\(crate\) struct DxDeployLaunchGateNotice/);
  assert.match(notices, /pub\(crate\) fn notice_rows/);
  assert.match(notices, /\.take\(3\)/);
  assert.match(notices, /severity: string_field\(row, "severity"\)/);
  assert.match(notices, /evidence_path: string_field\(row, "evidence_path"\)/);
  assert.match(fields, /pub\(crate\) fn bool_field/);
  assert.doesNotMatch(reader, /fn notice_rows/);
  assert.doesNotMatch(reader, /fn string_field/);
  assert.doesNotMatch(reader, /fn bool_field/);
  assert.doesNotMatch(reader, /fn usize_field/);
  assert.doesNotMatch(reader, /fn array_len/);
  assert.doesNotMatch(reader, /fn first_string_array_item/);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_deploy_launch_gate.rs") < 220,
    "dx_deploy_launch_gate.rs should stay focused on selecting and assembling launch receipts",
  );
  assert.ok(
    lineCount("crates/agent_ui/src/dx_deploy_launch_notices.rs") < 60,
    "launch notice parsing should stay compact",
  );
});

test("launch gate keeps source-owned blocker provenance", () => {
  const notices = read("crates/agent_ui/src/dx_deploy_launch_notices.rs");
  const rail = read("crates/agent_ui/src/dx_deploy_launch_gate_rail.rs");
  const prompts = read("crates/agent_ui/src/dx_deploy_launch_prompts.rs");

  assert.match(notices, /pub severity: Option<String>/);
  assert.match(notices, /pub evidence_path: Option<String>/);
  assert.match(notices, /severity: string_field\(row, "severity"\)/);
  assert.match(notices, /evidence_path: string_field\(row, "evidence_path"\)/);
  assert.match(rail, /notice\.severity/);
  assert.match(rail, /notice\.evidence_path/);
  assert.match(rail, /evidence_path/);
  assert.match(prompts, /blocker\.severity/);
  assert.match(prompts, /blocker\.evidence_path/);
  assert.match(prompts, /severity=/);
  assert.match(prompts, /evidence_path=/);
});

test("launch gate surfaces malformed latest receipts", () => {
  const reader = read("crates/agent_ui/src/dx_deploy_launch_gate.rs");
  const notices = read("crates/agent_ui/src/dx_deploy_launch_notices.rs");

  assert.match(reader, /fn invalid_snapshot/);
  assert.match(reader, /Result<DxDeployLaunchGateSnapshot, String>/);
  assert.match(reader, /status: Some\("invalid receipt"\.to_string\(\)\)/);
  assert.match(
    reader,
    /invalid_launch_receipt_notice\(\s*candidate\.label\.clone\(\),\s*error,\s*\)/s,
  );
  assert.match(notices, /pub\(crate\) const INVALID_LAUNCH_RECEIPT_NEXT_ACTION/);
  assert.match(notices, /code: Some\("invalid_launch_receipt"\.to_string\(\)\)/);
  assert.match(reader, /Unable to parse dx-check launch receipt/);
  assert.match(notices, /Regenerate the dx-check launch receipt before using deploy readiness/);
  assert.doesNotMatch(reader, /\.find_map\(parse_launch_gate_candidate\)/);
  assert.doesNotMatch(reader, /serde_json::from_slice\(&buffer\)\.ok\(\)/);
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
  assert.match(prompts, /launch_status_score_label\(&snapshot\.launch_gate\)/);
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
