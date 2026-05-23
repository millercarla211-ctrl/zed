import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

test("DX launch workspace UI stays split by rail ownership", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const expectedModules = [
    "crates/agent_ui/src/dx_launch_workspace/agents.rs",
    "crates/agent_ui/src/dx_launch_workspace/audit.rs",
    "crates/agent_ui/src/dx_launch_workspace/binary_cache.rs",
    "crates/agent_ui/src/dx_launch_workspace/binary_cache_labels.rs",
    "crates/agent_ui/src/dx_launch_workspace/check.rs",
    "crates/agent_ui/src/dx_launch_workspace/check_labels.rs",
    "crates/agent_ui/src/dx_launch_workspace/contracts.rs",
    "crates/agent_ui/src/dx_launch_workspace/launch_status.rs",
    "crates/agent_ui/src/dx_launch_workspace/launch_status_labels.rs",
    "crates/agent_ui/src/dx_launch_workspace/launch_receipts.rs",
    "crates/agent_ui/src/dx_launch_workspace/list_labels.rs",
    "crates/agent_ui/src/dx_launch_workspace/proof.rs",
    "crates/agent_ui/src/dx_launch_workspace/proof_labels.rs",
    "crates/agent_ui/src/dx_launch_workspace/readiness.rs",
    "crates/agent_ui/src/dx_launch_workspace/source_audit.rs",
    "crates/agent_ui/src/dx_launch_workspace/sources.rs",
    "crates/agent_ui/src/dx_launch_workspace/tool_history.rs",
    "crates/agent_ui/src/dx_launch_workspace/www_evidence.rs",
  ];

  for (const module of expectedModules) {
    assert.ok(existsSync(module), `expected focused launch workspace module ${module}`);
  }

  assert.match(parent, /^mod agents;$/m);
  assert.match(parent, /^mod audit;$/m);
  assert.match(parent, /^mod binary_cache;$/m);
  assert.match(parent, /^mod binary_cache_labels;$/m);
  assert.match(parent, /^mod check;$/m);
  assert.match(parent, /^mod check_labels;$/m);
  assert.match(parent, /^mod contracts;$/m);
  assert.match(parent, /^mod launch_status;$/m);
  assert.match(parent, /^mod launch_status_labels;$/m);
  assert.match(parent, /^mod launch_receipts;$/m);
  assert.match(parent, /^mod list_labels;$/m);
  assert.match(parent, /^mod proof;$/m);
  assert.match(parent, /^mod proof_labels;$/m);
  assert.match(parent, /^mod readiness;$/m);
  assert.match(parent, /^mod source_audit;$/m);
  assert.match(parent, /^mod sources;$/m);
  assert.match(parent, /^mod tool_history;$/m);
  assert.match(parent, /^mod www_evidence;$/m);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace.rs") < 1000,
    "dx_launch_workspace.rs should stay a coordinator instead of owning every rail",
  );
});

test("DX launch workspace delegates Launch Receipts rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const launchReceipts = read("crates/agent_ui/src/dx_launch_workspace/launch_receipts.rs");
  const launchReceiptRows = read(
    "crates/agent_ui/src/dx_launch_workspace/launch_receipts/rows.rs",
  );
  const launchReceiptStatus = read(
    "crates/agent_ui/src/dx_launch_workspace/launch_receipts/status.rs",
  );

  assert.match(parent, /launch_receipts::launch_receipt_review_state/);
  assert.doesNotMatch(parent, /fn launch_receipt_review_state/);
  assert.doesNotMatch(parent, /fn launch_receipt_row/);
  assert.match(launchReceipts, /^mod rows;$/m);
  assert.match(launchReceipts, /^mod status;$/m);
  assert.doesNotMatch(launchReceipts, /use self::rows::launch_receipt_row/);
  assert.match(launchReceipts, /use self::status::launch_receipt_status_rows/);
  assert.match(launchReceipts, /pub\(super\) fn launch_receipt_review_state/);
  assert.doesNotMatch(launchReceipts, /fn launch_receipt_row/);
  assert.match(launchReceipts, /DxLaunchReceiptReviewSnapshot/);
  assert.doesNotMatch(launchReceipts, /DxLaunchReceiptSummary/);
  assert.doesNotMatch(launchReceipts, /dx-launch-receipt-latest-malformed/);
  assert.doesNotMatch(launchReceipts, /dx-launch-receipt-latest-stale/);
  assert.doesNotMatch(launchReceipts, /dx-launch-receipt-schema-review/);
  assert.doesNotMatch(launchReceipts, /dx-launch-receipt-warning/);
  assert.doesNotMatch(launchReceipts, /Missing launch receipt directory/);
  assert.doesNotMatch(launchReceipts, /latest\.malformed/);
  assert.doesNotMatch(launchReceipts, /snapshot\.snapshots\.first/);
  assert.match(launchReceipts, /children\(launch_receipt_status_rows\(snapshot, cx\)\)/);
  assert.match(launchReceipts, /use super::metric_row/);
  assert.match(launchReceiptRows, /pub\(super\) fn launch_receipt_row/);
  assert.match(launchReceiptRows, /DxLaunchReceiptSummary/);
  assert.match(launchReceiptRows, /dx-launch-receipt-\{\}-\{\}/);
  assert.match(launchReceiptRows, /review_launch_receipt_metadata/);
  assert.match(launchReceiptRows, /receipt\.display_state\(\)/);
  assert.match(launchReceiptRows, /cx\.theme\(\)\.colors\(\)\.element_background/);
  assert.match(launchReceiptStatus, /pub\(super\) fn launch_receipt_status_rows/);
  assert.match(launchReceiptStatus, /DxLaunchReceiptReviewSnapshot/);
  assert.match(launchReceiptStatus, /use super::rows::launch_receipt_row/);
  assert.match(launchReceiptStatus, /Missing launch receipt directory/);
  assert.match(launchReceiptStatus, /No cached launch latest receipt/);
  assert.match(launchReceiptStatus, /dx-launch-receipt-latest-malformed/);
  assert.match(launchReceiptStatus, /dx-launch-receipt-latest-stale/);
  assert.match(launchReceiptStatus, /dx-launch-receipt-schema-review/);
  assert.match(launchReceiptStatus, /dx-launch-receipt-warning/);
  assert.match(launchReceiptStatus, /snapshot\.snapshots\.first/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/launch_receipts.rs") < 75);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/launch_receipts/rows.rs") < 80);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/launch_receipts/status.rs") < 95);
});

test("DX launch workspace delegates WWW Evidence rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const wwwEvidence = read("crates/agent_ui/src/dx_launch_workspace/www_evidence.rs");

  assert.match(parent, /www_evidence::www_launch_evidence_state/);
  assert.doesNotMatch(parent, /fn www_launch_evidence_state/);
  assert.match(wwwEvidence, /pub\(super\) fn www_launch_evidence_state/);
  assert.match(wwwEvidence, /DxWwwLaunchEvidenceSnapshot/);
  assert.match(wwwEvidence, /dx-www-evidence-warning/);
  assert.match(wwwEvidence, /dx-www-evidence-partial/);
  assert.match(wwwEvidence, /use super::\{bounded_items, metric_row, muted_card, signal_row\}/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/www_evidence.rs") < 130);
});

