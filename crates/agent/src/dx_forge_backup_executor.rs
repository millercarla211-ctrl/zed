use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const DX_FORGE_BACKUP_RUNNER_GATE_SCHEMA: &str = "zed.dx.forge.backup_runner_gate.v1";

pub(crate) const DX_FORGE_BACKUP_EXECUTION_SCHEMA: &str = "zed.dx.forge.backup_execution.v1";
pub(crate) const DX_FORGE_BACKUP_EXECUTION_RECEIPT_SCHEMA: &str =
    "zed.dx.forge.backup_execution_receipt.v1";
pub(crate) const DX_FORGE_BACKUP_MANIFEST_SCHEMA: &str = "zed.dx.forge.backup_manifest.v1";
pub(crate) const DX_FORGE_BACKUP_BUNDLE_FORMAT: &str = "dxzst_stream_v1";

#[derive(Clone, Debug)]
pub(crate) struct DxForgeBackupExecutionRequest {
    pub runner_gate: Value,
    pub approve_execution: bool,
    pub apply_quarantine_after_backup: bool,
    pub require_execution_receipt: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeBackupExecution {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxForgeBackupExecutionRequestSummary,
    pub gate: DxForgeBackupExecutionGateSummary,
    pub execution: DxForgeBackupExecutionSummary,
    pub manifest: DxForgeBackupManifestSummary,
    pub execution_receipt: Option<DxForgeBackupExecutionReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeBackupExecutionRequestSummary {
    pub approve_execution: bool,
    pub apply_quarantine_after_backup: bool,
    pub require_execution_receipt: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeBackupExecutionGateSummary {
    pub schema: String,
    pub gate_status: String,
    pub runner_ready: bool,
    pub operation: String,
    pub target_path: String,
    pub destination_path: Option<String>,
    pub planned_archive_path: String,
    pub planned_manifest_path: String,
    pub planned_quarantine_path: Option<String>,
    pub backup_paths_managed: bool,
    pub no_permanent_delete_enforced: bool,
    pub restore_receipt_required: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeBackupExecutionSummary {
    pub status: String,
    pub execution_ready: bool,
    pub permission_required: bool,
    pub execution_approved: bool,
    pub target_mutation_applied: bool,
    pub target_quarantined: bool,
    pub permanent_delete_performed: bool,
    pub wrote_backup_archive: bool,
    pub wrote_manifest: bool,
    pub ran_shell: bool,
    pub ran_external_process: bool,
    pub compression: String,
    pub archive_format: &'static str,
    pub archive_path: String,
    pub manifest_path: String,
    pub quarantine_path: Option<String>,
    pub archive_size_bytes: Option<u64>,
    pub manifest_size_bytes: Option<u64>,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeBackupManifestSummary {
    pub schema: &'static str,
    pub entry_count: usize,
    pub file_count: usize,
    pub directory_count: usize,
    pub total_file_bytes: u64,
    pub root_sha256: String,
    pub restore_instruction: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeBackupExecutionReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub execution_schema: &'static str,
    pub operation: String,
    pub archive_path_written: String,
    pub manifest_path_written: String,
    pub quarantine_path: Option<String>,
    pub target_mutation_applied: bool,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
struct DxForgeBackupManifest {
    schema: &'static str,
    generated_at_ms: u64,
    archive_format: &'static str,
    compression: &'static str,
    operation: String,
    target_path: String,
    destination_path: Option<String>,
    archive_path: String,
    planned_quarantine_path: Option<String>,
    no_permanent_delete: bool,
    restore_receipt_required: bool,
    entries: Vec<DxForgeBackupManifestEntry>,
    total_file_bytes: u64,
    root_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct DxForgeBackupManifestEntry {
    path: String,
    kind: &'static str,
    size_bytes: u64,
    sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct DxForgeBackupBundleHeader {
    format: &'static str,
    generated_at_ms: u64,
    operation: String,
    target_path: String,
}

#[derive(Clone, Debug, Serialize)]
struct DxForgeBackupBundleEntryHeader {
    path: String,
    kind: &'static str,
    size_bytes: u64,
    sha256: Option<String>,
}

pub(crate) fn execute_dx_forge_backup(
    request: DxForgeBackupExecutionRequest,
) -> Result<DxForgeBackupExecution, String> {
    let gate_value = decode_json_string(&request.runner_gate)?;
    let gate = locate_runner_gate(&gate_value).ok_or_else(|| {
        "DX Forge backup execution needs a zed.dx.forge.backup_runner_gate.v1 object or gate receipt."
            .to_string()
    })?;
    let gate_summary = summarize_gate(gate)?;
    let blockers = execution_blockers(&request, &gate_summary);
    let execution_ready = blockers.is_empty();

    if !execution_ready {
        return Ok(blocked_response(request, gate_summary, blockers));
    }

    let target_path = PathBuf::from(&gate_summary.target_path);
    let archive_path = PathBuf::from(&gate_summary.planned_archive_path);
    let manifest_path = PathBuf::from(&gate_summary.planned_manifest_path);
    let quarantine_path = gate_summary
        .planned_quarantine_path
        .as_deref()
        .map(PathBuf::from);

    let backup_result =
        write_backup_bundle(&target_path, &archive_path, &manifest_path, &gate_summary)?;
    let (target_quarantined, blockers) = match apply_quarantine_if_requested(
        &request,
        &gate_summary,
        &target_path,
        quarantine_path.as_deref(),
    ) {
        Ok(target_quarantined) => (target_quarantined, Vec::new()),
        Err(error) => (false, vec![error]),
    };
    let archive_size_bytes = fs::metadata(&archive_path)
        .map(|metadata| metadata.len())
        .ok();
    let manifest_size_bytes = fs::metadata(&manifest_path)
        .map(|metadata| metadata.len())
        .ok();
    let quarantine_failed = !blockers.is_empty();
    let execution_status = if target_quarantined {
        "backup_written_and_target_quarantined"
    } else if !quarantine_failed {
        "backup_written"
    } else {
        "backup_written_quarantine_failed"
    };
    let next_action = if quarantine_failed {
        "Use the backup, manifest, and execution receipt to audit the failed quarantine before retrying or restoring."
    } else {
        "Use the backup, manifest, and execution receipt to restore or audit this mutation before any future cleanup."
    };

    Ok(DxForgeBackupExecution {
        schema: DX_FORGE_BACKUP_EXECUTION_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxForgeBackupExecutionRequestSummary {
            approve_execution: request.approve_execution,
            apply_quarantine_after_backup: request.apply_quarantine_after_backup,
            require_execution_receipt: request.require_execution_receipt,
            root_mode: request.root_mode,
        },
        gate: gate_summary,
        execution: DxForgeBackupExecutionSummary {
            status: execution_status.to_string(),
            execution_ready: true,
            permission_required: true,
            execution_approved: true,
            target_mutation_applied: target_quarantined,
            target_quarantined,
            permanent_delete_performed: false,
            wrote_backup_archive: true,
            wrote_manifest: true,
            ran_shell: false,
            ran_external_process: false,
            compression: "zstd".to_string(),
            archive_format: DX_FORGE_BACKUP_BUNDLE_FORMAT,
            archive_path: archive_path.display().to_string(),
            manifest_path: manifest_path.display().to_string(),
            quarantine_path: quarantine_path.map(|path| path.display().to_string()),
            archive_size_bytes,
            manifest_size_bytes,
            blockers,
        },
        manifest: backup_result,
        execution_receipt: None,
        next_action: next_action.to_string(),
    })
}

fn blocked_response(
    request: DxForgeBackupExecutionRequest,
    gate: DxForgeBackupExecutionGateSummary,
    blockers: Vec<String>,
) -> DxForgeBackupExecution {
    let archive_path = gate.planned_archive_path.clone();
    let manifest_path = gate.planned_manifest_path.clone();
    let quarantine_path = gate.planned_quarantine_path.clone();

    DxForgeBackupExecution {
        schema: DX_FORGE_BACKUP_EXECUTION_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxForgeBackupExecutionRequestSummary {
            approve_execution: request.approve_execution,
            apply_quarantine_after_backup: request.apply_quarantine_after_backup,
            require_execution_receipt: request.require_execution_receipt,
            root_mode: request.root_mode,
        },
        gate,
        execution: DxForgeBackupExecutionSummary {
            status: if request.approve_execution {
                "blocked_after_approval".to_string()
            } else {
                "approval_required".to_string()
            },
            execution_ready: false,
            permission_required: true,
            execution_approved: request.approve_execution,
            target_mutation_applied: false,
            target_quarantined: false,
            permanent_delete_performed: false,
            wrote_backup_archive: false,
            wrote_manifest: false,
            ran_shell: false,
            ran_external_process: false,
            compression: "zstd".to_string(),
            archive_format: DX_FORGE_BACKUP_BUNDLE_FORMAT,
            archive_path,
            manifest_path,
            quarantine_path,
            archive_size_bytes: None,
            manifest_size_bytes: None,
            blockers,
        },
        manifest: DxForgeBackupManifestSummary {
            schema: DX_FORGE_BACKUP_MANIFEST_SCHEMA,
            entry_count: 0,
            file_count: 0,
            directory_count: 0,
            total_file_bytes: 0,
            root_sha256: String::new(),
            restore_instruction:
                "Resolve blockers, then rerun the backup executor before mutating the target."
                    .to_string(),
        },
        execution_receipt: None,
        next_action: "Resolve the listed Forge backup execution blockers before writing archives or quarantining targets."
            .to_string(),
    }
}

fn execution_blockers(
    request: &DxForgeBackupExecutionRequest,
    gate: &DxForgeBackupExecutionGateSummary,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !request.approve_execution {
        blockers.push("Forge backup execution has not been approved.".to_string());
    }
    if !request.require_execution_receipt {
        blockers.push("Forge backup execution requires an execution receipt.".to_string());
    }
    if gate.schema != DX_FORGE_BACKUP_RUNNER_GATE_SCHEMA {
        blockers.push(format!(
            "Expected runner gate schema {DX_FORGE_BACKUP_RUNNER_GATE_SCHEMA}, got {}.",
            gate.schema
        ));
    }
    if !gate.runner_ready || gate.gate_status != "runner_ready" {
        blockers.push("Forge runner gate must be runner_ready before execution.".to_string());
    }
    if !gate.backup_paths_managed {
        blockers.push(
            "Forge backup execution requires managed archive, manifest, and quarantine paths."
                .to_string(),
        );
    }
    if !gate.no_permanent_delete_enforced {
        blockers.push("Forge runner gate must enforce no permanent delete.".to_string());
    }
    if !gate.restore_receipt_required {
        blockers.push("Forge backup execution requires restore receipt policy.".to_string());
    }
    if gate.operation != "delete" && gate.operation != "cleanup" {
        blockers.push(
            "This first Forge executor only supports backup-only cleanup and delete-to-quarantine operations."
                .to_string(),
        );
    }
    if gate.operation == "delete" {
        if !request.apply_quarantine_after_backup {
            blockers.push(
                "Delete operations must explicitly set apply_quarantine_after_backup=true."
                    .to_string(),
            );
        }
        if gate.planned_quarantine_path.is_none() {
            blockers.push("Delete operations need a planned quarantine path.".to_string());
        }
    }

    for path in [
        Some(gate.target_path.as_str()),
        Some(gate.planned_archive_path.as_str()),
        Some(gate.planned_manifest_path.as_str()),
        gate.planned_quarantine_path.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if path.trim().is_empty() {
            blockers.push("Forge backup execution paths cannot be empty.".to_string());
        }
    }

    if !Path::new(&gate.target_path).exists() {
        blockers.push("Forge backup target no longer exists.".to_string());
    }
    if Path::new(&gate.planned_archive_path).exists() {
        blockers.push(
            "Planned Forge backup archive already exists; refusing to overwrite it.".to_string(),
        );
    }
    if Path::new(&gate.planned_manifest_path).exists() {
        blockers.push(
            "Planned Forge backup manifest already exists; refusing to overwrite it.".to_string(),
        );
    }
    if let Some(quarantine_path) = &gate.planned_quarantine_path
        && Path::new(quarantine_path).exists()
    {
        blockers.push(
            "Planned Forge quarantine path already exists; refusing to overwrite it.".to_string(),
        );
    }

    blockers
}

fn write_backup_bundle(
    target_path: &Path,
    archive_path: &Path,
    manifest_path: &Path,
    gate: &DxForgeBackupExecutionGateSummary,
) -> Result<DxForgeBackupManifestSummary, String> {
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to prepare Forge backup archive directory {}: {error}",
                parent.display()
            )
        })?;
    }
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to prepare Forge backup manifest directory {}: {error}",
                parent.display()
            )
        })?;
    }

