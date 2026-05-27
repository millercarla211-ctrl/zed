use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::dx_deploy_invalid_receipts::{
    DxDeployInvalidReceipt, note_invalid_receipt, read_deploy_receipt_json,
};
use crate::dx_deploy_provider_gate_summary::parse_deploy_provider_gate_receipt;
pub(crate) use crate::dx_deploy_provider_gate_summary::{
    DxDeployProviderGateQuickFix, DxDeployProviderGateReceiptSummary, DxDeployProviderGateRow,
};
use crate::dx_deploy_receipt_rank::{
    DxDeployReceiptSourceKind, command_receipt_source_rank, compare_rank_then_newest,
    provider_gate_receipt_source_rank,
};
use crate::dx_deploy_receipt_roots::{DxDeployReceiptRoot, deploy_receipt_roots};
pub(crate) use crate::dx_deploy_receipt_summary::{
    DxDeployCapabilityRow, DxDeployCommandReceiptSummary,
};
use crate::dx_deploy_receipt_summary::{
    deploy_provider_rows_from_value, parse_deploy_command_receipt,
};

#[derive(Clone, Default)]
pub(crate) struct DxDeployCapabilityMatrixSnapshot {
    pub root_exists: bool,
    pub receipt_count: usize,
    pub latest_receipts: Vec<String>,
    pub invalid_receipts: Vec<DxDeployInvalidReceipt>,
    pub plan: Option<DxDeployCommandReceiptSummary>,
    pub status: Option<DxDeployCommandReceiptSummary>,
    pub provider_gate: Option<DxDeployProviderGateReceiptSummary>,
    pub providers: Vec<DxDeployCapabilityRow>,
}

#[derive(Clone)]
struct DeployReceiptCandidate {
    modified: SystemTime,
    label: String,
    path: PathBuf,
    source_kind: DxDeployReceiptSourceKind,
}

pub(crate) fn deploy_capability_matrix_snapshot(
    workspace_roots: &[PathBuf],
) -> DxDeployCapabilityMatrixSnapshot {
    let roots = deploy_receipt_roots(workspace_roots);
    let mut root_exists = false;
    let mut receipt_count = 0;
    let mut latest_receipts = Vec::new();
    let mut plan_receipts = Vec::new();
    let mut status_receipts = Vec::new();
    let mut provider_gate_receipts = Vec::new();
    let mut matrix_receipts = Vec::new();
    let mut invalid_receipts = Vec::new();

    for root in roots {
        if root.path.is_dir() {
            root_exists = true;
        }

        let candidates = receipt_candidates(&root);
        receipt_count += candidates.len();
        for candidate in candidates {
            let file_name = candidate
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string();
            if is_deploy_plan_file(&file_name) {
                plan_receipts.push(candidate.clone());
            }
            if is_deploy_status_file(&file_name) {
                status_receipts.push(candidate.clone());
            }
            if is_provider_gate_file(&file_name) {
                provider_gate_receipts.push(candidate.clone());
            }
            if file_name == "provider-capability-matrix.json" {
                matrix_receipts.push(candidate.clone());
            }
            latest_receipts.push(candidate);
        }
    }

    sort_newest_first(&mut latest_receipts);
    sort_command_receipts(&mut plan_receipts);
    sort_command_receipts(&mut status_receipts);
    sort_provider_gate_receipts(&mut provider_gate_receipts);
    sort_command_receipts(&mut matrix_receipts);

    let plan = plan_receipts
        .first()
        .and_then(|candidate| parse_command_receipt(candidate, &mut invalid_receipts));
    let status = status_receipts
        .first()
        .and_then(|candidate| parse_command_receipt(candidate, &mut invalid_receipts));
    let provider_gate = provider_gate_receipts
        .first()
        .and_then(|candidate| parse_provider_gate_receipt(candidate, &mut invalid_receipts));
    let providers = parse_first_provider_rows(&status_receipts, &mut invalid_receipts)
        .or_else(|| parse_first_provider_rows(&plan_receipts, &mut invalid_receipts))
        .or_else(|| parse_first_provider_rows(&matrix_receipts, &mut invalid_receipts))
        .unwrap_or_default();

    DxDeployCapabilityMatrixSnapshot {
        root_exists,
        receipt_count,
        latest_receipts: latest_receipts
            .into_iter()
            .take(4)
            .map(|candidate| candidate.label)
            .collect(),
        invalid_receipts,
        plan,
        status,
        provider_gate,
        providers,
    }
}