test("DX launch workspace delegates Launch Audit rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const audit = read("crates/agent_ui/src/dx_launch_workspace/audit.rs");
  const auditStatus = read("crates/agent_ui/src/dx_launch_workspace/audit/status.rs");
  const auditWarnings = read("crates/agent_ui/src/dx_launch_workspace/audit/warnings.rs");

  assert.match(parent, /audit::launch_audit_state/);
  assert.doesNotMatch(parent, /fn launch_audit_state/);
  assert.match(audit, /^mod status;$/m);
  assert.match(audit, /^mod warnings;$/m);
  assert.match(audit, /use self::status::launch_audit_status_rows/);
  assert.match(audit, /use self::warnings::launch_audit_warning/);
  assert.match(audit, /pub\(super\) fn launch_audit_state/);
  assert.match(audit, /DxLaunchAuditSnapshot/);
  assert.match(audit, /children\(launch_audit_status_rows\(snapshot, cx\)\)/);
  assert.doesNotMatch(audit, /Missing launch example root/);
  assert.doesNotMatch(audit, /Missing \{label\}/);
  assert.doesNotMatch(audit, /schemas_path/);
  assert.doesNotMatch(audit, /fixtures_path/);
  assert.doesNotMatch(audit, /smoke_path/);
  assert.doesNotMatch(audit, /status_path/);
  assert.doesNotMatch(audit, /first_issue/);
  assert.doesNotMatch(audit, /redaction_requires_review/);
  assert.doesNotMatch(audit, /dx-launch-audit-warning/);
  assert.doesNotMatch(audit, /dx-launch-audit-redaction-review/);
  assert.doesNotMatch(audit, /dx-launch-audit-fanout-review/);
  assert.match(auditStatus, /pub\(super\) fn launch_audit_status_rows/);
  assert.match(auditStatus, /DxLaunchAuditSnapshot/);
  assert.match(auditStatus, /Missing launch example root/);
  assert.match(auditStatus, /Missing \{label\}/);
  assert.match(auditStatus, /schemas_path/);
  assert.match(auditStatus, /fixtures_path/);
  assert.match(auditStatus, /smoke_path/);
  assert.match(auditStatus, /status_path/);
  assert.match(auditStatus, /use super::super::muted_card/);
  assert.match(auditWarnings, /pub\(super\) fn launch_audit_warning/);
  assert.match(auditWarnings, /DxLaunchAuditSnapshot/);
  assert.match(auditWarnings, /SharedString/);
  assert.match(auditWarnings, /first_issue/);
  assert.match(auditWarnings, /redaction_requires_review/);
  assert.match(auditWarnings, /command_fanout_count/);
  assert.match(auditWarnings, /dx-launch-audit-warning/);
  assert.match(auditWarnings, /dx-launch-audit-redaction-review/);
  assert.match(auditWarnings, /dx-launch-audit-fanout-review/);
  assert.match(audit, /use super::\{bounded_items, metric_row, signal_row\}/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/audit.rs") < 95);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/audit/status.rs") < 55);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/audit/warnings.rs") < 45);
});

test("DX launch workspace delegates Source Audit rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const sourceAudit = read("crates/agent_ui/src/dx_launch_workspace/source_audit.rs");
  const sourceAuditStatus = read("crates/agent_ui/src/dx_launch_workspace/source_audit/status.rs");
  const sourceAuditWarnings = read(
    "crates/agent_ui/src/dx_launch_workspace/source_audit/warnings.rs",
  );

  assert.match(parent, /source_audit::launch_source_audit_state/);
  assert.doesNotMatch(parent, /fn launch_source_audit_state/);
  assert.match(sourceAudit, /^mod status;$/m);
  assert.match(sourceAudit, /^mod warnings;$/m);
  assert.match(sourceAudit, /use self::status::launch_source_audit_status_rows/);
  assert.match(sourceAudit, /use self::warnings::launch_source_audit_warning/);
  assert.match(sourceAudit, /pub\(super\) fn launch_source_audit_state/);
  assert.match(sourceAudit, /DxLaunchSourceAuditSnapshot/);
  assert.doesNotMatch(sourceAudit, /dx-source-audit-invalid/);
  assert.doesNotMatch(sourceAudit, /Missing source audit root/);
  assert.doesNotMatch(sourceAudit, /Missing source audit markdown summary/);
  assert.match(sourceAudit, /children\(launch_source_audit_status_rows\(snapshot, cx\)\)/);
  assert.doesNotMatch(sourceAudit, /first_issue/);
  assert.doesNotMatch(sourceAudit, /dx-source-audit-template-trust/);
  assert.doesNotMatch(sourceAudit, /dx-source-audit-www-qa/);
  assert.match(sourceAuditStatus, /pub\(super\) fn launch_source_audit_status_rows/);
  assert.match(sourceAuditStatus, /DxLaunchSourceAuditSnapshot/);
  assert.match(sourceAuditStatus, /Missing source audit root/);
  assert.match(sourceAuditStatus, /No source audit latest receipt/);
  assert.match(sourceAuditStatus, /dx-source-audit-invalid/);
  assert.match(sourceAuditStatus, /Missing source audit markdown summary/);
  assert.match(sourceAuditStatus, /Missing DX Studio QA receipt/);
  assert.match(sourceAuditStatus, /use super::super::\{muted_card, signal_row\}/);
  assert.match(sourceAuditWarnings, /pub\(super\) fn launch_source_audit_warning/);
  assert.match(sourceAuditWarnings, /DxLaunchSourceAuditSnapshot/);
  assert.match(sourceAuditWarnings, /SharedString/);
  assert.match(sourceAuditWarnings, /first_issue/);
  assert.match(sourceAuditWarnings, /risk_review_count/);
  assert.match(sourceAuditWarnings, /template_trust_passed/);
  assert.match(sourceAuditWarnings, /dx_studio_passed/);
  assert.match(sourceAuditWarnings, /dx-source-audit-warning/);
  assert.match(sourceAuditWarnings, /dx-source-audit-risk/);
  assert.match(sourceAuditWarnings, /dx-source-audit-template-trust/);
  assert.match(sourceAuditWarnings, /dx-source-audit-www-qa/);
  assert.match(sourceAudit, /use super::\{bounded_items, metric_row, signal_row, yes_no\}/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/source_audit.rs") < 100);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/source_audit/status.rs") < 70);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace/source_audit/warnings.rs") < 60,
  );
});

test("DX launch workspace delegates Launch Handoff rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const contracts = read("crates/agent_ui/src/dx_launch_workspace/contracts.rs");
  const contractStatus = read("crates/agent_ui/src/dx_launch_workspace/contracts/status.rs");

  assert.match(parent, /contracts::launch_contract_state/);
  assert.doesNotMatch(parent, /fn launch_contract_state/);
  assert.match(contracts, /^mod status;$/m);
  assert.match(contracts, /use self::status::launch_contract_status_rows/);
  assert.match(contracts, /pub\(super\) fn launch_contract_state/);
  assert.match(contracts, /DxLaunchContractSnapshot/);
  assert.doesNotMatch(contracts, /dx-launch-contract-fanout-review/);
  assert.doesNotMatch(contracts, /dx-launch-contract-redaction-review/);
  assert.doesNotMatch(contracts, /dx-launch-contract-warning/);
  assert.doesNotMatch(contracts, /Missing import manifest/);
  assert.doesNotMatch(contracts, /redaction_requires_review/);
  assert.doesNotMatch(contracts, /snapshot\.first_action/);
  assert.match(contracts, /children\(launch_contract_status_rows\(snapshot, cx\)\)/);
  assert.match(contracts, /use super::\{bounded_items, metric_row\}/);
  assert.match(contractStatus, /pub\(super\) fn launch_contract_status_rows/);
  assert.match(contractStatus, /DxLaunchContractSnapshot/);
  assert.match(contractStatus, /Missing import manifest/);
  assert.match(contractStatus, /Missing handoff packet/);
  assert.match(contractStatus, /dx-launch-contract-warning/);
  assert.match(contractStatus, /dx-launch-contract-redaction-review/);
  assert.match(contractStatus, /dx-launch-contract-fanout-review/);
  assert.match(contractStatus, /snapshot\.first_action/);
  assert.match(contractStatus, /use super::super::\{metric_row, muted_card, signal_row\}/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/contracts.rs") < 80);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/contracts/status.rs") < 80);
});