    let archive_file = create_new_file(archive_path, "Forge backup archive")?;
    let mut encoder = zstd::stream::Encoder::new(archive_file, 3).map_err(|error| {
        format!(
            "Failed to initialize zstd encoder for {}: {error}",
            archive_path.display()
        )
    })?;
    serde_json::to_writer(
        &mut encoder,
        &DxForgeBackupBundleHeader {
            format: DX_FORGE_BACKUP_BUNDLE_FORMAT,
            generated_at_ms: current_unix_ms(),
            operation: gate.operation.clone(),
            target_path: gate.target_path.clone(),
        },
    )
    .map_err(|error| format!("Failed to write Forge backup bundle header: {error}"))?;
    encoder
        .write_all(b"\n")
        .map_err(|error| format!("Failed to write Forge backup bundle separator: {error}"))?;

    let base_parent = target_path.parent().unwrap_or_else(|| Path::new(""));
    let mut entries = Vec::new();
    write_path_to_archive(target_path, base_parent, &mut encoder, &mut entries)?;
    encoder.finish().map_err(|error| {
        format!(
            "Failed to finish zstd archive {}: {error}",
            archive_path.display()
        )
    })?;

    let total_file_bytes = entries.iter().map(|entry| entry.size_bytes).sum();
    let file_count = entries.iter().filter(|entry| entry.kind == "file").count();
    let directory_count = entries
        .iter()
        .filter(|entry| entry.kind == "directory")
        .count();
    let root_sha256 = root_hash_for_entries(&entries);
    let manifest = DxForgeBackupManifest {
        schema: DX_FORGE_BACKUP_MANIFEST_SCHEMA,
        generated_at_ms: current_unix_ms(),
        archive_format: DX_FORGE_BACKUP_BUNDLE_FORMAT,
        compression: "zstd",
        operation: gate.operation.clone(),
        target_path: gate.target_path.clone(),
        destination_path: gate.destination_path.clone(),
        archive_path: archive_path.display().to_string(),
        planned_quarantine_path: gate.planned_quarantine_path.clone(),
        no_permanent_delete: true,
        restore_receipt_required: true,
        entries,
        total_file_bytes,
        root_sha256: root_sha256.clone(),
    };
    let manifest_json = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| format!("Failed to serialize Forge backup manifest: {error}"))?;
    let mut manifest_file = create_new_file(manifest_path, "Forge backup manifest")?;
    manifest_file.write_all(&manifest_json).map_err(|error| {
        format!(
            "Failed to write Forge backup manifest {}: {error}",
            manifest_path.display()
        )
    })?;

    Ok(DxForgeBackupManifestSummary {
        schema: DX_FORGE_BACKUP_MANIFEST_SCHEMA,
        entry_count: manifest.entries.len(),
        file_count,
        directory_count,
        total_file_bytes,
        root_sha256,
        restore_instruction:
            "Restore by reading the manifest, decoding the zstd backup bundle, verifying entry hashes, and writing a restore receipt."
                .to_string(),
    })
}

