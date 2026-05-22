use serde_json::Value;
use std::{
    cmp::Ordering,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::dx_deploy_receipt_rank::{
    DxDeployReceiptSourceKind, command_receipt_source_rank, compare_rank_then_newest,
    provider_gate_receipt_source_rank,
};
pub(crate) use crate::dx_deploy_receipt_summary::{
    DxDeployCapabilityRow, DxDeployCommandReceiptSummary, DxDeployProviderGateReceiptSummary,
};
use crate::dx_deploy_receipt_summary::{
    deploy_provider_rows_from_value, parse_deploy_command_receipt,
    parse_deploy_provider_gate_receipt,
};

const MAX_DEPLOY_RECEIPT_BYTES: u64 = 256 * 1024;
const DX_HUB_DEPLOY_RECEIPT_ROOT: &str = r"G:\Dx\.dx\receipts\deploy";
const DX_CLI_DEPLOY_RECEIPT_ROOT: &str = r"G:\Dx\cli\.dx\receipts\deploy";
const DX_WWW_DEPLOY_RECEIPT_ROOT: &str = r"G:\Dx\www\.dx\receipts\deploy";

#[derive(Clone, Default)]
pub(crate) struct DxDeployCapabilityMatrixSnapshot {
    pub root_exists: bool,
    pub receipt_count: usize,
    pub latest_receipts: Vec<String>,
    pub plan: Option<DxDeployCommandReceiptSummary>,
    pub status: Option<DxDeployCommandReceiptSummary>,
    pub provider_gate: Option<DxDeployProviderGateReceiptSummary>,
    pub providers: Vec<DxDeployCapabilityRow>,
}

struct DeployReceiptRoot {
    path: PathBuf,
    label: String,
    source_kind: DxDeployReceiptSourceKind,
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

    let plan = plan_receipts.first().and_then(parse_command_receipt);
    let status = status_receipts.first().and_then(parse_command_receipt);
    let provider_gate = provider_gate_receipts
        .first()
        .and_then(parse_provider_gate_receipt);
    let providers = status_receipts
        .first()
        .and_then(parse_provider_rows)
        .or_else(|| plan_receipts.first().and_then(parse_provider_rows))
        .or_else(|| matrix_receipts.first().and_then(parse_provider_rows))
        .unwrap_or_default();

    DxDeployCapabilityMatrixSnapshot {
        root_exists,
        receipt_count,
        latest_receipts: latest_receipts
            .into_iter()
            .take(4)
            .map(|candidate| candidate.label)
            .collect(),
        plan,
        status,
        provider_gate,
        providers,
    }
}

fn deploy_receipt_roots(workspace_roots: &[PathBuf]) -> Vec<DeployReceiptRoot> {
    let mut roots = Vec::new();

    for root in workspace_roots.iter().take(4) {
        push_receipt_root(
            &mut roots,
            root.join(".dx").join("receipts").join("deploy"),
            format!("{}\\.dx\\receipts\\deploy", root.display()),
            DxDeployReceiptSourceKind::Workspace,
        );
    }

    push_receipt_root(
        &mut roots,
        PathBuf::from(DX_HUB_DEPLOY_RECEIPT_ROOT),
        DX_HUB_DEPLOY_RECEIPT_ROOT.to_string(),
        DxDeployReceiptSourceKind::DxHub,
    );
    push_receipt_root(
        &mut roots,
        PathBuf::from(DX_CLI_DEPLOY_RECEIPT_ROOT),
        DX_CLI_DEPLOY_RECEIPT_ROOT.to_string(),
        DxDeployReceiptSourceKind::DxCli,
    );
    push_receipt_root(
        &mut roots,
        PathBuf::from(DX_WWW_DEPLOY_RECEIPT_ROOT),
        DX_WWW_DEPLOY_RECEIPT_ROOT.to_string(),
        DxDeployReceiptSourceKind::DxWww,
    );

    roots
}

fn push_receipt_root(
    roots: &mut Vec<DeployReceiptRoot>,
    path: PathBuf,
    label: String,
    source_kind: DxDeployReceiptSourceKind,
) {
    if roots.iter().any(|root| root.path == path) {
        return;
    }

    roots.push(DeployReceiptRoot {
        path,
        label,
        source_kind,
    });
}

fn receipt_candidates(root: &DeployReceiptRoot) -> Vec<DeployReceiptCandidate> {
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
) -> Option<DxDeployCommandReceiptSummary> {
    let value = read_json(&candidate.path)?;
    parse_deploy_command_receipt(candidate.label.clone(), &value)
}

fn parse_provider_rows(candidate: &DeployReceiptCandidate) -> Option<Vec<DxDeployCapabilityRow>> {
    let value = read_json(&candidate.path)?;
    let rows = deploy_provider_rows_from_value(&value);
    if rows.is_empty() { None } else { Some(rows) }
}

fn parse_provider_gate_receipt(
    candidate: &DeployReceiptCandidate,
) -> Option<DxDeployProviderGateReceiptSummary> {
    let value = read_json(&candidate.path)?;
    parse_deploy_provider_gate_receipt(candidate.label.clone(), &value)
}

fn read_json(path: &Path) -> Option<Value> {
    let mut file = File::open(path).ok()?;
    let mut buffer = Vec::new();
    file.by_ref()
        .take(MAX_DEPLOY_RECEIPT_BYTES)
        .read_to_end(&mut buffer)
        .ok()?;
    serde_json::from_slice(&buffer).ok()
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
