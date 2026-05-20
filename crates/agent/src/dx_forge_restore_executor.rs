use crate::dx_forge_backup_executor::{
    DX_FORGE_BACKUP_BUNDLE_FORMAT, DX_FORGE_BACKUP_EXECUTION_SCHEMA,
    DX_FORGE_BACKUP_MANIFEST_SCHEMA,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const DX_FORGE_RESTORE_EXECUTION_SCHEMA: &str = "zed.dx.forge.restore_execution.v1";
pub(crate) const DX_FORGE_RESTORE_EXECUTION_RECEIPT_SCHEMA: &str =
    "zed.dx.forge.restore_execution_receipt.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxForgeRestoreExecutionRequest {
    pub backup_execution: Value,
    pub approve_restore: bool,
    pub require_restore_receipt: bool,
    pub restore_destination_root: PathBuf,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreExecution {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxForgeRestoreExecutionRequestSummary,
    pub backup: DxForgeRestoreBackupSummary,
    pub restore: DxForgeRestoreSummary,
    pub manifest: DxForgeRestoreManifestSummary,
    pub restore_receipt: Option<DxForgeRestoreExecutionReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreExecutionRequestSummary {
    pub approve_restore: bool,
    pub require_restore_receipt: bool,
    pub restore_destination_root: String,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreBackupSummary {
    pub schema: String,
    pub status: String,
    pub operation: String,
    pub target_path: String,
    pub archive_path: String,
    pub manifest_path: String,
    pub quarantine_path: Option<String>,
    pub wrote_backup_archive: bool,
    pub wrote_manifest: bool,
    pub target_mutation_applied: bool,
    pub permanent_delete_performed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreSummary {
    pub status: String,
    pub restore_ready: bool,
    pub permission_required: bool,
    pub restore_approved: bool,
    pub wrote_restore_files: bool,
    pub overwrote_existing_files: bool,
    pub target_mutation_applied: bool,
    pub ran_shell: bool,
    pub ran_external_process: bool,
    pub used_native_zstd_library: bool,
    pub restore_destination_root: String,
    pub restored_file_count: usize,
    pub restored_directory_count: usize,
    pub restored_total_file_bytes: u64,
    pub verified_file_count: usize,
    pub verified_root_sha256: String,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreManifestSummary {
    pub schema: String,
    pub archive_format: String,
    pub compression: String,
    pub entry_count: usize,
    pub file_count: usize,
    pub directory_count: usize,
    pub total_file_bytes: u64,
    pub root_sha256: String,
    pub restored_root_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxForgeRestoreExecutionReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub restore_schema: &'static str,
    pub backup_archive_path: String,
    pub backup_manifest_path: String,
    pub restore_destination_root: String,
    pub restored_file_count: usize,
    pub restored_directory_count: usize,
    pub restored_total_file_bytes: u64,
    pub next_action: String,
}

#[derive(Clone, Debug, Deserialize)]
struct DxForgeBackupManifestOnDisk {
    schema: String,
    archive_format: String,
    compression: String,
    operation: String,
    target_path: String,
    archive_path: String,
    entries: Vec<DxForgeBackupManifestEntryOnDisk>,
    total_file_bytes: u64,
    root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DxForgeBackupManifestEntryOnDisk {
    path: String,
    kind: String,
    size_bytes: u64,
    sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct DxForgeBackupBundleHeaderOnDisk {
    format: String,
    operation: String,
    target_path: String,
}

#[derive(Clone, Debug, Deserialize)]
struct DxForgeBackupBundleEntryHeaderOnDisk {
    path: String,
    kind: String,
    size_bytes: u64,
    sha256: Option<String>,
}

pub(crate) fn execute_dx_forge_restore(
    request: DxForgeRestoreExecutionRequest,
) -> Result<DxForgeRestoreExecution, String> {
    let backup_value = decode_json_string(&request.backup_execution)?;
    let backup = locate_backup_execution(&backup_value).ok_or_else(|| {
        "DX Forge restore needs a zed.dx.forge.backup_execution.v1 object or execution receipt."
            .to_string()
    })?;
    let backup_summary = summarize_backup_execution(backup)?;
    let mut blockers = restore_blockers(&request, &backup_summary);
    let mut manifest_summary = DxForgeRestoreManifestSummary {
        schema: DX_FORGE_BACKUP_MANIFEST_SCHEMA.to_string(),
        archive_format: DX_FORGE_BACKUP_BUNDLE_FORMAT.to_string(),
        compression: "zstd".to_string(),
        entry_count: 0,
        file_count: 0,
        directory_count: 0,
        total_file_bytes: 0,
        root_sha256: String::new(),
        restored_root_sha256: String::new(),
    };

    if blockers.is_empty() {
        match restore_from_backup(&backup_summary, &request.restore_destination_root) {
            Ok(restored) => {
                manifest_summary = restored.manifest.clone();
                return Ok(success_response(
                    request,
                    backup_summary,
                    restored,
                    manifest_summary,
                ));
            }
            Err(error) => blockers.push(error),
        }
    }

    Ok(blocked_response(
        request,
        backup_summary,
        manifest_summary,
        blockers,
    ))
}

fn success_response(
    request: DxForgeRestoreExecutionRequest,
    backup: DxForgeRestoreBackupSummary,
    restored: RestoredBackup,
    manifest: DxForgeRestoreManifestSummary,
) -> DxForgeRestoreExecution {
    DxForgeRestoreExecution {
        schema: DX_FORGE_RESTORE_EXECUTION_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: request_summary(&request),
        backup,
        restore: DxForgeRestoreSummary {
            status: "restore_preview_written".to_string(),
            restore_ready: true,
            permission_required: true,
            restore_approved: true,
            wrote_restore_files: true,
            overwrote_existing_files: false,
            target_mutation_applied: false,
            ran_shell: false,
            ran_external_process: false,
            used_native_zstd_library: true,
            restore_destination_root: request.restore_destination_root.display().to_string(),
            restored_file_count: restored.file_count,
            restored_directory_count: restored.directory_count,
            restored_total_file_bytes: restored.total_file_bytes,
            verified_file_count: restored.file_count,
            verified_root_sha256: manifest.restored_root_sha256.clone(),
            blockers: Vec::new(),
        },
        manifest,
        restore_receipt: None,
        next_action:
            "Review the managed restore preview, then use the receipt for Forge panel history or a future explicit restore-to-target flow."
                .to_string(),
    }
}

fn blocked_response(
    request: DxForgeRestoreExecutionRequest,
    backup: DxForgeRestoreBackupSummary,
    manifest: DxForgeRestoreManifestSummary,
    blockers: Vec<String>,
) -> DxForgeRestoreExecution {
    DxForgeRestoreExecution {
        schema: DX_FORGE_RESTORE_EXECUTION_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: request_summary(&request),
        backup,
        restore: DxForgeRestoreSummary {
            status: if request.approve_restore {
                "blocked_after_approval".to_string()
            } else {
                "approval_required".to_string()
            },
            restore_ready: false,
            permission_required: true,
            restore_approved: request.approve_restore,
            wrote_restore_files: false,
            overwrote_existing_files: false,
            target_mutation_applied: false,
            ran_shell: false,
            ran_external_process: false,
            used_native_zstd_library: true,
            restore_destination_root: request.restore_destination_root.display().to_string(),
            restored_file_count: 0,
            restored_directory_count: 0,
            restored_total_file_bytes: 0,
            verified_file_count: 0,
            verified_root_sha256: String::new(),
            blockers,
        },
        manifest,
        restore_receipt: None,
        next_action:
            "Resolve the listed Forge restore blockers before writing a managed restore preview."
                .to_string(),
    }
}

fn request_summary(
    request: &DxForgeRestoreExecutionRequest,
) -> DxForgeRestoreExecutionRequestSummary {
    DxForgeRestoreExecutionRequestSummary {
        approve_restore: request.approve_restore,
        require_restore_receipt: request.require_restore_receipt,
        restore_destination_root: request.restore_destination_root.display().to_string(),
        root_mode: request.root_mode.clone(),
    }
}

fn restore_blockers(
    request: &DxForgeRestoreExecutionRequest,
    backup: &DxForgeRestoreBackupSummary,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !request.approve_restore {
        blockers.push("Forge restore execution has not been approved.".to_string());
    }
    if !request.require_restore_receipt {
        blockers.push("Forge restore execution requires a restore receipt.".to_string());
    }
    if backup.schema != DX_FORGE_BACKUP_EXECUTION_SCHEMA {
        blockers.push(format!(
            "Expected backup execution schema {DX_FORGE_BACKUP_EXECUTION_SCHEMA}, got {}.",
            backup.schema
        ));
    }
    if !backup.status.starts_with("backup_written") {
        blockers.push(
            "Forge restore requires a backup execution whose status starts with backup_written."
                .to_string(),
        );
    }
    if !backup.wrote_backup_archive || !backup.wrote_manifest {
        blockers
            .push("Forge restore requires written backup archive and manifest flags.".to_string());
    }
    if backup.permanent_delete_performed {
        blockers.push(
            "Refusing to restore from an execution that reports permanent delete.".to_string(),
        );
    }
    if backup.archive_path.trim().is_empty() || backup.manifest_path.trim().is_empty() {
        blockers.push("Forge restore archive and manifest paths cannot be empty.".to_string());
    }
    if !Path::new(&backup.archive_path).is_file() {
        blockers.push("Forge restore backup archive does not exist.".to_string());
    }
    if !Path::new(&backup.manifest_path).is_file() {
        blockers.push("Forge restore manifest does not exist.".to_string());
    }
    if request.restore_destination_root.as_os_str().is_empty() {
        blockers.push("Forge restore destination root cannot be empty.".to_string());
    }
    if request.restore_destination_root.exists() {
        blockers.push(
            "Forge restore destination root already exists; refusing to overwrite previews."
                .to_string(),
        );
    }

    blockers
}

struct RestoredBackup {
    manifest: DxForgeRestoreManifestSummary,
    file_count: usize,
    directory_count: usize,
    total_file_bytes: u64,
}

fn restore_from_backup(
    backup: &DxForgeRestoreBackupSummary,
    restore_destination_root: &Path,
) -> Result<RestoredBackup, String> {
    let archive_path = Path::new(&backup.archive_path);
    let manifest_path = Path::new(&backup.manifest_path);
    let manifest_json = fs::read(manifest_path).map_err(|error| {
        format!(
            "Failed to read Forge restore manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    let manifest: DxForgeBackupManifestOnDisk = serde_json::from_slice(&manifest_json)
        .map_err(|error| format!("Failed to parse Forge backup manifest: {error}"))?;
    validate_manifest(backup, &manifest)?;

    create_restore_destination_root(restore_destination_root)?;

    let archive_file = File::open(archive_path).map_err(|error| {
        format!(
            "Failed to open Forge backup archive {}: {error}",
            archive_path.display()
        )
    })?;
    let decoder = zstd::stream::Decoder::new(archive_file).map_err(|error| {
        format!(
            "Failed to initialize zstd decoder for {}: {error}",
            archive_path.display()
        )
    })?;
    let mut reader = BufReader::new(decoder);
    let bundle_header = read_bundle_header(&mut reader)?;
    validate_bundle_header(backup, &manifest, &bundle_header)?;

    let mut restored_entries = Vec::with_capacity(manifest.entries.len());
    for expected_entry in &manifest.entries {
        let entry_header = read_entry_header(&mut reader)?;
        validate_entry_header(expected_entry, &entry_header)?;
        restore_entry(
            &mut reader,
            restore_destination_root,
            expected_entry,
            &mut restored_entries,
        )?;
    }
    validate_no_extra_entries(&mut reader)?;

    let restored_root_sha256 = root_hash_for_entries(&restored_entries);
    if restored_root_sha256 != manifest.root_sha256 {
        return Err(format!(
            "Restored Forge root hash mismatch: expected {}, got {restored_root_sha256}.",
            manifest.root_sha256
        ));
    }

    let file_count = restored_entries
        .iter()
        .filter(|entry| entry.kind == "file")
        .count();
    let directory_count = restored_entries
        .iter()
        .filter(|entry| entry.kind == "directory")
        .count();
    let total_file_bytes = restored_entries.iter().map(|entry| entry.size_bytes).sum();

    Ok(RestoredBackup {
        manifest: DxForgeRestoreManifestSummary {
            schema: manifest.schema,
            archive_format: manifest.archive_format,
            compression: manifest.compression,
            entry_count: restored_entries.len(),
            file_count,
            directory_count,
            total_file_bytes,
            root_sha256: manifest.root_sha256,
            restored_root_sha256,
        },
        file_count,
        directory_count,
        total_file_bytes,
    })
}

fn validate_manifest(
    backup: &DxForgeRestoreBackupSummary,
    manifest: &DxForgeBackupManifestOnDisk,
) -> Result<(), String> {
    if manifest.schema != DX_FORGE_BACKUP_MANIFEST_SCHEMA {
        return Err(format!(
            "Expected Forge backup manifest schema {DX_FORGE_BACKUP_MANIFEST_SCHEMA}, got {}.",
            manifest.schema
        ));
    }
    if manifest.archive_format != DX_FORGE_BACKUP_BUNDLE_FORMAT {
        return Err(format!(
            "Expected Forge backup format {DX_FORGE_BACKUP_BUNDLE_FORMAT}, got {}.",
            manifest.archive_format
        ));
    }
    if manifest.compression != "zstd" {
        return Err(format!(
            "Expected Forge backup compression zstd, got {}.",
            manifest.compression
        ));
    }
    if manifest.operation != backup.operation {
        return Err(format!(
            "Forge backup operation mismatch: manifest {}, execution {}.",
            manifest.operation, backup.operation
        ));
    }
    if manifest.target_path != backup.target_path {
        return Err("Forge backup manifest target path does not match execution.".to_string());
    }
    if PathBuf::from(&manifest.archive_path) != PathBuf::from(&backup.archive_path) {
        return Err("Forge backup manifest archive path does not match execution.".to_string());
    }
    let manifest_total_bytes = manifest
        .entries
        .iter()
        .map(|entry| entry.size_bytes)
        .sum::<u64>();
    if manifest_total_bytes != manifest.total_file_bytes {
        return Err("Forge backup manifest total_file_bytes does not match entries.".to_string());
    }
    if root_hash_for_entries(&manifest.entries) != manifest.root_sha256 {
        return Err("Forge backup manifest root hash does not match entries.".to_string());
    }
    let mut seen_paths = HashSet::with_capacity(manifest.entries.len());
    for entry in &manifest.entries {
        safe_relative_path(&entry.path)?;
        if !seen_paths.insert(entry.path.clone()) {
            return Err(format!(
                "Forge backup manifest contains duplicate entry path {}.",
                entry.path
            ));
        }
        match entry.kind.as_str() {
            "directory" if entry.size_bytes == 0 && entry.sha256.is_none() => {}
            "file" if entry.sha256.is_some() => {}
            "directory" => {
                return Err(format!(
                    "Forge backup manifest directory entry {} has invalid size or hash fields.",
                    entry.path
                ));
            }
            "file" => {
                return Err(format!(
                    "Forge backup manifest file entry {} is missing a hash.",
                    entry.path
                ));
            }
            other => {
                return Err(format!(
                    "Forge backup manifest entry {} has unsupported kind {other}.",
                    entry.path
                ));
            }
        }
    }

    Ok(())
}

fn create_restore_destination_root(restore_destination_root: &Path) -> Result<(), String> {
    if let Some(parent) = restore_destination_root.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to prepare Forge restore parent directory {}: {error}",
                parent.display()
            )
        })?;
    }
    fs::create_dir(restore_destination_root).map_err(|error| {
        format!(
            "Failed to create Forge restore destination {}: {error}",
            restore_destination_root.display()
        )
    })
}

fn validate_bundle_header(
    backup: &DxForgeRestoreBackupSummary,
    manifest: &DxForgeBackupManifestOnDisk,
    header: &DxForgeBackupBundleHeaderOnDisk,
) -> Result<(), String> {
    if header.format != DX_FORGE_BACKUP_BUNDLE_FORMAT {
        return Err(format!(
            "Expected Forge backup bundle format {DX_FORGE_BACKUP_BUNDLE_FORMAT}, got {}.",
            header.format
        ));
    }
    if header.operation != backup.operation || header.operation != manifest.operation {
        return Err("Forge backup bundle operation does not match manifest/execution.".to_string());
    }
    if header.target_path != backup.target_path || header.target_path != manifest.target_path {
        return Err(
            "Forge backup bundle target path does not match manifest/execution.".to_string(),
        );
    }

    Ok(())
}

fn validate_entry_header(
    expected: &DxForgeBackupManifestEntryOnDisk,
    header: &DxForgeBackupBundleEntryHeaderOnDisk,
) -> Result<(), String> {
    if expected.path != header.path
        || expected.kind != header.kind
        || expected.size_bytes != header.size_bytes
        || expected.sha256 != header.sha256
    {
        return Err(format!(
            "Forge backup entry header mismatch for expected path {}.",
            expected.path
        ));
    }

    Ok(())
}

fn restore_entry<R: BufRead>(
    reader: &mut R,
    restore_destination_root: &Path,
    entry: &DxForgeBackupManifestEntryOnDisk,
    restored_entries: &mut Vec<DxForgeBackupManifestEntryOnDisk>,
) -> Result<(), String> {
    let relative_path = safe_relative_path(&entry.path)?;
    let destination_path = restore_destination_root.join(relative_path);

    match entry.kind.as_str() {
        "directory" => {
            fs::create_dir_all(&destination_path).map_err(|error| {
                format!(
                    "Failed to restore Forge directory {}: {error}",
                    destination_path.display()
                )
            })?;
        }
        "file" => {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "Failed to prepare Forge restore directory {}: {error}",
                        parent.display()
                    )
                })?;
            }
            let restored_sha256 = restore_file(reader, &destination_path, entry.size_bytes)?;
            if entry.sha256.as_deref() != Some(restored_sha256.as_str()) {
                return Err(format!(
                    "Forge restore hash mismatch for {}: expected {:?}, got {restored_sha256}.",
                    entry.path, entry.sha256
                ));
            }
            read_file_terminator(reader, &entry.path)?;
        }
        other => {
            return Err(format!("Unsupported Forge backup entry kind {other}."));
        }
    }

    restored_entries.push(entry.clone());
    Ok(())
}

fn restore_file<R: Read>(
    reader: &mut R,
    destination_path: &Path,
    size_bytes: u64,
) -> Result<String, String> {
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination_path)
        .map_err(|error| {
            format!(
                "Failed to create Forge restored file {}: {error}",
                destination_path.display()
            )
        })?;
    let mut hasher = Sha256::new();
    let mut remaining = size_bytes;
    let mut buffer = [0_u8; 64 * 1024];

    while remaining > 0 {
        let read_len = remaining.min(buffer.len() as u64) as usize;
        reader
            .read_exact(&mut buffer[..read_len])
            .map_err(|error| {
                format!(
                    "Failed to read Forge backup bytes for {}: {error}",
                    destination_path.display()
                )
            })?;
        output.write_all(&buffer[..read_len]).map_err(|error| {
            format!(
                "Failed to write Forge restored file {}: {error}",
                destination_path.display()
            )
        })?;
        hasher.update(&buffer[..read_len]);
        remaining -= read_len as u64;
    }

    Ok(hex_digest(&hasher.finalize()))
}

fn read_file_terminator<R: Read>(reader: &mut R, path: &str) -> Result<(), String> {
    let mut terminator = [0_u8; 1];
    reader.read_exact(&mut terminator).map_err(|error| {
        format!("Failed to read Forge backup file terminator after {path}: {error}")
    })?;
    if terminator != [b'\n'] {
        return Err(format!(
            "Forge backup file terminator after {path} was not a newline."
        ));
    }

    Ok(())
}

fn read_bundle_header<R: BufRead>(
    reader: &mut R,
) -> Result<DxForgeBackupBundleHeaderOnDisk, String> {
    let mut line = String::new();
    let bytes = reader
        .read_line(&mut line)
        .map_err(|error| format!("Failed to read Forge backup bundle header: {error}"))?;
    if bytes == 0 {
        return Err("Forge backup bundle is empty.".to_string());
    }
    serde_json::from_str(trim_line(&line))
        .map_err(|error| format!("Failed to parse Forge backup bundle header: {error}"))
}

fn read_entry_header<R: BufRead>(
    reader: &mut R,
) -> Result<DxForgeBackupBundleEntryHeaderOnDisk, String> {
    let mut line = String::new();
    let bytes = reader
        .read_line(&mut line)
        .map_err(|error| format!("Failed to read Forge backup entry header: {error}"))?;
    if bytes == 0 {
        return Err("Forge backup ended before all manifest entries were restored.".to_string());
    }
    let line = trim_line(&line);
    let json = line
        .strip_prefix("DX_ENTRY ")
        .ok_or_else(|| "Forge backup entry header is missing DX_ENTRY prefix.".to_string())?;
    serde_json::from_str(json)
        .map_err(|error| format!("Failed to parse Forge backup entry header: {error}"))
}

fn validate_no_extra_entries<R: BufRead>(reader: &mut R) -> Result<(), String> {
    let mut trailing = String::new();
    let bytes = reader
        .read_line(&mut trailing)
        .map_err(|error| format!("Failed to inspect Forge backup trailing data: {error}"))?;
    if bytes != 0 && !trim_line(&trailing).is_empty() {
        return Err("Forge backup contains extra trailing entry data.".to_string());
    }

    Ok(())
}

fn safe_relative_path(path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("Forge restore entry path cannot be empty.".to_string());
    }
    let relative = Path::new(path);
    if relative.is_absolute() {
        return Err(format!(
            "Forge restore refuses absolute entry path {}.",
            relative.display()
        ));
    }
    let mut normalized = PathBuf::new();
    for component in relative.components() {
        match component {
            Component::Normal(segment) => normalized.push(segment),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "Forge restore refuses unsafe entry path {}.",
                    relative.display()
                ));
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err("Forge restore entry path normalized to empty.".to_string());
    }

    Ok(normalized)
}