fn write_path_to_archive(
    path: &Path,
    base_parent: &Path,
    encoder: &mut zstd::stream::Encoder<'_, File>,
    entries: &mut Vec<DxForgeBackupManifestEntry>,
) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Failed to read metadata for {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Refusing to back up symlink {} in this first Forge executor.",
            path.display()
        ));
    }

    let relative_path = path
        .strip_prefix(base_parent)
        .unwrap_or(path)
        .display()
        .to_string();

    if metadata.is_dir() {
        write_bundle_entry_header(
            encoder,
            &DxForgeBackupBundleEntryHeader {
                path: relative_path.clone(),
                kind: "directory",
                size_bytes: 0,
                sha256: None,
            },
        )?;
        entries.push(DxForgeBackupManifestEntry {
            path: relative_path,
            kind: "directory",
            size_bytes: 0,
            sha256: None,
        });

        let mut children = fs::read_dir(path)
            .map_err(|error| format!("Failed to read directory {}: {error}", path.display()))?
            .collect::<Result<Vec<_>, io::Error>>()
            .map_err(|error| {
                format!("Failed to enumerate directory {}: {error}", path.display())
            })?;
        children.sort_by_key(|entry| entry.path());
        for child in children {
            write_path_to_archive(&child.path(), base_parent, encoder, entries)?;
        }
    } else if metadata.is_file() {
        let sha256 = hash_file(path)?;
        write_bundle_entry_header(
            encoder,
            &DxForgeBackupBundleEntryHeader {
                path: relative_path.clone(),
                kind: "file",
                size_bytes: metadata.len(),
                sha256: Some(sha256.clone()),
            },
        )?;
        let mut file = File::open(path)
            .map_err(|error| format!("Failed to open backup source {}: {error}", path.display()))?;
        io::copy(&mut file, &mut *encoder).map_err(|error| {
            format!("Failed to write backup source {}: {error}", path.display())
        })?;
        encoder.write_all(b"\n").map_err(|error| {
            format!(
                "Failed to terminate backup entry {}: {error}",
                path.display()
            )
        })?;
        entries.push(DxForgeBackupManifestEntry {
            path: relative_path,
            kind: "file",
            size_bytes: metadata.len(),
            sha256: Some(sha256),
        });
    }

    Ok(())
}