fn receipt_candidates(root: &DxDeployReceiptRoot) -> Vec<DeployReceiptCandidate> {
    let Ok(entries) = fs::read_dir(&root.path) else {
        return Vec::new();
    };

    let mut receipts = Vec::new();
    for entry in entries.flatten().take(128) {
        let path = entry.path();
        if !path.is_file() || !is_json_file(&path) {
            continue;
        }

        let modified = path
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let label = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| format!("{}\\{}", root.label, name))
            .unwrap_or_else(|| path.display().to_string());

        receipts.push(DeployReceiptCandidate {
            modified,
            label,
            path,
            source_kind: root.source_kind,
        });
    }

    receipts
}

fn parse_command_receipt(
    candidate: &DeployReceiptCandidate,
    invalid_receipts: &mut Vec<DxDeployInvalidReceipt>,
) -> Option<DxDeployCommandReceiptSummary> {
    let value = match read_deploy_receipt_json(&candidate.path) {
        Ok(value) => value,
        Err(error) => {
            note_invalid_receipt(invalid_receipts, &candidate.label, error);
            return None;
        }
    };
    parse_deploy_command_receipt(candidate.label.clone(), &value)
}

fn parse_first_provider_rows(
    candidates: &[DeployReceiptCandidate],
    invalid_receipts: &mut Vec<DxDeployInvalidReceipt>,
) -> Option<Vec<DxDeployCapabilityRow>> {
    let candidate = candidates.first()?;
    parse_provider_rows(candidate, invalid_receipts)
}

fn parse_provider_rows(
    candidate: &DeployReceiptCandidate,
    invalid_receipts: &mut Vec<DxDeployInvalidReceipt>,
) -> Option<Vec<DxDeployCapabilityRow>> {
    let value = match read_deploy_receipt_json(&candidate.path) {
        Ok(value) => value,
        Err(error) => {
            note_invalid_receipt(invalid_receipts, &candidate.label, error);
            return None;
        }
    };
    let rows = deploy_provider_rows_from_value(&value);
    if rows.is_empty() { None } else { Some(rows) }
}

fn parse_provider_gate_receipt(
    candidate: &DeployReceiptCandidate,
    invalid_receipts: &mut Vec<DxDeployInvalidReceipt>,
) -> Option<DxDeployProviderGateReceiptSummary> {
    let value = match read_deploy_receipt_json(&candidate.path) {
        Ok(value) => value,
        Err(error) => {
            note_invalid_receipt(invalid_receipts, &candidate.label, error);
            return None;
        }
    };
    parse_deploy_provider_gate_receipt(candidate.label.clone(), &value)
}

fn sort_newest_first(candidates: &mut [DeployReceiptCandidate]) {
    candidates.sort_by(|left, right| {
        right
            .modified
            .partial_cmp(&left.modified)
            .unwrap_or(Ordering::Equal)
    });
}

fn sort_command_receipts(candidates: &mut [DeployReceiptCandidate]) {
    candidates.sort_by(|left, right| {
        compare_rank_then_newest(
            command_receipt_source_rank(left.source_kind),
            left.modified,
            command_receipt_source_rank(right.source_kind),
            right.modified,
        )
    });
}

fn sort_provider_gate_receipts(candidates: &mut [DeployReceiptCandidate]) {
    candidates.sort_by(|left, right| {
        compare_rank_then_newest(
            provider_gate_receipt_source_rank(left.source_kind),
            left.modified,
            provider_gate_receipt_source_rank(right.source_kind),
            right.modified,
        )
    });
}

fn is_deploy_plan_file(name: &str) -> bool {
    name == "deploy-plan-latest.json"
        || (name.starts_with("deploy-plan-") && name.ends_with(".json"))
}

fn is_deploy_status_file(name: &str) -> bool {
    name == "deploy-status-latest.json"
        || (name.starts_with("deploy-status-") && name.ends_with(".json"))
}

fn is_provider_gate_file(name: &str) -> bool {
    name.starts_with("deploy-")
        && name.ends_with(".json")
        && !is_deploy_plan_file(name)
        && !is_deploy_status_file(name)
        && name != "provider-capability-matrix.json"
}

fn is_json_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json")
    )
}