fn summarize_backup_execution(value: &Value) -> Result<DxForgeRestoreBackupSummary, String> {
    Ok(DxForgeRestoreBackupSummary {
        schema: string_field(value, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        status: string_field(value, &["execution", "status"])
            .unwrap_or_else(|| "unknown".to_string()),
        operation: string_field(value, &["gate", "operation"])
            .ok_or_else(|| "Forge backup execution is missing gate.operation.".to_string())?,
        target_path: string_field(value, &["gate", "target_path"])
            .ok_or_else(|| "Forge backup execution is missing gate.target_path.".to_string())?,
        archive_path: string_field(value, &["execution", "archive_path"]).ok_or_else(|| {
            "Forge backup execution is missing execution.archive_path.".to_string()
        })?,
        manifest_path: string_field(value, &["execution", "manifest_path"]).ok_or_else(|| {
            "Forge backup execution is missing execution.manifest_path.".to_string()
        })?,
        quarantine_path: string_field(value, &["execution", "quarantine_path"]),
        wrote_backup_archive: bool_field(value, &["execution", "wrote_backup_archive"])
            .unwrap_or(false),
        wrote_manifest: bool_field(value, &["execution", "wrote_manifest"]).unwrap_or(false),
        target_mutation_applied: bool_field(value, &["execution", "target_mutation_applied"])
            .unwrap_or(false),
        permanent_delete_performed: bool_field(value, &["execution", "permanent_delete_performed"])
            .unwrap_or(false),
    })
}

fn locate_backup_execution(value: &Value) -> Option<&Value> {
    if value.get("schema").and_then(Value::as_str) == Some(DX_FORGE_BACKUP_EXECUTION_SCHEMA) {
        return Some(value);
    }

    value.get("backup_execution").filter(|execution| {
        execution.get("schema").and_then(Value::as_str) == Some(DX_FORGE_BACKUP_EXECUTION_SCHEMA)
    })
}

fn root_hash_for_entries(entries: &[DxForgeBackupManifestEntryOnDisk]) -> String {
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

fn decode_json_string(value: &Value) -> Result<Value, String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if text.starts_with('{') {
            return serde_json::from_str(text).map_err(|error| {
                format!("Failed to parse stringified DX Forge backup execution JSON: {error}")
            });
        }
    }

    Ok(value.clone())
}

fn trim_line(line: &str) -> &str {
    line.trim_end_matches(|character| character == '\r' || character == '\n')
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
