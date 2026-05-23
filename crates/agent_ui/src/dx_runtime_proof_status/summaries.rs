use super::fields::{
    array_len_at, bool_at, compact_string_array_at, compact_string_at, string_array_at, string_at,
    usize_at,
};
use super::receipts::read_json;
use super::{DxRuntimeProofPlanSummary, DxRuntimeProofReceiptSummary};
use serde_json::Value;
use std::path::Path;

pub(super) fn parse_plan_summary(path: &Path, label: &str) -> Option<DxRuntimeProofPlanSummary> {
    let value = read_json(path)?;
    let plan = value.get("runtime_proof_plan").unwrap_or(&value);
    let request = plan.get("request").unwrap_or(&Value::Null);
    let status = plan.get("status").unwrap_or(&Value::Null);
    let evidence_contract = plan.get("evidence_contract").unwrap_or(&Value::Null);
    let blockers = string_array_at(status, "blockers");
    let checklist_step_count = usize_at(status, "checklist_step_count")
        .max(array_len_at(plan, "checklist"))
        .max(usize_at(&value, "checklist_step_count"));

    Some(DxRuntimeProofPlanSummary {
        label: label.to_string(),
        status: string_at(status, "status")
            .or_else(|| string_at(plan, "status"))
            .unwrap_or_else(|| "unknown".to_string()),
        expected_final_command: string_at(request, "expected_final_command")
            .or_else(|| string_at(evidence_contract, "final_command")),
        checklist_step_count,
        required_step_count: usize_at(status, "required_step_count"),
        minimum_evidence_lines_for_pass: usize_at(
            evidence_contract,
            "minimum_evidence_lines_for_pass",
        ),
        accepted_evidence_examples: string_array_at(
            evidence_contract,
            "accepted_evidence_examples",
        ),
        requires_clean_git: bool_at(request, "require_clean_git"),
        requires_diff_check: bool_at(request, "require_diff_check"),
        requires_visual_evidence: bool_at(request, "require_runtime_visual_evidence"),
        requires_import: bool_at(request, "require_runtime_proof_import")
            || bool_at(
                evidence_contract,
                "runtime_green_claim_requires_import_receipt",
            ),
        blocker_count: usize_at(status, "blocker_count").max(blockers.len()),
        blockers,
        next_action: string_at(plan, "next_action").or_else(|| string_at(&value, "next_action")),
    })
}

pub(super) fn parse_import_summary(
    path: &Path,
    label: &str,
) -> Option<DxRuntimeProofReceiptSummary> {
    let value = read_json(path)?;
    let proof = value.get("runtime_proof").unwrap_or(&value);
    let request = proof.get("request").unwrap_or(proof);
    let validation = proof.get("validation").unwrap_or(&Value::Null);
    let operator_status_copy = proof.get("operator_status_copy").unwrap_or(&Value::Null);

    Some(DxRuntimeProofReceiptSummary {
        label: label.to_string(),
        operator_status: string_at(request, "operator_status")
            .or_else(|| string_at(operator_status_copy, "operator_status"))
            .unwrap_or_else(|| "unknown".to_string()),
        validation_status: string_at(validation, "status").unwrap_or_else(|| "unknown".to_string()),
        runtime_green_candidate: bool_at(validation, "runtime_green_candidate"),
        can_claim_runtime_green: bool_at(operator_status_copy, "can_claim_runtime_green"),
        evidence_count: usize_at(validation, "evidence_count"),
        blocker_count: usize_at(validation, "blocker_count"),
        headline: string_at(operator_status_copy, "headline"),
        proof_summary: compact_string_at(request, "proof_summary"),
        final_command: compact_string_at(request, "final_command"),
        source: compact_string_at(request, "source"),
        evidence_samples: compact_string_array_at(request, "evidence", 3),
        blockers: string_array_at(validation, "blockers"),
    })
}

pub(super) fn parse_status_summary(
    path: &Path,
    label: &str,
) -> Option<DxRuntimeProofReceiptSummary> {
    let value = read_json(path)?;
    let status_copy = value.get("operator_status_copy").unwrap_or(&Value::Null);
    let validation = value.get("validation").unwrap_or(&Value::Null);
    let request = value.get("request").unwrap_or(&Value::Null);

    Some(DxRuntimeProofReceiptSummary {
        label: label.to_string(),
        operator_status: string_at(status_copy, "operator_status").unwrap_or_else(|| {
            string_at(validation, "operator_status").unwrap_or_else(|| "unknown".to_string())
        }),
        validation_status: string_at(validation, "status").unwrap_or_else(|| "unknown".to_string()),
        runtime_green_candidate: bool_at(validation, "runtime_green_candidate"),
        can_claim_runtime_green: bool_at(status_copy, "can_claim_runtime_green"),
        evidence_count: usize_at(validation, "evidence_count"),
        blocker_count: usize_at(validation, "blocker_count"),
        headline: string_at(status_copy, "headline"),
        proof_summary: compact_string_at(request, "proof_summary"),
        final_command: compact_string_at(request, "final_command"),
        source: compact_string_at(request, "source"),
        evidence_samples: compact_string_array_at(request, "evidence", 3),
        blockers: string_array_at(validation, "blockers"),
    })
}
