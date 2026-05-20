use serde::Serialize;
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const DX_FORGE_SAFETY_POLICY_SCHEMA: &str = "zed.dx.forge.safety_policy.v1";

pub(crate) const DX_FORGE_BACKUP_RUNNER_GATE_SCHEMA: &str = "zed.dx.forge.backup_runner_gate.v1";
pub(crate) const DX_FORGE_BACKUP_RUNNER_GATE_RECEIPT_SCHEMA: &str =
    "zed.dx.forge.backup_runner_gate_receipt.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxForgeBackupRunnerGateRequest {
    pub forge_safety_policy: Value,
    pub approve_runner: bool,
    pub require_existing_target: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeBackupRunnerGate {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxForgeBackupRunnerGateRequestSummary,
    pub policy: DxForgeBackupRunnerGatePolicySummary,
    pub validation: DxForgeBackupRunnerGateValidation,
    pub runner_receipt: Option<DxForgeBackupRunnerGateReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeBackupRunnerGateRequestSummary {
    pub approve_runner: bool,
    pub require_existing_target: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeBackupRunnerGatePolicySummary {
    pub schema: String,
    pub operation: String,
    pub policy_status: String,
    pub target_path: Option<String>,
    pub target_exists: bool,
    pub target_under_workspace: bool,
    pub destination_path: Option<String>,
    pub destination_under_workspace: Option<bool>,
    pub policy_approved: bool,
    pub backup_required_before_mutation: bool,
    pub restore_receipt_required: bool,
    pub no_permanent_delete: bool,
    pub quarantine_instead_of_delete: bool,
    pub dry_run_only: bool,
    pub policy_tool_deleted_files: bool,
    pub policy_tool_moved_files: bool,
    pub policy_tool_wrote_backup: bool,
    pub backup_strategy: String,
    pub compression: String,
    pub content_hash_required: bool,
    pub managed_backup_dir: Option<String>,
    pub planned_archive_path: Option<String>,
    pub planned_manifest_path: Option<String>,
    pub planned_quarantine_path: Option<String>,
    pub archive_under_managed_backup_dir: bool,
    pub manifest_under_managed_manifest_dir: bool,
    pub quarantine_under_managed_quarantine_dir: bool,
    pub captures_destination_before_overwrite: bool,
    pub safe_to_run_after_approval: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeBackupRunnerGateValidation {
    pub status: String,
    pub runner_ready: bool,
    pub permission_required: bool,
    pub runner_approved: bool,
    pub would_run_archive_process: bool,
    pub would_mutate_target_after_backup: bool,
    pub tool_ran_forge: bool,
    pub tool_ran_zstd: bool,
    pub tool_deleted_files: bool,
    pub tool_moved_files: bool,
    pub tool_wrote_backup_archive: bool,
    pub no_permanent_delete_enforced: bool,
    pub backup_archive_planned: bool,
    pub manifest_planned: bool,
    pub restore_receipt_required: bool,
    pub quarantine_ready_for_delete: bool,
    pub backup_paths_managed: bool,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeBackupRunnerGateReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub runner_gate_schema: &'static str,
    pub operation: String,
    pub runner_ready: bool,
    pub planned_archive_path: Option<String>,
    pub planned_manifest_path: Option<String>,
    pub planned_quarantine_path: Option<String>,
    pub next_action: String,
}

pub(crate) fn build_dx_forge_backup_runner_gate(
    request: DxForgeBackupRunnerGateRequest,
) -> Result<DxForgeBackupRunnerGate, String> {
    let policy_value = decode_json_string(&request.forge_safety_policy)?;
    let policy = locate_forge_policy(&policy_value).ok_or_else(|| {
        "DX Forge backup runner gate needs a zed.dx.forge.safety_policy.v1 object or policy receipt."
            .to_string()
    })?;
    let summary = summarize_policy(policy);
    let backup_archive_planned = summary.planned_archive_path.is_some();
    let manifest_planned = summary.planned_manifest_path.is_some();
    let quarantine_ready_for_delete = summary.operation != "delete"
        || (summary.quarantine_instead_of_delete
            && summary.planned_quarantine_path.is_some()
            && summary.quarantine_under_managed_quarantine_dir);
    let backup_paths_managed = summary.archive_under_managed_backup_dir
        && summary.manifest_under_managed_manifest_dir
        && quarantine_ready_for_delete;
    let restore_receipt_required = summary.restore_receipt_required;

    let mut blockers = Vec::new();
    if !request.approve_runner {
        blockers.push("Forge backup runner has not been approved for this gate.".to_string());
    }
    if summary.schema != DX_FORGE_SAFETY_POLICY_SCHEMA {
        blockers.push(format!(
            "Expected Forge safety policy schema {DX_FORGE_SAFETY_POLICY_SCHEMA}, got {}.",
            summary.schema
        ));
    }
    if summary.policy_status != "backup_policy_ready" {
        blockers.push(format!(
            "Forge safety policy status is `{}` instead of `backup_policy_ready`.",
            summary.policy_status
        ));
    }
    if request.require_existing_target && !summary.target_exists {
        blockers.push("Forge target must exist before backup runner execution.".to_string());
    }
    if !summary.target_under_workspace {
        blockers.push(
            "Forge target must be inside the active workspace unless a future runner adds an explicit outside-workspace override."
                .to_string(),
        );
    }
    if !summary.policy_approved {
        blockers.push("Forge safety policy was not approved for runner preparation.".to_string());
    }
    if !summary.backup_required_before_mutation {
        blockers.push("Forge policy must require backup before mutation.".to_string());
    }
    if !summary.restore_receipt_required {
        blockers.push("Forge policy must require a restore receipt.".to_string());
    }
    if !summary.no_permanent_delete {
        blockers.push("Forge policy must forbid permanent deletes.".to_string());
    }
    if summary.operation == "delete" && !summary.quarantine_instead_of_delete {
        blockers.push(
            "Delete operations must quarantine after backup instead of deleting.".to_string(),
        );
    }
    if !summary.dry_run_only {
        blockers.push("Forge safety policy must come from a dry-run planning tool.".to_string());
    }
    if summary.policy_tool_deleted_files
        || summary.policy_tool_moved_files
        || summary.policy_tool_wrote_backup
    {
        blockers.push(
            "Forge safety policy tool must not have deleted, moved, or written backup files."
                .to_string(),
        );
    }
    if summary.compression != "zstd" {
        blockers.push("Forge backup runner expects zstd compression.".to_string());
    }
    if !summary.content_hash_required {
        blockers.push("Forge backup manifest must require content hashes.".to_string());
    }
    if !backup_archive_planned {
        blockers.push("Forge backup runner needs a planned archive path.".to_string());
    }
    if !manifest_planned {
        blockers.push("Forge backup runner needs a planned manifest path.".to_string());
    }
    if !backup_paths_managed {
        blockers.push("Forge backup, manifest, and quarantine paths must stay under the managed dx-forge root.".to_string());
    }
    if !summary.safe_to_run_after_approval {
        blockers.push("Forge backup plan is not marked safe_to_run_after_approval.".to_string());
    }
    if summary.operation == "overwrite" && !summary.captures_destination_before_overwrite {
        blockers
            .push("Overwrite operations must capture the destination before mutation.".to_string());
    }
    if matches!(summary.operation.as_str(), "move" | "overwrite")
        && summary.destination_path.is_none()
    {
        blockers.push("Move/overwrite backup runner gates need a destination path.".to_string());
    }

    let runner_ready = blockers.is_empty();
    let status = if runner_ready {
        "runner_ready"
    } else if request.approve_runner {
        "blocked_after_approval"
    } else {
        "approval_required"
    };
    let next_action = if runner_ready {
        "Use this gate receipt with execute_dx_forge_backup to write a native zstd backup bundle, manifest, and execution receipt before any target mutation."
            .to_string()
    } else if request.approve_runner {
        "Resolve the listed Forge backup runner blockers before any archive or quarantine execution."
            .to_string()
    } else {
        "Review this Forge backup runner gate, then rerun with approve_runner=true when ready to authorize the future backup/quarantine runner."
            .to_string()
    };

    Ok(DxForgeBackupRunnerGate {
        schema: DX_FORGE_BACKUP_RUNNER_GATE_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxForgeBackupRunnerGateRequestSummary {
            approve_runner: request.approve_runner,
            require_existing_target: request.require_existing_target,
            root_mode: request.root_mode,
        },
        policy: summary,
        validation: DxForgeBackupRunnerGateValidation {
            status: status.to_string(),
            runner_ready,
            permission_required: true,
            runner_approved: request.approve_runner,
            would_run_archive_process: runner_ready,
            would_mutate_target_after_backup: runner_ready,
            tool_ran_forge: false,
            tool_ran_zstd: false,
            tool_deleted_files: false,
            tool_moved_files: false,
            tool_wrote_backup_archive: false,
            no_permanent_delete_enforced: true,
            backup_archive_planned,
            manifest_planned,
            restore_receipt_required,
            quarantine_ready_for_delete,
            backup_paths_managed,
            blockers,
        },
        runner_receipt: None,
        next_action,
    })
}

fn summarize_policy(policy: &Value) -> DxForgeBackupRunnerGatePolicySummary {
    let managed_backup_dir = string_field(policy, &["backup_plan", "managed_backup_dir"]);
    let managed_root = managed_backup_dir
        .as_deref()
        .and_then(|dir| Path::new(dir).parent())
        .map(Path::to_path_buf);
    let managed_manifest_dir = managed_root.as_ref().map(|root| root.join("manifests"));
    let managed_quarantine_dir = managed_root.as_ref().map(|root| root.join("quarantine"));
    let planned_archive_path = string_field(policy, &["backup_plan", "planned_archive_path"]);
    let planned_manifest_path = string_field(policy, &["backup_plan", "planned_manifest_path"]);
    let planned_quarantine_path = string_field(policy, &["backup_plan", "planned_quarantine_path"]);

    DxForgeBackupRunnerGatePolicySummary {
        schema: string_field(policy, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        operation: string_field(policy, &["request", "operation"])
            .unwrap_or_else(|| "unknown".to_string()),
        policy_status: string_field(policy, &["policy", "status"])
            .unwrap_or_else(|| "unknown".to_string()),
        target_path: string_field(policy, &["target", "resolved_path"]),
        target_exists: bool_field(policy, &["target", "exists"]).unwrap_or(false),
        target_under_workspace: bool_field(policy, &["target", "under_workspace"]).unwrap_or(false),
        destination_path: string_field(policy, &["destination", "resolved_path"]),
        destination_under_workspace: bool_field(policy, &["destination", "under_workspace"]),
        policy_approved: bool_field(policy, &["policy", "policy_approved"]).unwrap_or(false),
        backup_required_before_mutation: bool_field(
            policy,
            &["policy", "backup_required_before_mutation"],
        )
        .unwrap_or(false),
        restore_receipt_required: bool_field(policy, &["policy", "restore_receipt_required"])
            .unwrap_or(false),
        no_permanent_delete: bool_field(policy, &["policy", "no_permanent_delete"])
            .unwrap_or(false),
        quarantine_instead_of_delete: bool_field(
            policy,
            &["policy", "quarantine_instead_of_delete"],
        )
        .unwrap_or(false),
        dry_run_only: bool_field(policy, &["policy", "dry_run_only"]).unwrap_or(false),
        policy_tool_deleted_files: bool_field(policy, &["policy", "tool_deleted_files"])
            .unwrap_or(true),
        policy_tool_moved_files: bool_field(policy, &["policy", "tool_moved_files"])
            .unwrap_or(true),
        policy_tool_wrote_backup: bool_field(policy, &["policy", "tool_wrote_backup"])
            .unwrap_or(true),
        backup_strategy: string_field(policy, &["backup_plan", "strategy"])
            .unwrap_or_else(|| "unknown".to_string()),
        compression: string_field(policy, &["backup_plan", "compression"])
            .unwrap_or_else(|| "unknown".to_string()),
        content_hash_required: bool_field(policy, &["backup_plan", "content_hash_required"])
            .unwrap_or(false),
        managed_backup_dir: managed_backup_dir.clone(),
        planned_archive_path: planned_archive_path.clone(),
        planned_manifest_path: planned_manifest_path.clone(),
        planned_quarantine_path: planned_quarantine_path.clone(),
        archive_under_managed_backup_dir: path_under(
            planned_archive_path.as_deref(),
            managed_backup_dir.as_deref().map(Path::new),
        ),
        manifest_under_managed_manifest_dir: path_under(
            planned_manifest_path.as_deref(),
            managed_manifest_dir.as_deref(),
        ),
        quarantine_under_managed_quarantine_dir: path_under(
            planned_quarantine_path.as_deref(),
            managed_quarantine_dir.as_deref(),
        ),
        captures_destination_before_overwrite: bool_field(
            policy,
            &["backup_plan", "captures_destination_before_overwrite"],
        )
        .unwrap_or(false),
        safe_to_run_after_approval: bool_field(
            policy,
            &["backup_plan", "safe_to_run_after_approval"],
        )
        .unwrap_or(false),
    }
}

fn path_under(path: Option<&str>, root: Option<&Path>) -> bool {
    let (Some(path), Some(root)) = (path, root) else {
        return false;
    };
    Path::new(path).starts_with(root)
}

fn locate_forge_policy(value: &Value) -> Option<&Value> {
    if value.get("schema").and_then(Value::as_str) == Some(DX_FORGE_SAFETY_POLICY_SCHEMA) {
        return Some(value);
    }

    value.get("forge_safety_policy").filter(|policy| {
        policy.get("schema").and_then(Value::as_str) == Some(DX_FORGE_SAFETY_POLICY_SCHEMA)
    })
}

fn decode_json_string(value: &Value) -> Result<Value, String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if text.starts_with('{') {
            return serde_json::from_str(text).map_err(|error| {
                format!("Failed to parse stringified DX Forge safety policy JSON: {error}")
            });
        }
    }

    Ok(value.clone())
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

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
