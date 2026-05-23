import test from "node:test";
import assert from "node:assert/strict";

import { lineCount, read } from "./dx-deploy-source-guard.ts";

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
