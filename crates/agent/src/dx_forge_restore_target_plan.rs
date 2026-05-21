use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(crate) const DX_FORGE_RESTORE_TARGET_PLAN_SCHEMA: &str = "zed.dx.forge.restore_target_plan.v1";
pub(crate) const DX_FORGE_RESTORE_TARGET_PLAN_RECEIPT_SCHEMA: &str =
    "zed.dx.forge.restore_target_plan_receipt.v1";
const DX_FORGE_RESTORE_APPROVAL_SCHEMA: &str = "zed.dx.forge.restore_approval.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxForgeRestoreTargetPlanRequest {
    pub restore_approval: Value,
    pub target_path: Option<String>,
    pub require_approval_ready: bool,
    pub require_rollback_verified: bool,
    pub require_preview_exists: bool,
    pub root_mode: String,
    pub generated_at_ms: u128,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreTargetPlan {
    pub schema: &'static str,
    pub generated_at_ms: u128,
    pub request: DxForgeRestoreTargetPlanRequestSummary,
    pub approval: DxForgeRestoreTargetPlanApprovalSummary,
    pub target: DxForgeRestoreTargetPlanTargetSummary,
    pub validation: DxForgeRestoreTargetPlanValidation,
    pub checklist: Vec<DxForgeRestoreTargetPlanStep>,
    pub plan_receipt: Option<DxForgeRestoreTargetPlanReceipt>,
    pub safety: DxForgeRestoreTargetPlanSafety,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreTargetPlanRequestSummary {
    pub target_path: String,
    pub target_path_source: &'static str,
    pub require_approval_ready: bool,
    pub require_rollback_verified: bool,
    pub require_preview_exists: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreTargetPlanApprovalSummary {
    pub schema: String,
    pub validation_status: String,
    pub approval_ready: bool,
    pub receipt_usable_for_restore_to_target: bool,
    pub rollback_verified: bool,
    pub overwrite_approved: bool,
    pub evidence_count: usize,
    pub blocker_count: usize,
    pub target_path: String,
    pub restore_destination_root: String,
    pub backup_archive_path: String,
    pub backup_manifest_path: String,
    pub restored_file_count: usize,
    pub restored_directory_count: usize,
    pub restored_total_file_bytes: u64,
    pub verified_root_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreTargetPlanTargetSummary {
    pub path: String,
    pub exists: bool,
    pub kind: &'static str,
    pub parent_path: Option<String>,
    pub parent_exists: bool,
    pub read_only: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreTargetPlanValidation {
    pub status: &'static str,
    pub plan_ready: bool,
    pub blocker_count: usize,
    pub blockers: Vec<String>,
    pub warning_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreTargetPlanStep {
    pub order: usize,
    pub label: &'static str,
    pub required: bool,
    pub status: &'static str,
    pub evidence_required: &'static str,
    pub future_action: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreTargetPlanSafety {
    pub writes_plan_receipt_only: bool,
    pub mutates_target_path: bool,
    pub overwrites_target_files: bool,
    pub deletes_files: bool,
    pub runs_shell: bool,
    pub runs_external_processes: bool,
    pub runs_forge_binary: bool,
    pub runs_zstd_binary: bool,
    pub starts_local_servers: bool,
    pub dispatches_browser_input: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreTargetPlanReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub plan_ready: bool,
    pub target_path: String,
    pub restore_destination_root: String,
    pub blocker_count: usize,
    pub next_action: String,
}

pub(crate) fn build_dx_forge_restore_target_plan(
    request: DxForgeRestoreTargetPlanRequest,
) -> DxForgeRestoreTargetPlan {
    let approval_value = decode_json_string(&request.restore_approval);
    let restore_approval = approval_value
        .as_ref()
        .ok()
        .and_then(locate_restore_approval);
    let approval = restore_approval
        .map(summarize_restore_approval)
        .unwrap_or_else(DxForgeRestoreTargetPlanApprovalSummary::missing);
    let requested_target = clean_optional_text(request.target_path);
    let target_path_source = if requested_target.is_some() {
        "input"
    } else if !approval.target_path.is_empty() {
        "restore_approval"
    } else {
        "missing"
    };
    let target_path = requested_target
        .clone()
        .or_else(|| clean_optional_text(Some(approval.target_path.clone())))
        .unwrap_or_default();
    let target = inspect_target(&target_path);
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if approval_value.is_err() {
        blockers.push("Restore approval input could not be parsed as JSON.".to_string());
    }
    if restore_approval.is_none() {
        blockers.push(
            "Restore target plan needs a zed.dx.forge.restore_approval.v1 object or receipt."
                .to_string(),
        );
    }
    if request.require_approval_ready && !approval.approval_ready {
        blockers.push("Restore approval receipt is not approval-ready.".to_string());
    }
    if requested_target
        .as_ref()
        .is_some_and(|target| !approval.target_path.is_empty() && target != &approval.target_path)
    {
        blockers.push(
            "Input target path differs from the target path captured in the approval receipt."
                .to_string(),
        );
    }
    if !approval.receipt_usable_for_restore_to_target {
        blockers.push("Restore approval receipt is not usable for restore-to-target.".to_string());
    }
    if request.require_rollback_verified && !approval.rollback_verified {
        blockers.push("Rollback evidence is not verified in the approval receipt.".to_string());
    }
    if target_path.is_empty() {
        blockers.push("Restore target plan needs a live target path.".to_string());
    }
    if approval.backup_archive_path.is_empty() || approval.backup_manifest_path.is_empty() {
        blockers.push("Restore target plan needs backup archive and manifest paths.".to_string());
    }
    if approval.restore_destination_root.is_empty() {
        blockers.push("Restore target plan needs a managed restore preview path.".to_string());
    } else if request.require_preview_exists
        && !Path::new(&approval.restore_destination_root).is_dir()
    {
        blockers.push("Managed restore preview path is missing on disk.".to_string());
    }
    if target.exists {
        warnings.push(format!("Target path already exists as {}.", target.kind));
        if !approval.overwrite_approved {
            blockers.push(
                "Target path exists; approval receipt does not explicitly approve overwrite risk."
                    .to_string(),
            );
        }
    } else if !target.parent_exists {
        blockers.push("Target parent directory is missing.".to_string());
    }
    if target.read_only {
        warnings.push("Target path is read-only.".to_string());
    }
    if approval.blocker_count > 0 {
        blockers.push(format!(
            "Restore approval receipt still reports {} blocker(s).",
            approval.blocker_count
        ));
    }

    let plan_ready = blockers.is_empty();
    let status = if plan_ready {
        "restore_target_plan_ready"
    } else {
        "restore_target_plan_blocked"
    };
    let next_action = if plan_ready {
        "Review this dry-run plan in Forge history, then request an explicit governed restore-to-target executor only inside the approved mutation window."
            .to_string()
    } else {
        "Resolve the listed blockers before any restore-to-target executor can be considered."
            .to_string()
    };

    DxForgeRestoreTargetPlan {
        schema: DX_FORGE_RESTORE_TARGET_PLAN_SCHEMA,
        generated_at_ms: request.generated_at_ms,
        request: DxForgeRestoreTargetPlanRequestSummary {
            target_path: target_path.clone(),
            target_path_source,
            require_approval_ready: request.require_approval_ready,
            require_rollback_verified: request.require_rollback_verified,
            require_preview_exists: request.require_preview_exists,
            root_mode: request.root_mode,
        },
        approval,
        target,
        validation: DxForgeRestoreTargetPlanValidation {
            status,
            plan_ready,
            blocker_count: blockers.len(),
            blockers,
            warning_count: warnings.len(),
            warnings,
        },
        checklist: restore_target_checklist(),
        plan_receipt: None,
        safety: DxForgeRestoreTargetPlanSafety {
            writes_plan_receipt_only: true,
            mutates_target_path: false,
            overwrites_target_files: false,
            deletes_files: false,
            runs_shell: false,
            runs_external_processes: false,
            runs_forge_binary: false,
            runs_zstd_binary: false,
            starts_local_servers: false,
            dispatches_browser_input: false,
        },
        next_action,
    }
}

impl DxForgeRestoreTargetPlanApprovalSummary {
    fn missing() -> Self {
        Self {
            schema: "missing".to_string(),
            validation_status: "missing".to_string(),
            approval_ready: false,
            receipt_usable_for_restore_to_target: false,
            rollback_verified: false,
            overwrite_approved: false,
            evidence_count: 0,
            blocker_count: 0,
            target_path: String::new(),
            restore_destination_root: String::new(),
            backup_archive_path: String::new(),
            backup_manifest_path: String::new(),
            restored_file_count: 0,
            restored_directory_count: 0,
            restored_total_file_bytes: 0,
            verified_root_sha256: String::new(),
        }
    }
}

fn summarize_restore_approval(value: &Value) -> DxForgeRestoreTargetPlanApprovalSummary {
    DxForgeRestoreTargetPlanApprovalSummary {
        schema: string_field(value, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        validation_status: string_field(value, &["validation", "status"])
            .unwrap_or_else(|| "unknown".to_string()),
        approval_ready: bool_field(value, &["validation", "approval_ready"]).unwrap_or(false),
        receipt_usable_for_restore_to_target: bool_field(
            value,
            &["validation", "receipt_usable_for_restore_to_target"],
        )
        .unwrap_or(false),
        rollback_verified: bool_field(value, &["request", "rollback_verified"]).unwrap_or(false),
        overwrite_approved: bool_field(value, &["request", "overwrite_approved"]).unwrap_or(false),
        evidence_count: usize_field(value, &["validation", "evidence_count"]).unwrap_or(0),
        blocker_count: usize_field(value, &["validation", "blocker_count"]).unwrap_or(0),
        target_path: string_field(value, &["request", "target_path"]).unwrap_or_default(),
        restore_destination_root: string_field(value, &["restore", "restore_destination_root"])
            .unwrap_or_default(),
        backup_archive_path: string_field(value, &["restore", "backup_archive_path"])
            .unwrap_or_default(),
        backup_manifest_path: string_field(value, &["restore", "backup_manifest_path"])
            .unwrap_or_default(),
        restored_file_count: usize_field(value, &["restore", "restored_file_count"]).unwrap_or(0),
        restored_directory_count: usize_field(value, &["restore", "restored_directory_count"])
            .unwrap_or(0),
        restored_total_file_bytes: u64_field(value, &["restore", "restored_total_file_bytes"])
            .unwrap_or(0),
        verified_root_sha256: string_field(value, &["restore", "verified_root_sha256"])
            .unwrap_or_default(),
    }
}

fn inspect_target(target_path: &str) -> DxForgeRestoreTargetPlanTargetSummary {
    let path = PathBuf::from(target_path);
    let metadata = (!target_path.is_empty())
        .then(|| std::fs::metadata(&path))
        .and_then(Result::ok);
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());

    DxForgeRestoreTargetPlanTargetSummary {
        path: target_path.to_string(),
        exists: metadata.is_some(),
        kind: metadata
            .as_ref()
            .map(|metadata| {
                if metadata.is_dir() {
                    "directory"
                } else if metadata.is_file() {
                    "file"
                } else {
                    "other"
                }
            })
            .unwrap_or("missing"),
        parent_path: parent.map(path_string),
        parent_exists: parent.map(Path::exists).unwrap_or(false),
        read_only: metadata
            .map(|metadata| metadata.permissions().readonly())
            .unwrap_or(false),
    }
}

fn restore_target_checklist() -> Vec<DxForgeRestoreTargetPlanStep> {
    vec![
        DxForgeRestoreTargetPlanStep {
            order: 1,
            label: "Confirm managed restore preview",
            required: true,
            status: "manual_required",
            evidence_required: "Restore preview path and verified root hash.",
            future_action: "Compare preview contents against the requested target.",
        },
        DxForgeRestoreTargetPlanStep {
            order: 2,
            label: "Confirm rollback posture",
            required: true,
            status: "manual_required",
            evidence_required: "Backup archive, manifest, and rollback note.",
            future_action: "Keep rollback evidence visible before mutation.",
        },
        DxForgeRestoreTargetPlanStep {
            order: 3,
            label: "Inspect live target",
            required: true,
            status: "manual_required",
            evidence_required: "Target existence, parent directory, and overwrite posture.",
            future_action: "Stop if the live target differs from the approval receipt.",
        },
        DxForgeRestoreTargetPlanStep {
            order: 4,
            label: "Request governed mutation window",
            required: true,
            status: "manual_required",
            evidence_required: "Explicit user approval for the future restore-to-target executor.",
            future_action: "Only a separate governed executor may mutate the target.",
        },
    ]
}

fn locate_restore_approval(value: &Value) -> Option<&Value> {
    if value.get("schema").and_then(Value::as_str) == Some(DX_FORGE_RESTORE_APPROVAL_SCHEMA) {
        return Some(value);
    }

    value.get("restore_approval").filter(|approval| {
        approval.get("schema").and_then(Value::as_str) == Some(DX_FORGE_RESTORE_APPROVAL_SCHEMA)
    })
}

fn decode_json_string(value: &Value) -> Result<Value, String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if text.starts_with('{') {
            return serde_json::from_str(text).map_err(|error| {
                format!("Failed to parse stringified DX Forge restore approval JSON: {error}")
            });
        }
    }

    Ok(value.clone())
}

fn clean_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

fn string_field(value: &Value, path: &[&str]) -> Option<String> {
    value_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn bool_field(value: &Value, path: &[&str]) -> Option<bool> {
    value_at(value, path).and_then(Value::as_bool)
}

fn u64_field(value: &Value, path: &[&str]) -> Option<u64> {
    value_at(value, path).and_then(Value::as_u64)
}

fn usize_field(value: &Value, path: &[&str]) -> Option<usize> {
    u64_field(value, path).and_then(|value| usize::try_from(value).ok())
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}