test("DX launch workspace delegates Launch Status rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const launchStatus = read("crates/agent_ui/src/dx_launch_workspace/launch_status.rs");
  const launchStatusRows = read("crates/agent_ui/src/dx_launch_workspace/launch_status/rows.rs");
  const launchStatusStatus = read(
    "crates/agent_ui/src/dx_launch_workspace/launch_status/status.rs",
  );
  const launchStatusWarnings = read(
    "crates/agent_ui/src/dx_launch_workspace/launch_status/warnings.rs",
  );
  const launchStatusLabels = read(
    "crates/agent_ui/src/dx_launch_workspace/launch_status_labels.rs",
  );

  assert.match(parent, /launch_status::launch_status_state/);
  assert.doesNotMatch(parent, /fn launch_status_state/);
  assert.match(launchStatus, /^mod rows;$/m);
  assert.match(launchStatus, /^mod status;$/m);
  assert.match(launchStatus, /^mod warnings;$/m);
  assert.match(launchStatus, /use self::rows::launch_status_valid_detail_rows/);
  assert.match(launchStatus, /use self::status::launch_status_status_rows/);
  assert.match(launchStatus, /pub\(super\) fn launch_status_state/);
  assert.match(launchStatus, /DxLaunchStatusSnapshot/);
  assert.match(launchStatus, /children\(launch_status_status_rows\(snapshot, cx\)\)/);
  assert.doesNotMatch(launchStatus, /fn launch_status_warning/);
  assert.doesNotMatch(launchStatus, /launch_status_warning\(snapshot\)/);
  assert.doesNotMatch(launchStatus, /redaction_requires_review/);
  assert.doesNotMatch(launchStatus, /launch_status_optional_summary/);
  assert.doesNotMatch(launchStatus, /Missing launch receipts/);
  assert.doesNotMatch(launchStatus, /Run dx launch status --json/);
  assert.doesNotMatch(launchStatus, /launch_status_next_action_label/);
  assert.doesNotMatch(launchStatus, /IconName::Warning/);
  assert.match(launchStatusRows, /pub\(super\) fn launch_status_valid_detail_rows/);
  assert.match(launchStatusRows, /DxLaunchStatusSnapshot/);
  assert.match(launchStatusRows, /Agent Next/);
  assert.match(launchStatusRows, /Token Next/);
  assert.match(launchStatusRows, /Discovery Next/);
  assert.match(launchStatusRows, /launch_status_optional_summary/);
  assert.match(launchStatusRows, /launch_status_next_action_label/);
  assert.match(launchStatusStatus, /pub\(super\) fn launch_status_status_rows/);
  assert.match(launchStatusStatus, /DxLaunchStatusSnapshot/);
  assert.match(launchStatusStatus, /Missing launch receipts/);
  assert.match(launchStatusStatus, /Run dx launch status --json/);
  assert.match(launchStatusStatus, /launch_status_warning\(snapshot\)/);
  assert.match(launchStatusStatus, /launch_status_next_action_label/);
  assert.match(launchStatusStatus, /IconName::Warning/);
  assert.match(launchStatusStatus, /use super::warnings::launch_status_warning/);
  assert.match(launchStatusStatus, /use super::super::\{muted_card, signal_row\}/);
  assert.match(launchStatusWarnings, /pub\(super\) fn launch_status_warning/);
  assert.match(launchStatusWarnings, /DxLaunchStatusSnapshot/);
  assert.match(launchStatusWarnings, /SharedString/);
  assert.match(launchStatusWarnings, /dx-launch-status-invalid/);
  assert.match(launchStatusWarnings, /dx-launch-status-redaction-review/);
  assert.match(launchStatusWarnings, /dx-launch-status-warning/);
  assert.match(launchStatus, /use super::launch_status_labels::\{/);
  assert.match(launchStatus, /use super::\{metric_row, yes_no\}/);
  assert.doesNotMatch(launchStatus, /snapshot\.operator_summary\.clone\(\)/);
  assert.doesNotMatch(launchStatus, /snapshot\.redaction_summary\.is_empty\(\)/);
  assert.match(launchStatusLabels, /pub\(crate\) fn launch_status_summary_label/);
  assert.match(launchStatusLabels, /pub\(crate\) fn launch_status_next_action_label/);
  assert.match(launchStatusLabels, /pub\(crate\) fn launch_status_command_label/);
  assert.match(launchStatusLabels, /pub\(crate\) fn launch_status_optional_summary/);
  assert.match(launchStatusLabels, /launch_status_labels_trim_nonblank_receipt_text/);
  assert.match(launchStatusLabels, /launch_status_labels_fall_back_for_blank_receipt_text/);
  assert.match(launchStatusLabels, /launch_status_optional_summary_ignores_blank_text/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/launch_status.rs") < 90);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/launch_status/rows.rs") < 60);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/launch_status/status.rs") < 65);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace/launch_status/warnings.rs") < 45,
  );
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/launch_status_labels.rs") < 110);
});