fn write_bundle_entry_header(
    encoder: &mut zstd::stream::Encoder<'_, File>,
    header: &DxForgeBackupBundleEntryHeader,
) -> Result<(), String> {
    encoder
        .write_all(b"DX_ENTRY ")
        .map_err(|error| format!("Failed to write Forge backup entry prefix: {error}"))?;
    serde_json::to_writer(&mut *encoder, header)
        .map_err(|error| format!("Failed to write Forge backup entry header: {error}"))?;
    encoder
        .write_all(b"\n")
        .map_err(|error| format!("Failed to write Forge backup entry separator: {error}"))
}

fn apply_quarantine_if_requested(
    request: &DxForgeBackupExecutionRequest,
    gate: &DxForgeBackupExecutionGateSummary,
    target_path: &Path,
    quarantine_path: Option<&Path>,
) -> Result<bool, String> {
    if gate.operation != "delete" {
        return Ok(false);
    }
    if !request.apply_quarantine_after_backup {
        return Ok(false);
    }
    let quarantine_path = quarantine_path
        .ok_or_else(|| "Delete-to-quarantine execution needs a quarantine path.".to_string())?;
    if let Some(parent) = quarantine_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to prepare Forge quarantine directory {}: {error}",
                parent.display()
            )
        })?;
    }
    fs::rename(target_path, quarantine_path).map_err(|error| {
        format!(
            "Failed to quarantine {} to {} after backup: {error}",
            target_path.display(),
            quarantine_path.display()
        )
    })?;
    Ok(true)
}

