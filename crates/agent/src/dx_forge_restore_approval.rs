use serde::Serialize;
use serde_json::Value;

pub(crate) const DX_FORGE_RESTORE_APPROVAL_SCHEMA: &str = "zed.dx.forge.restore_approval.v1";
pub(crate) const DX_FORGE_RESTORE_APPROVAL_RECEIPT_SCHEMA: &str =
    "zed.dx.forge.restore_approval_receipt.v1";
const DX_FORGE_RESTORE_EXECUTION_SCHEMA: &str = "zed.dx.forge.restore_execution.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxForgeRestoreApprovalRequest {
    pub restore_execution: Value,
    pub target_path: Option<String>,
    pub operator_approval: bool,
    pub rollback_verified: bool,
    pub overwrite_approved: bool,
    pub evidence: Vec<String>,
    pub blockers: Vec<String>,
    pub approval_note: Option<String>,
    pub root_mode: String,
    pub generated_at_ms: u128,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreApproval {
    pub schema: &'static str,
    pub generated_at_ms: u128,
    pub request: DxForgeRestoreApprovalRequestSummary,
    pub restore: DxForgeRestoreApprovalRestoreSummary,
    pub validation: DxForgeRestoreApprovalValidation,
    pub approval_receipt: Option<DxForgeRestoreApprovalReceipt>,
    pub safety: DxForgeRestoreApprovalSafety,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreApprovalRequestSummary {
    pub target_path: String,
    pub target_path_source: &'static str,
    pub operator_approval: bool,
    pub rollback_verified: bool,
    pub overwrite_approved: bool,
    pub evidence: Vec<String>,
    pub blockers: Vec<String>,
    pub approval_note: Option<String>,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreApprovalRestoreSummary {
    pub schema: String,
    pub status: String,
    pub backup_target_path: String,
    pub restore_destination_root: String,
    pub backup_archive_path: String,
    pub backup_manifest_path: String,
    pub restored_file_count: usize,
    pub restored_directory_count: usize,
    pub restored_total_file_bytes: u64,
    pub verified_root_sha256: String,
    pub target_mutation_applied: bool,
    pub overwrote_existing_files: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreApprovalValidation {
    pub status: &'static str,
    pub approval_ready: bool,
    pub receipt_usable_for_restore_to_target: bool,
    pub evidence_count: usize,
    pub blocker_count: usize,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreApprovalSafety {
    pub writes_managed_receipts_only: bool,
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
pub(crate) struct DxForgeRestoreApprovalReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub approval_ready: bool,
    pub target_path: String,
    pub restore_destination_root: String,
    pub blocker_count: usize,
    pub next_action: String,
}

pub(crate) fn build_dx_forge_restore_approval(
    request: DxForgeRestoreApprovalRequest,
) -> DxForgeRestoreApproval {
    let restore_value = decode_json_string(&request.restore_execution);
    let restore_execution = restore_value
        .as_ref()
        .ok()
        .and_then(locate_restore_execution);
    let restore = restore_execution
        .map(summarize_restore_execution)
        .unwrap_or_else(DxForgeRestoreApprovalRestoreSummary::missing);
    let target_path = clean_optional_text(request.target_path);
    let target_path_source = if target_path.is_some() {
        "input"
    } else if !restore.backup_target_path.is_empty() {
        "restore_receipt"
    } else {
        "missing"
    };
    let resolved_target_path = target_path
        .or_else(|| clean_optional_text(Some(restore.backup_target_path.clone())))
        .unwrap_or_default();
    let evidence = clean_lines(request.evidence, 24);
    let explicit_blockers = clean_lines(request.blockers, 24);
    let approval_note = clean_optional_text(request.approval_note);
    let mut blockers = explicit_blockers.clone();
    let mut warnings = Vec::new();

    if restore_value.is_err() {
        blockers.push("Restore execution input could not be parsed as JSON.".to_string());
    }
    if restore_execution.is_none() {
        blockers.push(
            "Restore approval needs a zed.dx.forge.restore_execution.v1 object or receipt."
                .to_string(),
        );
    }
    if resolved_target_path.is_empty() {
        blockers.push("Restore-to-target approval needs a target path.".to_string());
    }
    if restore.status != "restore_preview_written" {
        blockers.push(
            "Restore approval needs a managed restore preview whose status is restore_preview_written."
                .to_string(),
        );
    }
    if restore.restore_destination_root.is_empty() {
        blockers.push("Restore approval needs a managed restore preview path.".to_string());
    }
    if restore.target_mutation_applied {
        blockers.push(
            "Refusing approval capture for a restore receipt that reports target mutation."
                .to_string(),
        );
    }
    if restore.overwrote_existing_files {
        blockers.push(
            "Refusing approval capture for a restore receipt that reports overwritten preview files."
                .to_string(),
        );
    }
    if !request.operator_approval {
        blockers.push("Operator approval has not been captured.".to_string());
    }
    if !request.rollback_verified {
        blockers.push("Rollback evidence has not been verified.".to_string());
    }
    if evidence.is_empty() {
        blockers.push("Restore approval needs at least one evidence line.".to_string());
    }
    if request.overwrite_approved {
        warnings.push(
            "Overwrite approval was recorded, but this tool did not overwrite or mutate target files."
                .to_string(),
        );
    }

    let evidence_count = evidence.len();
    let approval_ready = blockers.is_empty();
    let status = if approval_ready {
        "approval_receipt_ready"
    } else {
        "needs_approval_evidence"
    };
    let next_action = if approval_ready {
        "Review this managed approval receipt before any future governed restore-to-target tool mutates files."
            .to_string()
    } else {
        "Resolve the listed blockers, then capture restore approval evidence again.".to_string()
    };

    DxForgeRestoreApproval {
        schema: DX_FORGE_RESTORE_APPROVAL_SCHEMA,
        generated_at_ms: request.generated_at_ms,
        request: DxForgeRestoreApprovalRequestSummary {
            target_path: resolved_target_path.clone(),
            target_path_source,
            operator_approval: request.operator_approval,
            rollback_verified: request.rollback_verified,
            overwrite_approved: request.overwrite_approved,
            evidence,
            blockers: explicit_blockers,
            approval_note,
            root_mode: request.root_mode,
        },
        restore,
        validation: DxForgeRestoreApprovalValidation {
            status,
            approval_ready,
            receipt_usable_for_restore_to_target: approval_ready,
            evidence_count,
            blocker_count: blockers.len(),
            blockers,
            warnings,
        },
        approval_receipt: None,
        safety: DxForgeRestoreApprovalSafety {
            writes_managed_receipts_only: true,
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

impl DxForgeRestoreApprovalRestoreSummary {
    fn missing() -> Self {
        Self {
            schema: "missing".to_string(),
            status: "missing".to_string(),
            backup_target_path: String::new(),
            restore_destination_root: String::new(),
            backup_archive_path: String::new(),
            backup_manifest_path: String::new(),
            restored_file_count: 0,
            restored_directory_count: 0,
            restored_total_file_bytes: 0,
            verified_root_sha256: String::new(),
            target_mutation_applied: false,
            overwrote_existing_files: false,
        }
    }
}

fn summarize_restore_execution(value: &Value) -> DxForgeRestoreApprovalRestoreSummary {
    DxForgeRestoreApprovalRestoreSummary {
        schema: string_field(value, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        status: string_field(value, &["restore", "status"])
            .unwrap_or_else(|| "unknown".to_string()),
        backup_target_path: string_field(value, &["backup", "target_path"]).unwrap_or_default(),
        restore_destination_root: string_field(value, &["restore", "restore_destination_root"])
            .unwrap_or_default(),
        backup_archive_path: string_field(value, &["backup", "archive_path"]).unwrap_or_default(),
        backup_manifest_path: string_field(value, &["backup", "manifest_path"]).unwrap_or_default(),
        restored_file_count: usize_field(value, &["restore", "restored_file_count"]).unwrap_or(0),
        restored_directory_count: usize_field(value, &["restore", "restored_directory_count"])
            .unwrap_or(0),
        restored_total_file_bytes: u64_field(value, &["restore", "restored_total_file_bytes"])
            .unwrap_or(0),
        verified_root_sha256: string_field(value, &["restore", "verified_root_sha256"])
            .or_else(|| string_field(value, &["manifest", "restored_root_sha256"]))
            .unwrap_or_default(),
        target_mutation_applied: bool_field(value, &["restore", "target_mutation_applied"])
            .unwrap_or(false),
        overwrote_existing_files: bool_field(value, &["restore", "overwrote_existing_files"])
            .unwrap_or(false),
    }
}

fn locate_restore_execution(value: &Value) -> Option<&Value> {
    if value.get("schema").and_then(Value::as_str) == Some(DX_FORGE_RESTORE_EXECUTION_SCHEMA) {
        return Some(value);
    }

    value.get("restore_execution").filter(|execution| {
        execution.get("schema").and_then(Value::as_str) == Some(DX_FORGE_RESTORE_EXECUTION_SCHEMA)
    })
}

fn decode_json_string(value: &Value) -> Result<Value, String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if text.starts_with('{') {
            return serde_json::from_str(text).map_err(|error| {
                format!("Failed to parse stringified DX Forge restore execution JSON: {error}")
            });
        }
    }

    Ok(value.clone())
}

fn clean_lines(values: Vec<String>, limit: usize) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|value| clean_optional_text(Some(value)))
        .take(limit)
        .collect()
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