test("DX launch workspace delegates Launch Gate rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const readiness = read("crates/agent_ui/src/dx_launch_workspace/readiness.rs");
  const readinessExamples = read("crates/agent_ui/src/dx_launch_workspace/readiness/examples.rs");
  const readinessWarnings = read("crates/agent_ui/src/dx_launch_workspace/readiness/warnings.rs");

  assert.match(parent, /readiness::launch_readiness_state/);
  assert.doesNotMatch(parent, /fn launch_readiness_state/);
  assert.match(readiness, /^mod examples;$/m);
  assert.match(readiness, /^mod warnings;$/m);
  assert.match(
    readiness,
    /use self::\{examples::launch_readiness_example_rows, warnings::launch_readiness_warning\}/,
  );
  assert.match(readiness, /pub\(super\) fn launch_readiness_state/);
  assert.match(readiness, /DxLaunchReadinessSnapshot/);
  assert.doesNotMatch(readiness, /dx-launch-readiness-warning/);
  assert.doesNotMatch(readiness, /dx-launch-readiness-redaction-review/);
  assert.doesNotMatch(readiness, /dx-launch-readiness-fanout-review/);
  assert.doesNotMatch(readiness, /snapshot\.examples\.iter\(\)\.take\(3\)/);
  assert.match(readiness, /use super::\{bounded_items, metric_row, muted_card\}/);
  assert.match(readinessExamples, /pub\(super\) fn launch_readiness_example_rows/);
  assert.match(readinessExamples, /DxLaunchReadinessSnapshot/);
  assert.match(readinessExamples, /snapshot\.examples\.iter\(\)\.take\(3\)/);
  assert.match(readinessExamples, /Example \{\}/);
  assert.match(readinessExamples, /Next \{\}/);
  assert.match(readinessExamples, /use super::super::metric_row/);
  assert.match(readinessWarnings, /pub\(super\) fn launch_readiness_warning/);
  assert.match(readinessWarnings, /DxLaunchReadinessSnapshot/);
  assert.match(readinessWarnings, /snapshot\.first_issue/);
  assert.match(readinessWarnings, /redaction_requires_review/);
  assert.match(readinessWarnings, /no_command_fanout/);
  assert.match(readinessWarnings, /dx-launch-readiness-warning/);
  assert.match(readinessWarnings, /dx-launch-readiness-redaction-review/);
  assert.match(readinessWarnings, /dx-launch-readiness-fanout-review/);
  assert.match(readinessWarnings, /use super::super::signal_row/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/readiness.rs") < 110);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/readiness/examples.rs") < 35);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/readiness/warnings.rs") < 45);
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
  const agentActions = read("crates/agent_ui/src/dx_launch_workspace/agents/actions.rs");
  const agentAutomations = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/automations.rs",
  );
  const agentBridge = read("crates/agent_ui/src/dx_launch_workspace/agents/bridge.rs");
  const agentBridgeReview = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/bridge/review.rs",
  );
  const agentBridgeSummary = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/bridge/summary.rs",
  );
  const agentBridgeSummaryContract = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/bridge/summary/contract.rs",
  );
  const agentBridgeSummaryGate = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/bridge/summary/gate.rs",
  );
  const agentBridgeSummaryImport = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/bridge/summary/import.rs",
  );
  const agentBridgeSummaryOverview = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/bridge/summary/overview.rs",
  );
  const agentProviderLabels = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/provider_labels.rs",
  );
  const agentProviderDetailLabels = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/provider_labels/detail.rs",
  );
  const agentProviderStateLabels = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/provider_labels/state.rs",
  );
  const agentProviderTextLabels = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/provider_labels/text.rs",
  );
  const agentProviderLabelTests = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/provider_labels_tests.rs",
  );
  const agentProviders = read("crates/agent_ui/src/dx_launch_workspace/agents/providers.rs");
  const agentProviderRows = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/providers/rows.rs",
  );
  const agentReceipts = read("crates/agent_ui/src/dx_launch_workspace/agents/receipts.rs");
  const agentReceiptLabels = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/receipts/labels.rs",
  );
  const agentReceiptRows = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/receipts/rows.rs",
  );
  const agentReceiptRowDetails = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/receipts/row_details.rs",
  );
  const agentReceiptStatus = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/receipts/status.rs",
  );
  const agentSocial = read("crates/agent_ui/src/dx_launch_workspace/agents/social.rs");
  const agentSocialRows = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/social/rows.rs",
  );
  const agentSocialActions = read(
    "crates/agent_ui/src/dx_launch_workspace/agents/social_actions.rs",
  );
  const sources = read("crates/agent_ui/src/dx_launch_workspace/sources.rs");
  const sourceAttachments = read(
    "crates/agent_ui/src/dx_launch_workspace/sources/attachments.rs",
  );
  const sourceReceipts = read("crates/agent_ui/src/dx_launch_workspace/sources/receipts.rs");
  const sourceRows = read("crates/agent_ui/src/dx_launch_workspace/sources/rows.rs");
  const sourceDrilldowns = read(
    "crates/agent_ui/src/dx_launch_workspace/sources/drilldowns.rs",
  );
  const sourceKinds = read("crates/agent_ui/src/dx_launch_workspace/sources/kinds.rs");

  assert.match(parent, /agents::dx_agent_bridge_state/);
  assert.match(parent, /agents::dx_agent_social_state/);
  assert.match(parent, /agents::dx_agent_automation_state/);
  assert.match(parent, /sources::source_set_stack/);
  assert.doesNotMatch(parent, /fn dx_agent_bridge_state/);
  assert.doesNotMatch(parent, /fn source_set_stack/);
  assert.match(agents, /^mod actions;$/m);
  assert.match(agents, /^mod automations;$/m);
  assert.match(agents, /^mod bridge;$/m);
  assert.match(agents, /^mod provider_labels;$/m);
  assert.match(agents, /^mod providers;$/m);
  assert.match(agents, /^mod receipts;$/m);
  assert.match(agents, /^mod social;$/m);
  assert.match(agents, /^mod social_actions;$/m);
  assert.match(agents, /pub\(super\) use automations::dx_agent_automation_state/);
  assert.match(agents, /pub\(super\) use bridge::dx_agent_bridge_state/);
  assert.match(agents, /pub\(super\) use providers::dx_agent_provider_state/);
  assert.match(agents, /pub\(super\) use receipts::dx_agent_receipt_state/);
  assert.match(agents, /pub\(super\) use social::dx_agent_social_state/);
  assert.doesNotMatch(agents, /fn dx_agent_action_line/);
  assert.doesNotMatch(agents, /fn dx_agent_automation_row/);
  assert.doesNotMatch(agents, /fn dx_agent_bridge_state/);
  assert.doesNotMatch(agents, /fn dx_agent_provider_row/);
  assert.doesNotMatch(agents, /fn dx_agent_model_row/);
  assert.doesNotMatch(agents, /fn dx_agent_receipt_row/);
  assert.doesNotMatch(agents, /fn dx_agent_receipt_root_state/);
  assert.doesNotMatch(agents, /fn dx_agent_social_action_row/);
  assert.doesNotMatch(agents, /fn dx_agent_social_row/);
  assert.match(agentActions, /pub\(super\) fn dx_agent_action_line/);
  assert.match(agentActions, /DxAgentRowAction/);
  assert.match(agentAutomations, /pub\(in super::super\) fn dx_agent_automation_state/);
  assert.match(agentAutomations, /fn dx_agent_automation_row/);
  assert.match(agentAutomations, /DxAgentAutomation/);
  assert.match(agentAutomations, /dx-agent-automation-\{ix\}/);
  assert.match(agentAutomations, /use super::actions::dx_agent_action_line/);
  assert.match(agentBridge, /pub\(in super::super\) fn dx_agent_bridge_state/);
  assert.match(agentBridge, /DxAgentBridgeSnapshot/);
  assert.match(agentBridge, /^mod review;$/m);
  assert.match(agentBridge, /^mod summary;$/m);
  assert.match(agentBridge, /use self::review::dx_agent_bridge_review_row/);
  assert.match(agentBridge, /use self::summary::dx_agent_bridge_summary_rows/);
  assert.match(agentBridge, /children\(dx_agent_bridge_summary_rows\(snapshot\)\)/);
  assert.match(agentBridge, /child\(dx_agent_bridge_review_row\(snapshot\)\)/);
  assert.doesNotMatch(agentBridge, /recovery_counts\.label/);
  assert.match(agentBridge, /dx-agent-action-error/);
  assert.doesNotMatch(agentBridge, /dx-agent-release-gate-blocker/);
  assert.doesNotMatch(agentBridge, /dx-agent-import-summary-fanout-review/);
  assert.doesNotMatch(agentBridge, /dx-agent-contract-redaction-review/);
  assert.doesNotMatch(agentBridge, /dx-agent-bridge-error/);
  assert.doesNotMatch(agentBridge, /snapshot\.last_error/);
  assert.match(agentBridge, /use super::super::\{metric_row, muted_card, signal_row\}/);
  assert.match(agentBridgeReview, /pub\(super\) fn dx_agent_bridge_review_row/);
  assert.match(agentBridgeReview, /DxAgentBridgeSnapshot/);
  assert.match(agentBridgeReview, /dx-agent-action-error-redaction/);
  assert.match(agentBridgeReview, /dx-agent-release-gate-blocker/);
  assert.match(agentBridgeReview, /dx-agent-release-gate-fanout-review/);
  assert.match(agentBridgeReview, /dx-agent-release-gate-warning/);
  assert.match(agentBridgeReview, /dx-agent-import-summary-blocker/);
  assert.match(agentBridgeReview, /dx-agent-import-summary-fanout-review/);
  assert.match(agentBridgeReview, /dx-agent-import-summary-warning/);
  assert.match(agentBridgeReview, /dx-agent-contract-redaction-review/);
  assert.match(agentBridgeReview, /dx-agent-bridge-error/);
  assert.match(agentBridgeReview, /snapshot\.release_gate\.next_action/);
  assert.match(agentBridgeReview, /use super::super::super::signal_row/);
  assert.match(agentBridgeSummary, /pub\(super\) fn dx_agent_bridge_summary_rows/);
  assert.match(agentBridgeSummary, /Vec<AnyElement>/);
  assert.match(agentBridgeSummary, /DxAgentBridgeSnapshot/);
  assert.match(agentBridgeSummary, /^mod contract;$/m);
  assert.match(agentBridgeSummary, /^mod gate;$/m);
  assert.match(agentBridgeSummary, /^mod import;$/m);
  assert.match(agentBridgeSummary, /^mod overview;$/m);
  assert.match(agentBridgeSummary, /dx_agent_bridge_overview_rows\(snapshot\)/);
  assert.match(agentBridgeSummary, /dx_agent_bridge_contract_rows\(snapshot\)/);
  assert.match(agentBridgeSummary, /dx_agent_bridge_import_rows\(snapshot\)/);
  assert.match(agentBridgeSummary, /dx_agent_bridge_gate_rows\(snapshot\)/);
  assert.doesNotMatch(agentBridgeSummary, /metric_row\("Bridge"/);
  assert.doesNotMatch(agentBridgeSummary, /metric_row\(\s*"Gate Recovery"/);
  assert.doesNotMatch(agentBridgeSummary, /receipt_index\.returned_receipt_count/);
  assert.doesNotMatch(agentBridgeSummary, /recovery_counts\.label\(\)/);
  assert.doesNotMatch(agentBridgeSummary, /no_command_fanout/);
  assert.match(agentBridgeSummaryOverview, /pub\(super\) fn dx_agent_bridge_overview_rows/);
  assert.match(agentBridgeSummaryOverview, /metric_row\("Bridge"/);
  assert.match(agentBridgeSummaryOverview, /connected_accounts_summary\.connected/);
  assert.match(agentBridgeSummaryOverview, /catalog\.present/);
  assert.match(agentBridgeSummaryContract, /pub\(super\) fn dx_agent_bridge_contract_rows/);
  assert.match(agentBridgeSummaryContract, /metric_row\("Contract"/);
  assert.match(agentBridgeSummaryContract, /provider_catalog_receipt_count/);
  assert.doesNotMatch(agentBridgeSummaryContract, /import_summary\.release_gate_status/);
  assert.match(agentBridgeSummaryImport, /pub\(super\) fn dx_agent_bridge_import_rows/);
  assert.match(agentBridgeSummaryImport, /metric_row\("Import"/);
  assert.match(agentBridgeSummaryImport, /import_summary\.release_gate_status/);
  assert.match(agentBridgeSummaryImport, /recovery_counts\.label\(\)/);
  assert.match(agentBridgeSummaryImport, /import_summary\.no_command_fanout/);
  assert.match(agentBridgeSummaryImport, /action_error\.present/);
  assert.match(agentBridgeSummaryGate, /pub\(super\) fn dx_agent_bridge_gate_rows/);
  assert.match(agentBridgeSummaryGate, /metric_row\("Gate"/);
  assert.match(agentBridgeSummaryGate, /metric_row\(\s*"Gate Recovery"/);
  assert.match(agentBridgeSummaryGate, /receipt_index\.returned_receipt_count/);
  assert.match(agentBridgeSummaryGate, /release_gate\.no_command_fanout/);
  assert.match(agentProviders, /pub\(in super::super\) fn dx_agent_provider_state/);
  assert.match(agentProviders, /^mod rows;$/m);
  assert.match(agentProviders, /use self::rows::\{dx_agent_model_row, dx_agent_provider_row\}/);
  assert.match(agentProviders, /dx-agent-provider-\{ix\}/);
  assert.match(agentProviders, /dx-agent-model-\{ix\}/);
  assert.doesNotMatch(agentProviders, /fn dx_agent_provider_row/);
  assert.doesNotMatch(agentProviders, /fn dx_agent_model_row/);
  assert.doesNotMatch(agentProviders, /DxAgentProvider/);
  assert.doesNotMatch(agentProviders, /DxAgentModel/);
  assert.doesNotMatch(agentProviders, /use super::provider_labels::\{/);
  assert.doesNotMatch(agentProviders, /provider\.compatibility\.join/);
  assert.doesNotMatch(agentProviders, /model\.compatibility\.join/);
  assert.match(agentProviderRows, /pub\(super\) fn dx_agent_provider_row/);
  assert.match(agentProviderRows, /pub\(super\) fn dx_agent_model_row/);
  assert.match(agentProviderRows, /DxAgentProvider/);
  assert.match(agentProviderRows, /DxAgentModel/);
  assert.match(agentProviderRows, /provider_state_label/);
  assert.match(agentProviderRows, /model_detail_label/);
  assert.doesNotMatch(agentProviderRows, /dx-agent-provider-\{ix\}/);
  assert.doesNotMatch(agentProviderRows, /dx-agent-model-\{ix\}/);
  assert.doesNotMatch(agentProviderRows, /DxAgentBridgeSnapshot/);
  assert.match(agentProviderLabels, /#\[path = "provider_labels\/detail\.rs"\]/);
  assert.match(agentProviderLabels, /#\[path = "provider_labels\/state\.rs"\]/);
  assert.match(agentProviderLabels, /#\[path = "provider_labels\/text\.rs"\]/);
  assert.match(agentProviderLabels, /#\[path = "provider_labels_tests\.rs"\]/);
  assert.match(agentProviderLabels, /pub\(crate\) use self::detail::\{/);
  assert.match(agentProviderLabels, /pub\(crate\) use self::state::\{/);
  assert.doesNotMatch(agentProviderLabels, /fn provider_state_label/);
  assert.doesNotMatch(agentProviderLabels, /fn provider_detail_label/);
  assert.match(agentProviderDetailLabels, /pub\(crate\) fn provider_detail_label/);
  assert.match(agentProviderDetailLabels, /pub\(crate\) fn model_detail_label/);
  assert.match(agentProviderDetailLabels, /fn compatibility_label/);
  assert.match(agentProviderDetailLabels, /use super::text::nonblank_or/);
  assert.doesNotMatch(agentProviderDetailLabels, /provider_state_label/);
  assert.match(agentProviderStateLabels, /pub\(crate\) fn provider_state_label/);
  assert.match(agentProviderStateLabels, /pub\(crate\) fn model_state_label/);
  assert.match(agentProviderStateLabels, /use super::text::nonblank_or/);
  assert.doesNotMatch(agentProviderStateLabels, /provider_detail_label/);
  assert.match(agentProviderTextLabels, /pub\(super\) fn nonblank_or/);
  assert.match(agentProviderLabelTests, /provider_detail_label_trims_blank_compatibility/);
  assert.match(agentProviderLabelTests, /model_detail_label_falls_back_for_blank_ids/);
  assert.match(agentProviderLabelTests, /detail_labels_disclose_compatibility_overflow/);
  assert.match(agentReceipts, /pub\(in super::super\) fn dx_agent_receipt_state/);
  assert.match(agentReceipts, /^mod labels;$/m);
  assert.match(agentReceipts, /^mod row_details;$/m);
  assert.match(agentReceipts, /^mod rows;$/m);
  assert.match(agentReceipts, /^mod status;$/m);
  assert.match(agentReceipts, /use self::rows::dx_agent_receipt_row/);
  assert.match(
    agentReceipts,
    /use self::status::\{dx_agent_receipt_root_state, dx_agent_receipt_warning_rows\}/,
  );
  assert.doesNotMatch(agentReceipts, /fn dx_agent_receipt_root_state/);
  assert.doesNotMatch(agentReceipts, /fn dx_agent_receipt_row/);
  assert.doesNotMatch(agentReceipts, /DxAgentReceipt/);
  assert.doesNotMatch(agentReceipts, /dx-agent-receipt-inbox-malformed/);
  assert.doesNotMatch(agentReceipts, /dx-agent-receipt-unsafe-row/);
  assert.match(agentReceipts, /children\(dx_agent_receipt_warning_rows\(snapshot, unsafe_count\)\)/);
  assert.match(agentReceipts, /use super::super::\{metric_row, muted_card\}/);
  assert.match(agentReceiptLabels, /pub\(super\) fn receipt_state_label/);
  assert.match(agentReceiptLabels, /pub\(super\) fn receipt_detail_label/);
  assert.match(agentReceiptLabels, /pub\(super\) fn receipt_provider_model_label/);
  assert.match(agentReceiptLabels, /pub\(super\) fn receipt_action_label/);
  assert.match(agentReceiptLabels, /pub\(super\) fn receipt_social_label/);
  assert.match(agentReceiptLabels, /pub\(super\) fn receipt_automation_label/);
  assert.match(agentReceiptLabels, /metadata_redacted/);
  assert.match(agentReceiptLabels, /retry_supported/);
  assert.match(agentReceiptLabels, /social_needs_auth/);
  assert.match(agentReceiptLabels, /automation_enabled/);
  assert.match(agentReceiptLabels, /use super::super::super::list_labels::yes_no/);
  assert.match(agentReceiptRows, /pub\(super\) fn dx_agent_receipt_row/);
  assert.match(agentReceiptRows, /DxAgentReceipt/);
  assert.match(agentReceiptRows, /use super::labels::receipt_state_label/);
  assert.match(agentReceiptRows, /use super::row_details::dx_agent_receipt_detail_rows/);
  assert.match(agentReceiptRows, /children\(dx_agent_receipt_detail_rows\(receipt\)\)/);
  assert.doesNotMatch(agentReceiptRows, /use super::labels::\{/);
  assert.doesNotMatch(agentReceiptRows, /receipt_detail_label/);
  assert.doesNotMatch(agentReceiptRows, /receipt_provider_model_label/);
  assert.doesNotMatch(agentReceiptRows, /receipt_action_label/);
  assert.doesNotMatch(agentReceiptRows, /receipt_social_label/);
  assert.doesNotMatch(agentReceiptRows, /receipt_automation_label/);
  assert.doesNotMatch(agentReceiptRows, /last_error/);
  assert.doesNotMatch(agentReceiptRows, /next_action/);
  assert.doesNotMatch(agentReceiptRows, /metadata_redacted/);
  assert.doesNotMatch(agentReceiptRows, /retry_supported/);
  assert.doesNotMatch(agentReceiptRows, /social_needs_auth/);
  assert.doesNotMatch(agentReceiptRows, /automation_enabled/);
  assert.doesNotMatch(agentReceiptRows, /use super::super::super::list_labels::yes_no/);
  assert.match(agentReceiptRows, /use super::super::super::metric_row/);
  assert.match(agentReceiptRowDetails, /pub\(super\) fn dx_agent_receipt_detail_rows/);
  assert.match(agentReceiptRowDetails, /receipt_detail_label/);
  assert.match(agentReceiptRowDetails, /receipt_provider_model_label/);
  assert.match(agentReceiptRowDetails, /receipt_action_label/);
  assert.match(agentReceiptRowDetails, /receipt_social_label/);
  assert.match(agentReceiptRowDetails, /receipt_automation_label/);
  assert.match(agentReceiptRowDetails, /last_error/);
  assert.match(agentReceiptRowDetails, /next_action/);
  assert.match(agentReceiptRowDetails, /schema_version/);
  assert.doesNotMatch(agentReceiptRowDetails, /receipt_state_label/);
  assert.match(agentReceiptStatus, /pub\(super\) fn dx_agent_receipt_root_state/);
  assert.match(agentReceiptStatus, /pub\(super\) fn dx_agent_receipt_warning_rows/);
  assert.match(agentReceiptStatus, /DxAgentBridgeSnapshot/);
  assert.match(agentReceiptStatus, /dx-agent-receipt-root-missing/);
  assert.match(agentReceiptStatus, /dx-agent-receipt-inbox-root-missing/);
  assert.match(agentReceiptStatus, /dx-agent-receipt-inbox-malformed/);
  assert.match(agentReceiptStatus, /dx-agent-receipt-index-error/);
  assert.match(agentReceiptStatus, /dx-agent-receipt-unsafe-row/);
  assert.match(agentReceiptStatus, /use super::super::super::signal_row/);
  assert.match(agentSocial, /pub\(in super::super\) fn dx_agent_social_state/);
  assert.match(agentSocial, /^mod rows;$/m);
  assert.match(agentSocial, /use self::rows::dx_agent_social_row/);
  assert.doesNotMatch(agentSocial, /fn dx_agent_social_row/);
  assert.doesNotMatch(agentSocial, /DxAgentSocialAccount/);
  assert.doesNotMatch(agentSocial, /use super::actions::dx_agent_action_line/);
  assert.match(agentSocial, /dx-agent-social-connect-receipt/);
  assert.match(agentSocial, /use super::social_actions::dx_agent_social_action_row/);
  assert.match(agentSocialRows, /pub\(super\) fn dx_agent_social_row/);
  assert.match(agentSocialRows, /DxAgentSocialAccount/);
  assert.match(agentSocialRows, /dx_agent_action_line/);
  assert.match(agentSocialRows, /Connected/);
  assert.match(agentSocialRows, /QR ready/);
  assert.match(agentSocialRows, /account\.next_action/);
  assert.match(agentSocialRows, /use super::super::actions::dx_agent_action_line/);
  assert.match(agentSocialRows, /use super::super::super::metric_row/);
  assert.match(agentSocialActions, /pub\(super\) fn dx_agent_social_action_row/);
  assert.match(agentSocialActions, /DxAgentSocialActionSummary/);
  assert.match(agentSocialActions, /connect_supported/);
  assert.match(agentSocialActions, /manual_revoke_required/);
  assert.match(sources, /pub\(super\) fn source_set_stack/);
  assert.match(sources, /^mod attachments;$/m);
  assert.match(sources, /^mod drilldowns;$/m);
  assert.match(sources, /^mod kinds;$/m);
  assert.match(sources, /^mod receipts;$/m);
  assert.match(sources, /^mod rows;$/m);
  assert.match(sources, /pub\(super\) use self::attachments::source_attachment_state/);
  assert.match(sources, /pub\(super\) use self::receipts::receipt_source_state/);
  assert.match(sources, /use self::rows::source_item_row/);
  assert.doesNotMatch(sources, /pub\(super\) fn source_attachment_state/);
  assert.doesNotMatch(sources, /DxSourceAttachmentSummary/);
  assert.doesNotMatch(sources, /Attach-ready/);
  assert.match(sourceAttachments, /pub\(super\) fn source_attachment_state/);
  assert.match(sourceAttachments, /DxSourceAttachmentSummary/);
  assert.match(sourceAttachments, /Attach-ready/);
  assert.match(sourceAttachments, /Produced media/);
  assert.match(sourceAttachments, /Restore previews/);
  assert.match(sourceAttachments, /Create a source-pack or media receipt first/);
  assert.match(sourceAttachments, /use super::super::\{metric_row, muted_card\}/);
  assert.doesNotMatch(sources, /pub\(super\) fn receipt_source_state/);
  assert.doesNotMatch(sources, /DxReceiptSnapshot/);
  assert.doesNotMatch(sources, /latest-receipt-\{ix\}/);
  assert.doesNotMatch(sources, /IconName::FileTextOutlined/);
  assert.doesNotMatch(sources, /fn source_item_row/);
  assert.doesNotMatch(sources, /fn source_receipt_drilldown_row/);
  assert.doesNotMatch(sources, /fn source_kind_icon/);
  assert.match(sourceReceipts, /pub\(super\) fn receipt_source_state/);
  assert.match(sourceReceipts, /DxReceiptSnapshot/);
  assert.match(sourceReceipts, /latest-receipt-\{ix\}/);
  assert.match(sourceReceipts, /IconName::FileTextOutlined/);
  assert.match(sourceReceipts, /Receipts not found/);
  assert.match(sourceReceipts, /use super::super::\{metric_row, muted_card, source_row\}/);
  assert.match(sourceRows, /use super::drilldowns::source_receipt_drilldown_row/);
  assert.match(sourceRows, /use super::kinds::source_kind_icon/);
  assert.match(sourceRows, /pub\(super\) fn source_item_row/);
  assert.doesNotMatch(sourceRows, /fn source_receipt_drilldown_row/);
  assert.doesNotMatch(sourceRows, /fn source_kind_icon/);
  assert.doesNotMatch(sourceRows, /DxSourceKind::ForgeRestorePreview/);
  assert.match(sourceRows, /source-proof-\{\}-\{ix\}/);
  assert.match(sourceRows, /source-warning-\{\}-\{ix\}/);
  assert.match(sourceRows, /use super::super::signal_row/);
  assert.match(sourceDrilldowns, /pub\(super\) fn source_receipt_drilldown_row/);
  assert.match(sourceDrilldowns, /DxSourceReceiptDrilldown/);
  assert.match(sourceDrilldowns, /source-receipt-label/);
  assert.match(sourceDrilldowns, /IconName::FileTextOutlined/);
  assert.match(sourceDrilldowns, /editor_background/);
  assert.match(sourceKinds, /pub\(super\) fn source_kind_icon/);
  assert.match(sourceKinds, /DxSourceKind::ForgeRestorePreview/);
  assert.match(sourceKinds, /IconName::Archive/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents.rs") < 40);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/actions.rs") < 60);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/automations.rs") < 100);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/bridge.rs") < 105);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/bridge/review.rs") < 105);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/bridge/summary.rs") < 35);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace/agents/bridge/summary/overview.rs") < 55,
  );
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace/agents/bridge/summary/contract.rs") < 45,
  );
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace/agents/bridge/summary/import.rs") < 60,
  );
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace/agents/bridge/summary/gate.rs") < 85,
  );
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/provider_labels.rs") < 20);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace/agents/provider_labels/detail.rs") < 55,
  );
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace/agents/provider_labels/state.rs") < 30,
  );
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace/agents/provider_labels/text.rs") < 15,
  );
  assert.ok(
    lineCount("crates/agent_ui/src/dx_launch_workspace/agents/provider_labels_tests.rs") < 90,
  );
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/providers.rs") < 90);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/providers/rows.rs") < 75);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/receipts.rs") < 105);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/receipts/labels.rs") < 95);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/receipts/row_details.rs") < 95);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/receipts/rows.rs") < 55);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/receipts/status.rs") < 80);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/social.rs") < 70);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/social/rows.rs") < 70);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/agents/social_actions.rs") < 90);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/sources.rs") < 95);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/sources/attachments.rs") < 60);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/sources/receipts.rs") < 55);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/sources/rows.rs") < 90);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/sources/drilldowns.rs") < 45);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/sources/kinds.rs") < 25);
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
  const checkRows = read("crates/agent_ui/src/dx_launch_workspace/check/rows.rs");
  const checkSummary = read(
    "crates/agent_ui/src/dx_launch_workspace/check/summary.rs",
  );
  const labels = read("crates/agent_ui/src/dx_launch_workspace/check_labels.rs");
  const labelCounts = read("crates/agent_ui/src/dx_launch_workspace/check_labels/counts.rs");
  const labelRun = read("crates/agent_ui/src/dx_launch_workspace/check_labels/run.rs");
  const labelTests = read("crates/agent_ui/src/dx_launch_workspace/check_labels_tests.rs");

  assert.match(parent, /check::check_score_state/);
  assert.doesNotMatch(parent, /fn check_score_state/);
  assert.doesNotMatch(parent, /fn check_outcome_label/);
  assert.match(check, /^mod rows;$/m);
  assert.match(check, /^mod summary;$/m);
  assert.match(check, /use self::rows::\{/);
  assert.match(check, /use self::summary::check_summary_rows/);
  assert.match(check, /pub\(super\) fn check_score_state/);
  assert.doesNotMatch(check, /fn check_outcome_label/);
  assert.doesNotMatch(check, /check_duration_label/);
  assert.doesNotMatch(check, /fn checked_paths_label/);
  assert.doesNotMatch(check, /fn skipped_checks_label/);
  assert.doesNotMatch(check, /last_run_label_with_generated_at/);
  assert.doesNotMatch(check, /scoring_config_applies_to_score/);
  assert.doesNotMatch(check, /panel\.checked_paths\.iter\(\)/);
  assert.doesNotMatch(check, /requires_user_approval/);
  assert.doesNotMatch(check, /section\.estimated/);
  assert.match(checkSummary, /pub\(super\) fn check_summary_rows/);
  assert.match(checkSummary, /DxCheckPanelSnapshot/);
  assert.match(checkSummary, /last_run_label_with_generated_at/);
  assert.match(checkSummary, /check_duration_label/);
  assert.match(checkSummary, /checked_paths_label/);
  assert.match(checkSummary, /skipped_checks_label/);
  assert.match(checkSummary, /panel\.checked_paths\.iter\(\)/);
  assert.match(checkSummary, /panel\.skipped_expensive_checks\.iter\(\)/);
  assert.match(checkSummary, /scoring_config_applies_to_score/);
  assert.match(checkRows, /pub\(super\) fn check_section_row/);
  assert.match(checkRows, /DxCheckPanelSection/);
  assert.match(checkRows, /section\.estimated/);
  assert.match(checkRows, /pub\(super\) fn check_blocker_row/);
  assert.match(checkRows, /dx-check-panel-blocker-\{ix\}/);
  assert.match(checkRows, /pub\(super\) fn check_warning_rows/);
  assert.match(checkRows, /dx-check-panel-warning-\{ix\}/);
  assert.match(checkRows, /Warn next/);
  assert.match(checkRows, /pub\(super\) fn check_quick_fix_rows/);
  assert.match(checkRows, /DxCheckPanelQuickFix/);
  assert.match(checkRows, /requires_user_approval/);
  assert.match(checkRows, /writes_receipts/);
  assert.match(labels, /#\[path = "check_labels\/counts\.rs"\]\s*mod counts;/);
  assert.match(labels, /#\[path = "check_labels\/run\.rs"\]\s*mod run;/);
  assert.match(labels, /pub\(crate\) use counts::\{/);
  assert.match(labels, /pub\(crate\) use run::\{/);
  assert.match(labels, /#\[cfg\(test\)\]\s*mod check_labels_tests;/);
  assert.doesNotMatch(labels, /fn nonblank_count/);
  assert.doesNotMatch(labels, /pub\(crate\) fn check_outcome_label/);
  assert.match(labelCounts, /pub\(crate\) fn check_outcome_label/);
  assert.match(labelCounts, /pub\(crate\) fn checked_paths_label/);
  assert.match(labelCounts, /pub\(crate\) fn skipped_checks_label/);
  assert.match(labelCounts, /fn nonblank_count/);
  assert.match(labelCounts, /filter\(\|value\| !value\.trim\(\)\.is_empty\(\)\)/);
  assert.match(labelRun, /pub\(crate\) fn check_duration_label/);
  assert.match(labelRun, /pub\(crate\) fn last_run_label_with_generated_at/);
  assert.match(labelRun, /Last run Unix ms: \{generated_at\}/);
  assert.doesNotMatch(labels, /mod tests/);
  assert.doesNotMatch(labels, /#\[test\]/);
  assert.match(labelTests, /use super::\*/);
  assert.match(labelTests, /last_run_label_uses_generated_timestamp_when_label_is_blank/);
  assert.match(labelTests, /last_run_label_trims_nonblank_receipt_labels/);
  assert.match(labelTests, /path_and_skip_labels_cover_empty_single_and_plural/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/check.rs") < 100);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/check/rows.rs") < 110);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/check/summary.rs") < 80);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/check_labels.rs") < 15);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/check_labels/counts.rs") < 55);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/check_labels/run.rs") < 45);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/check_labels_tests.rs") < 120);
});