fn create_new_file(path: &Path, label: &str) -> Result<File, String> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("Failed to create {label} {}: {error}", path.display()))
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|error| format!("Failed to hash {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Failed to read {} for hashing: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_digest(&hasher.finalize()))
}

fn root_hash_for_entries(entries: &[DxForgeBackupManifestEntry]) -> String {
    let mut hasher = Sha256::new();
    for entry in entries {
        hasher.update(entry.path.as_bytes());
        hasher.update(entry.kind.as_bytes());
        hasher.update(entry.size_bytes.to_le_bytes());
        if let Some(sha256) = &entry.sha256 {
            hasher.update(sha256.as_bytes());
        }
    }
    hex_digest(&hasher.finalize())
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn summarize_gate(gate: &Value) -> Result<DxForgeBackupExecutionGateSummary, String> {
    let operation = string_field(gate, &["policy", "operation"])
        .ok_or_else(|| "Forge runner gate is missing policy.operation.".to_string())?;
    let target_path = string_field(gate, &["policy", "target_path"])
        .ok_or_else(|| "Forge runner gate is missing policy.target_path.".to_string())?;
    let planned_archive_path = string_field(gate, &["policy", "planned_archive_path"])
        .ok_or_else(|| "Forge runner gate is missing policy.planned_archive_path.".to_string())?;
    let planned_manifest_path = string_field(gate, &["policy", "planned_manifest_path"])
        .ok_or_else(|| "Forge runner gate is missing policy.planned_manifest_path.".to_string())?;

    Ok(DxForgeBackupExecutionGateSummary {
        schema: string_field(gate, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        gate_status: string_field(gate, &["validation", "status"])
            .unwrap_or_else(|| "unknown".to_string()),
        runner_ready: bool_field(gate, &["validation", "runner_ready"]).unwrap_or(false),
        operation,
        target_path,
        destination_path: string_field(gate, &["policy", "destination_path"]),
        planned_archive_path,
        planned_manifest_path,
        planned_quarantine_path: string_field(gate, &["policy", "planned_quarantine_path"]),
        backup_paths_managed: bool_field(gate, &["validation", "backup_paths_managed"])
            .unwrap_or(false),
        no_permanent_delete_enforced: bool_field(
            gate,
            &["validation", "no_permanent_delete_enforced"],
        )
        .unwrap_or(false),
        restore_receipt_required: bool_field(gate, &["validation", "restore_receipt_required"])
            .unwrap_or(false),
    })
}

fn locate_runner_gate(value: &Value) -> Option<&Value> {
    if value.get("schema").and_then(Value::as_str) == Some(DX_FORGE_BACKUP_RUNNER_GATE_SCHEMA) {
        return Some(value);
    }

    value.get("runner_gate").filter(|gate| {
        gate.get("schema").and_then(Value::as_str) == Some(DX_FORGE_BACKUP_RUNNER_GATE_SCHEMA)
    })
}

fn decode_json_string(value: &Value) -> Result<Value, String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if text.starts_with('{') {
            return serde_json::from_str(text).map_err(|error| {
                format!("Failed to parse stringified DX Forge backup runner gate JSON: {error}")
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
