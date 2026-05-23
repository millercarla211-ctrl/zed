use serde_json::Value;
use std::{
    fs::File,
    io::{Read, Result},
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::dx_deploy_check_roots::check_receipt_roots;
use crate::dx_deploy_launch_actions::{DxDeployLaunchAction, launch_actions};
use crate::dx_deploy_launch_approval_evidence::{
    DxDeployLaunchApprovalEvidence, launch_approval_evidence,
};
use crate::dx_deploy_launch_buckets::{DxDeployLaunchBucket, launch_buckets};
use crate::dx_deploy_launch_evidence::{
    DxDeployLaunchChain, DxDeployLaunchEvidenceSource, launch_chain, launch_evidence_sources,
};
use crate::dx_deploy_launch_notices::{DxDeployLaunchGateNotice, notice_rows};
use crate::dx_deploy_launch_outcome::{DxDeployLaunchOutcome, launch_outcome};
use crate::dx_deploy_launch_scope::{DxDeployLaunchScope, launch_scope};
use crate::dx_deploy_receipt_fields::{
    array_len, bool_field, first_string_array_item, string_field, usize_field,
};

const MAX_CHECK_RECEIPT_BYTES: u64 = 256 * 1024;

#[derive(Clone, Default)]
pub(crate) struct DxDeployLaunchGateSnapshot {
    pub receipt_found: bool,
    pub label: String,
    pub schema_version: Option<String>,
    pub command: Option<String>,
    pub status: Option<String>,
    pub score: Option<usize>,
    pub max_score: Option<usize>,
    pub score_estimated: Option<bool>,
    pub outcome: DxDeployLaunchOutcome,
    pub scope: DxDeployLaunchScope,
    pub buckets: Vec<DxDeployLaunchBucket>,
    pub approval_evidence: DxDeployLaunchApprovalEvidence,
    pub source_status: Option<String>,
    pub source_approved: Option<bool>,
    pub runtime_status: Option<String>,
    pub runtime_approved: Option<bool>,
    pub launch_status: Option<String>,
    pub launch_approved: Option<bool>,
    pub blocker_count: usize,
    pub warning_count: usize,
    pub blockers: Vec<DxDeployLaunchGateNotice>,
    pub warnings: Vec<DxDeployLaunchGateNotice>,
    pub quick_actions: Vec<DxDeployLaunchAction>,
    pub quick_action_count: usize,
    pub evidence_sources: Vec<DxDeployLaunchEvidenceSource>,
    pub chain: Option<DxDeployLaunchChain>,
    pub next_action: Option<String>,
}

struct LaunchGateCandidate {
    root_rank: u8,
    file_rank: u8,
    modified: SystemTime,
    label: String,
    path: PathBuf,
}

pub(crate) fn deploy_launch_gate_snapshot(
    workspace_roots: &[PathBuf],
) -> DxDeployLaunchGateSnapshot {
    let mut candidates = Vec::new();

    for root in check_receipt_roots(workspace_roots) {
        push_check_candidates(&mut candidates, root.path, root.label, root.root_rank);
    }

    candidates.sort_by(|left, right| {
        left.root_rank
            .cmp(&right.root_rank)
            .then_with(|| left.file_rank.cmp(&right.file_rank))
            .then_with(|| {
                right
                    .modified
                    .partial_cmp(&left.modified)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    candidates
        .iter()
        .find_map(parse_launch_gate_candidate)
        .unwrap_or_else(|| DxDeployLaunchGateSnapshot {
            label: "No dx-check launch receipt".to_string(),
            ..Default::default()
        })
}

fn push_check_candidates(
    candidates: &mut Vec<LaunchGateCandidate>,
    root: PathBuf,
    root_label: String,
    rank: u8,
) {
    for (file_rank, file_name) in ["check-launch-latest.json", "check-latest.json"]
        .into_iter()
        .enumerate()
    {
        let path = root.join(file_name);
        if !path.is_file() {
            continue;
        }

        let modified = path
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        candidates.push(LaunchGateCandidate {
            root_rank: rank,
            file_rank: u8::try_from(file_rank).unwrap_or(u8::MAX),
            modified,
            label: format!("{root_label}\\{file_name}"),
            path,
        });
    }
}

fn parse_launch_gate_candidate(
    candidate: &LaunchGateCandidate,
) -> Option<DxDeployLaunchGateSnapshot> {
    let receipt = read_json(&candidate.path)?;
    let zed = receipt.get("zed");
    let source_ready = receipt.get("source_ready");
    let runtime_approved = receipt.get("runtime_approved");
    let launch_approved = receipt.get("launch_approved");
    let quick_actions = launch_actions(zed.and_then(|value| value.get("quick_fixes")));
    let quick_action_count = zed
        .and_then(|value| usize_field(value, "quick_fix_count"))
        .unwrap_or(quick_actions.len());

    Some(DxDeployLaunchGateSnapshot {
        receipt_found: true,
        label: candidate.label.clone(),
        schema_version: zed
            .and_then(|value| string_field(value, "schema_version"))
            .or_else(|| string_field(&receipt, "schema_version")),
        command: string_field(&receipt, "command"),
        status: zed
            .and_then(|value| string_field(value, "status"))
            .or_else(|| string_field(&receipt, "status")),
        score: zed
            .and_then(|value| usize_field(value, "score_value"))
            .or_else(|| usize_field(&receipt, "score")),
        max_score: zed
            .and_then(|value| usize_field(value, "score_max"))
            .or_else(|| usize_field(&receipt, "max_score")),
        score_estimated: zed
            .and_then(|value| bool_field(value, "score_estimated"))
            .or_else(|| bool_field(&receipt, "score_estimated")),
        outcome: launch_outcome(&receipt),
        scope: launch_scope(&receipt),
        buckets: launch_buckets(&receipt),
        approval_evidence: launch_approval_evidence(&receipt),
        source_status: source_ready.and_then(|value| string_field(value, "status")),
        source_approved: source_ready.and_then(|value| bool_field(value, "approved")),
        runtime_status: runtime_approved.and_then(|value| string_field(value, "status")),
        runtime_approved: runtime_approved.and_then(|value| bool_field(value, "approved")),
        launch_status: launch_approved.and_then(|value| string_field(value, "status")),
        launch_approved: launch_approved.and_then(|value| bool_field(value, "approved")),
        blocker_count: zed
            .and_then(|value| usize_field(value, "blocker_count"))
            .unwrap_or_else(|| array_len(zed.unwrap_or(&receipt), "blockers")),
        warning_count: zed
            .and_then(|value| usize_field(value, "warning_count"))
            .unwrap_or_else(|| array_len(zed.unwrap_or(&receipt), "warnings")),
        blockers: notice_rows(zed.and_then(|value| value.get("blockers"))),
        warnings: notice_rows(zed.and_then(|value| value.get("warnings"))),
        quick_actions,
        quick_action_count,
        evidence_sources: launch_evidence_sources(&receipt),
        chain: launch_chain(&receipt),
        next_action: first_string_array_item(&receipt, "next_actions")
            .or_else(|| launch_approved.and_then(|value| string_field(value, "next_action"))),
    })
}

fn read_json(path: &Path) -> Option<Value> {
    let mut file = File::open(path).ok()?;
    let mut buffer = Vec::new();
    read_limited(&mut file, &mut buffer).ok()?;
    serde_json::from_slice(&buffer).ok()
}

fn read_limited(file: &mut File, buffer: &mut Vec<u8>) -> Result<usize> {
    file.by_ref()
        .take(MAX_CHECK_RECEIPT_BYTES)
        .read_to_end(buffer)
}