test("DX launch workspace delegates Tool History rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const toolHistory = read("crates/agent_ui/src/dx_launch_workspace/tool_history.rs");
  const toolHistoryRows = read("crates/agent_ui/src/dx_launch_workspace/tool_history/rows.rs");
  const toolHistorySummaryRows = read(
    "crates/agent_ui/src/dx_launch_workspace/tool_history/summary_rows.rs",
  );

  assert.match(parent, /tool_history::tool_history_state/);
  assert.doesNotMatch(parent, /fn tool_history_state/);
  assert.doesNotMatch(parent, /fn tool_history_bucket/);
  assert.doesNotMatch(parent, /fn tool_history_summary_row/);
  assert.match(toolHistory, /^mod rows;$/m);
  assert.match(toolHistory, /^mod summary_rows;$/m);
  assert.match(toolHistory, /use self::rows::tool_history_bucket/);
  assert.match(toolHistory, /pub\(super\) fn tool_history_state/);
  assert.doesNotMatch(toolHistory, /fn tool_history_bucket/);
  assert.doesNotMatch(toolHistory, /fn tool_history_summary_row/);
  assert.doesNotMatch(toolHistory, /DxToolHistoryReceiptSummary/);
  assert.match(toolHistory, /dx-tool-history-\{ix\}/);
  assert.match(toolHistoryRows, /pub\(super\) fn tool_history_bucket/);
  assert.match(toolHistoryRows, /use super::summary_rows::tool_history_summary_row/);
  assert.doesNotMatch(toolHistoryRows, /fn tool_history_summary_row/);
  assert.doesNotMatch(toolHistoryRows, /DxToolHistoryReceiptSummary/);
  assert.match(toolHistoryRows, /bucket\.latest_summaries/);
  assert.doesNotMatch(toolHistoryRows, /target_path/);
  assert.doesNotMatch(toolHistoryRows, /restore_destination_root/);
  assert.doesNotMatch(toolHistoryRows, /blocker_count/);
  assert.match(toolHistoryRows, /source_row/);
  assert.match(toolHistorySummaryRows, /pub\(super\) fn tool_history_summary_row/);
  assert.match(toolHistorySummaryRows, /DxToolHistoryReceiptSummary/);
  assert.match(toolHistorySummaryRows, /target_path/);
  assert.match(toolHistorySummaryRows, /restore_destination_root/);
  assert.match(toolHistorySummaryRows, /blocker_count/);
  assert.match(toolHistorySummaryRows, /signal_row/);
  assert.match(toolHistorySummaryRows, /row_id/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/tool_history.rs") < 35);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/tool_history/rows.rs") < 70);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/tool_history/summary_rows.rs") < 70);
});

