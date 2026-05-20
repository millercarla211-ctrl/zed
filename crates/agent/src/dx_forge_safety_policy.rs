use serde::Serialize;
use std::{
    env, fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const DX_FORGE_ROOT_ENV: &str = "DX_FORGE_ROOT";
const DEFAULT_FORGE_ROOT: &str = r"G:\Workspaces\flow\forge";

pub(crate) const DX_FORGE_SAFETY_POLICY_SCHEMA: &str = "zed.dx.forge.safety_policy.v1";
pub(crate) const DX_FORGE_SAFETY_POLICY_RECEIPT_SCHEMA: &str =
    "zed.dx.forge.safety_policy_receipt.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxForgeSafetyPolicyRequest {
    pub target_path: String,
    pub operation: Option<String>,
    pub destination_path: Option<String>,
    pub reason: Option<String>,
    pub approve_policy: bool,
    pub allow_outside_workspace: bool,
    pub workspace_root: Option<PathBuf>,
    pub managed_artifact_root: PathBuf,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeSafetyPolicy {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxForgeSafetyPolicyRequestSummary,
    pub target: DxForgeSafetyTargetSummary,
    pub destination: Option<DxForgeSafetyDestinationSummary>,
    pub forge: DxForgeToolRootSummary,
    pub policy: DxForgeSafetyPolicyDecision,
    pub backup_plan: DxForgeBackupPlan,
    pub restore_plan: DxForgeRestorePlan,
    pub policy_receipt: Option<DxForgeSafetyPolicyReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeSafetyPolicyRequestSummary {
    pub operation: String,
    pub reason: Option<String>,
    pub approve_policy: bool,
    pub allow_outside_workspace: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeSafetyTargetSummary {
    pub original: String,
    pub resolved_path: String,
    pub exists: bool,
    pub is_file: bool,
    pub is_dir: bool,
    pub size_bytes: Option<u64>,
    pub under_workspace: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeSafetyDestinationSummary {
    pub original: String,
    pub resolved_path: String,
    pub exists: bool,
    pub under_workspace: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeToolRootSummary {
    pub env_var: &'static str,
    pub root: String,
    pub root_exists: bool,
    pub cargo_toml_exists: bool,
    pub integration_ready: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeSafetyPolicyDecision {
    pub status: String,
    pub permission_required: bool,
    pub policy_approved: bool,
    pub backup_required_before_mutation: bool,
    pub restore_receipt_required: bool,
    pub no_permanent_delete: bool,
    pub quarantine_instead_of_delete: bool,
    pub tool_deleted_files: bool,
    pub tool_moved_files: bool,
    pub tool_wrote_backup: bool,
    pub dry_run_only: bool,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeBackupPlan {
    pub schema: &'static str,
    pub strategy: String,
    pub managed_backup_dir: String,
    pub planned_archive_path: String,
    pub planned_manifest_path: String,
    pub planned_quarantine_path: Option<String>,
    pub compression: String,
    pub content_hash_required: bool,
    pub captures_destination_before_overwrite: bool,
    pub safe_to_run_after_approval: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestorePlan {
    pub restore_instruction: String,
    pub restore_receipt_required: bool,
    pub original_path: String,
    pub destination_path: Option<String>,
    pub archive_path: String,
    pub manifest_path: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeSafetyPolicyReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub policy_schema: &'static str,
    pub operation: String,
    pub policy_status: String,
    pub next_action: String,
}

pub(crate) fn build_dx_forge_safety_policy(
    request: DxForgeSafetyPolicyRequest,
) -> Result<DxForgeSafetyPolicy, String> {
    let operation = normalize_operation(request.operation)?;
    let target = summarize_target(
        &request.target_path,
        request.workspace_root.as_deref(),
        "DX Forge safety policy needs a target path.",
    )?;
    let destination = request
        .destination_path
        .as_deref()
        .map(|path| summarize_destination(path, request.workspace_root.as_deref()))
        .transpose()?;
    let forge = forge_root_summary();
    let blockers = policy_blockers(
        &operation,
        &target,
        destination.as_ref(),
        request.approve_policy,
        request.allow_outside_workspace,
    );
    let policy_ready = blockers.is_empty();
    let policy_status = if policy_ready {
        "backup_policy_ready"
    } else if request.approve_policy {
        "blocked_after_approval"
    } else {
        "approval_required"
    };
    let backup_plan = build_backup_plan(
        &operation,
        &target,
        destination.as_ref(),
        &request.managed_artifact_root,
        policy_ready,
    );
    let restore_plan = build_restore_plan(&target, destination.as_ref(), &backup_plan);
    let next_action = if policy_ready {
        "Use this policy receipt to implement a future Forge/zstd backup runner that archives first and quarantines delete targets instead of permanently deleting them."
            .to_string()
    } else if request.approve_policy {
        "Resolve the listed Forge safety blockers before preparing a backup runner.".to_string()
    } else {
        "Review this backup-first policy, then rerun with approve_policy=true when ready to authorize the future backup runner."
            .to_string()
    };

    Ok(DxForgeSafetyPolicy {
        schema: DX_FORGE_SAFETY_POLICY_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxForgeSafetyPolicyRequestSummary {
            operation: operation.clone(),
            reason: clean_optional_text(request.reason, 240),
            approve_policy: request.approve_policy,
            allow_outside_workspace: request.allow_outside_workspace,
            root_mode: request.root_mode,
        },
        target,
        destination,
        forge,
        policy: DxForgeSafetyPolicyDecision {
            status: policy_status.to_string(),
            permission_required: true,
            policy_approved: request.approve_policy,
            backup_required_before_mutation: true,
            restore_receipt_required: true,
            no_permanent_delete: true,
            quarantine_instead_of_delete: operation == "delete",
            tool_deleted_files: false,
            tool_moved_files: false,
            tool_wrote_backup: false,
            dry_run_only: true,
            blockers,
        },
        backup_plan,
        restore_plan,
        policy_receipt: None,
        next_action,
    })
}

fn normalize_operation(operation: Option<String>) -> Result<String, String> {
    let operation = operation
        .unwrap_or_else(|| "delete".to_string())
        .trim()
        .to_ascii_lowercase()
        .replace('-', "_");

    match operation.as_str() {
        "" | "delete" | "remove" => Ok("delete".to_string()),
        "move" | "rename" => Ok("move".to_string()),
        "overwrite" | "replace" => Ok("overwrite".to_string()),
        "cleanup" | "clean" => Ok("cleanup".to_string()),
        _ => Err(format!(
            "Unsupported DX Forge safety operation `{operation}`. Use delete, move, overwrite, or cleanup."
        )),
    }
}

fn summarize_target(
    target_path: &str,
    workspace_root: Option<&Path>,
    empty_error: &str,
) -> Result<DxForgeSafetyTargetSummary, String> {
    let original = compact_text(target_path);
    if original.is_empty() {
        return Err(empty_error.to_string());
    }

    let resolved = normalize_existing_or_lexical(resolve_path(&original, workspace_root));
    let workspace_root = workspace_root.map(|root| normalize_existing_or_lexical(root));
    let metadata = fs::metadata(&resolved).ok();
    Ok(DxForgeSafetyTargetSummary {
        original,
        resolved_path: resolved.display().to_string(),
        exists: metadata.is_some(),
        is_file: metadata
            .as_ref()
            .map(|metadata| metadata.is_file())
            .unwrap_or(false),
        is_dir: metadata
            .as_ref()
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false),
        size_bytes: metadata.as_ref().map(|metadata| metadata.len()),
        under_workspace: workspace_root
            .as_ref()
            .map(|root| resolved.starts_with(root))
            .unwrap_or(false),
    })
}

fn summarize_destination(
    destination_path: &str,
    workspace_root: Option<&Path>,
) -> Result<DxForgeSafetyDestinationSummary, String> {
    let original = compact_text(destination_path);
    if original.is_empty() {
        return Err("DX Forge safety destination path cannot be empty.".to_string());
    }

    let resolved = normalize_existing_or_lexical(resolve_path(&original, workspace_root));
    let workspace_root = workspace_root.map(|root| normalize_existing_or_lexical(root));
    Ok(DxForgeSafetyDestinationSummary {
        original,
        exists: resolved.exists(),
        under_workspace: workspace_root
            .as_ref()
            .map(|root| resolved.starts_with(root))
            .unwrap_or(false),
        resolved_path: resolved.display().to_string(),
    })
}

fn resolve_path(path: &str, workspace_root: Option<&Path>) -> PathBuf {
    let candidate = PathBuf::from(path);
    let resolved = if candidate.is_absolute() {
        candidate
    } else if let Some(root) = workspace_root {
        root.join(candidate)
    } else {
        candidate
    };
    normalize_path(resolved)
}

fn normalize_existing_or_lexical(path: impl AsRef<Path>) -> PathBuf {
    fs::canonicalize(path.as_ref()).unwrap_or_else(|_| normalize_path(path.as_ref().to_path_buf()))
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(_) | Component::Prefix(_) | Component::RootDir => {
                normalized.push(component.as_os_str());
            }
        }
    }

    normalized
}

fn forge_root_summary() -> DxForgeToolRootSummary {
    let root = env::var_os(DX_FORGE_ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_FORGE_ROOT));
    let cargo_toml = root.join("Cargo.toml");

    DxForgeToolRootSummary {
        env_var: DX_FORGE_ROOT_ENV,
        root: root.display().to_string(),
        root_exists: root.exists(),
        cargo_toml_exists: cargo_toml.is_file(),
        integration_ready: root.exists() && cargo_toml.is_file(),
    }
}

fn policy_blockers(
    operation: &str,
    target: &DxForgeSafetyTargetSummary,
    destination: Option<&DxForgeSafetyDestinationSummary>,
    approve_policy: bool,
    allow_outside_workspace: bool,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !approve_policy {
        blockers.push("Forge safety policy has not been approved for this plan.".to_string());
    }
    if !target.exists {
        blockers.push("Target path does not exist, so no backup can be planned yet.".to_string());
    }
    if !target.under_workspace && !allow_outside_workspace {
        blockers.push(
            "Target path is outside the active workspace; set allow_outside_workspace only after explicit review."
                .to_string(),
        );
    }
    if matches!(operation, "move" | "overwrite") && destination.is_none() {
        blockers.push("Move/overwrite safety plans need a destination path.".to_string());
    }
    if let Some(destination) = destination {
        if !destination.under_workspace && !allow_outside_workspace {
            blockers.push(
                "Destination path is outside the active workspace; set allow_outside_workspace only after explicit review."
                    .to_string(),
            );
        }
    }

    blockers
}

fn build_backup_plan(
    operation: &str,
    target: &DxForgeSafetyTargetSummary,
    destination: Option<&DxForgeSafetyDestinationSummary>,
    artifact_root: &Path,
    safe_to_run_after_approval: bool,
) -> DxForgeBackupPlan {
    let backup_dir = artifact_root.join("backups");
    let manifest_dir = artifact_root.join("manifests");
    let quarantine_dir = artifact_root.join("quarantine");
    let stem = backup_stem(operation, target);
    let planned_archive_path = backup_dir.join(format!("{stem}.tar.zst"));
    let planned_manifest_path = manifest_dir.join(format!("{stem}.json"));
    let planned_quarantine_path = (operation == "delete").then(|| {
        quarantine_dir
            .join(&stem)
            .join(file_name_for_path(&target.resolved_path))
            .display()
            .to_string()
    });

    DxForgeBackupPlan {
        schema: "zed.dx.forge.backup_plan.v1",
        strategy: if operation == "delete" {
            "zstd_archive_then_quarantine".to_string()
        } else {
            "zstd_archive_before_mutation".to_string()
        },
        managed_backup_dir: backup_dir.display().to_string(),
        planned_archive_path: planned_archive_path.display().to_string(),
        planned_manifest_path: planned_manifest_path.display().to_string(),
        planned_quarantine_path,
        compression: "zstd".to_string(),
        content_hash_required: true,
        captures_destination_before_overwrite: destination
            .map(|destination| destination.exists)
            .unwrap_or(false),
        safe_to_run_after_approval,
    }
}

fn build_restore_plan(
    target: &DxForgeSafetyTargetSummary,
    destination: Option<&DxForgeSafetyDestinationSummary>,
    backup: &DxForgeBackupPlan,
) -> DxForgeRestorePlan {
    DxForgeRestorePlan {
        restore_instruction:
            "Restore from the planned archive, verify the manifest content hash, then write a restore receipt."
                .to_string(),
        restore_receipt_required: true,
        original_path: target.resolved_path.clone(),
        destination_path: destination.map(|destination| destination.resolved_path.clone()),
        archive_path: backup.planned_archive_path.clone(),
        manifest_path: backup.planned_manifest_path.clone(),
    }
}

fn backup_stem(operation: &str, target: &DxForgeSafetyTargetSummary) -> String {
    let timestamp = current_unix_ms();
    let name = file_name_for_path(&target.resolved_path);
    format!(
        "{operation}-{timestamp}-{}",
        sanitize_file_component(&name, 80)
    )
}

fn file_name_for_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target")
        .to_string()
}

fn clean_optional_text(value: Option<String>, max_chars: usize) -> Option<String> {
    value
        .map(|value| truncate_for_char_budget(&compact_text(&value), max_chars))
        .filter(|value| !value.is_empty())
}

fn compact_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn sanitize_file_component(text: &str, max_chars: usize) -> String {
    let mut sanitized = String::new();
    for character in text.chars().take(max_chars) {
        if character.is_ascii_alphanumeric() {
            sanitized.push(character.to_ascii_lowercase());
        } else if matches!(character, '-' | '_' | '.') {
            sanitized.push(character);
        } else if character.is_whitespace() || matches!(character, ':' | '/' | '\\') {
            sanitized.push('-');
        }
    }
    let sanitized = sanitized.trim_matches(['-', '.', '_']);
    if sanitized.is_empty() {
        "target".to_string()
    } else {
        sanitized.to_string()
    }
}

fn truncate_for_char_budget(text: &str, max_chars: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let suffix = "...";
    if max_chars <= suffix.len() {
        return text.chars().take(max_chars).collect();
    }

    let mut truncated = text
        .chars()
        .take(max_chars - suffix.len())
        .collect::<String>();
    truncated.push_str(suffix);
    truncated
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