test("DX launch workspace delegates Proof rail rendering", () => {
  const parent = read("crates/agent_ui/src/dx_launch_workspace.rs");
  const proof = read("crates/agent_ui/src/dx_launch_workspace/proof.rs");
  const proofFreshness = read("crates/agent_ui/src/dx_launch_workspace/proof/freshness.rs");
  const proofLabels = read("crates/agent_ui/src/dx_launch_workspace/proof_labels.rs");
  const proofLabelEvidence = read("crates/agent_ui/src/dx_launch_workspace/proof_labels/evidence.rs");
  const proofLabelReceipt = read("crates/agent_ui/src/dx_launch_workspace/proof_labels/receipt.rs");
  const proofLabelTests = read("crates/agent_ui/src/dx_launch_workspace/proof_labels_tests.rs");
  const proofRuntimeRows = read(
    "crates/agent_ui/src/dx_launch_workspace/proof/runtime_rows.rs",
  );
  const proofRuntimePlanRows = read(
    "crates/agent_ui/src/dx_launch_workspace/proof/runtime_rows/plan.rs",
  );
  const proofRuntimeReceiptRows = read(
    "crates/agent_ui/src/dx_launch_workspace/proof/runtime_rows/receipt.rs",
  );

  assert.match(parent, /proof::proof_freshness_state/);
  assert.match(parent, /proof::runtime_proof_status_state/);
  assert.doesNotMatch(parent, /fn proof_freshness_state/);
  assert.doesNotMatch(parent, /fn runtime_proof_status_state/);
  assert.doesNotMatch(parent, /fn runtime_proof_plan_row/);
  assert.doesNotMatch(parent, /fn runtime_proof_receipt_row/);
  assert.match(proof, /^mod freshness;$/m);
  assert.match(proof, /^mod runtime_rows;$/m);
  assert.match(proof, /pub\(super\) use freshness::proof_freshness_state/);
  assert.match(proof, /pub\(super\) fn runtime_proof_status_state/);
  assert.doesNotMatch(proof, /fn proof_freshness_bucket_row/);
  assert.doesNotMatch(proof, /fn runtime_proof_plan_row/);
  assert.doesNotMatch(proof, /fn runtime_proof_receipt_row/);
  assert.match(proof, /use self::runtime_rows::\{runtime_proof_plan_row, runtime_proof_receipt_row\}/);
  assert.match(proofFreshness, /pub\(in super::super\) fn proof_freshness_state/);
  assert.match(proofFreshness, /fn proof_freshness_bucket_row/);
  assert.match(proofFreshness, /DxProofFreshnessBucket/);
  assert.match(proofFreshness, /dx-proof-freshness-\{ix\}/);
  assert.match(proofFreshness, /use super::super::metric_row/);
  assert.match(proofRuntimeRows, /^mod plan;$/m);
  assert.match(proofRuntimeRows, /^mod receipt;$/m);
  assert.match(proofRuntimeRows, /pub\(super\) use plan::runtime_proof_plan_row/);
  assert.match(proofRuntimeRows, /pub\(super\) use receipt::runtime_proof_receipt_row/);
  assert.doesNotMatch(proofRuntimeRows, /fn runtime_proof_plan_row/);
  assert.doesNotMatch(proofRuntimeRows, /fn runtime_proof_receipt_row/);
  assert.match(proofRuntimePlanRows, /pub\(in super::super\) fn runtime_proof_plan_row/);
  assert.match(proofRuntimePlanRows, /DxRuntimeProofPlanSummary/);
  assert.match(proofRuntimePlanRows, /dx-runtime-proof-latest-plan/);
  assert.match(proofRuntimePlanRows, /runtime_proof_evidence_detail/);
  assert.match(proofRuntimePlanRows, /runtime_proof_requirements_label/);
  assert.match(proofRuntimePlanRows, /minimum_evidence_lines_for_pass/);
  assert.match(proofRuntimeReceiptRows, /pub\(in super::super\) fn runtime_proof_receipt_row/);
  assert.match(proofRuntimeReceiptRows, /DxRuntimeProofReceiptSummary/);
  assert.match(proofRuntimeReceiptRows, /runtime_proof_receipt_state_label/);
  assert.match(proofRuntimeReceiptRows, /evidence_samples\.first/);
  assert.match(proofRuntimeReceiptRows, /final_command/);
  assert.doesNotMatch(proof, /fn runtime_proof_plan_evidence_detail/);
  assert.doesNotMatch(proof, /fn runtime_proof_plan_requirements/);
  assert.match(proofLabels, /#\[path = "proof_labels\/evidence\.rs"\]\s*mod evidence;/);
  assert.match(proofLabels, /#\[path = "proof_labels\/receipt\.rs"\]\s*mod receipt;/);
  assert.match(
    proofLabels,
    /pub\(crate\) use evidence::\{runtime_proof_evidence_detail, runtime_proof_requirements_label\}/,
  );
  assert.match(proofLabels, /pub\(crate\) use receipt::runtime_proof_receipt_state_label/);
  assert.doesNotMatch(proofLabels, /pub\(crate\) fn runtime_proof_evidence_detail/);
  assert.doesNotMatch(proofLabels, /pub\(crate\) fn runtime_proof_requirements_label/);
  assert.doesNotMatch(proofLabels, /pub\(crate\) fn runtime_proof_receipt_state_label/);
  assert.doesNotMatch(proofLabels, /#\[test\]/);
  assert.match(proofLabelEvidence, /pub\(crate\) fn runtime_proof_evidence_detail/);
  assert.match(proofLabelEvidence, /pub\(crate\) fn runtime_proof_requirements_label/);
  assert.match(proofLabelEvidence, /accepted_evidence_examples/);
  assert.match(proofLabelEvidence, /visual proof/);
  assert.match(proofLabelReceipt, /pub\(crate\) fn runtime_proof_receipt_state_label/);
  assert.match(proofLabelReceipt, /Claim-ready/);
  assert.match(proofLabelReceipt, /validation_status\.trim/);
  assert.match(proofLabelTests, /use super::\*/);
  assert.match(proofLabelTests, /runtime_proof_evidence_detail_ignores_blank_examples/);
  assert.match(proofLabelTests, /runtime_proof_requirements_label_keeps_contract_words/);
  assert.match(proofLabelTests, /runtime_proof_receipt_state_label_handles_blank_validation_status/);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/proof.rs") < 90);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/proof/freshness.rs") < 80);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/proof/runtime_rows.rs") < 30);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/proof/runtime_rows/plan.rs") < 95);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/proof/runtime_rows/receipt.rs") < 95);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/proof_labels.rs") < 20);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/proof_labels/evidence.rs") < 60);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/proof_labels/receipt.rs") < 35);
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/proof_labels_tests.rs") < 70);
});
