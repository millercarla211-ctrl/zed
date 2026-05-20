use serde::Serialize;
use serde_json::Value;
use std::{
    cmp::Ordering,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const DX_SOURCE_ATTACHMENT_SCHEMA: &str = "zed.dx.sources.attachment.v1";
pub(crate) const DX_SOURCE_ATTACHMENT_RECEIPT_SCHEMA: &str = "zed.dx.sources.attachment_receipt.v1";

const MAX_RECEIPT_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug)]
pub(crate) struct DxSourceAttachmentRequest {
    pub workspace_roots: Vec<PathBuf>,
    pub selection: DxSourceAttachmentSelection,
    pub max_sources_per_set: usize,
    pub write_attachment_receipt: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug)]
pub(crate) struct DxSourceAttachmentSelection {
    pub workspace_roots: bool,
    pub metasearch_source_packs: bool,
    pub media_outputs: bool,
    pub forge_restore_previews: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSourceAttachment {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxSourceAttachmentRequestSummary,
    pub summary: DxSourceAttachmentSummary,
    pub source_sets: Vec<DxSourceAttachmentSet>,
    pub source_attachment_receipt: Option<DxSourceAttachmentReceipt>,
    pub safety: DxSourceAttachmentSafety,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSourceAttachmentRequestSummary {
    pub root_mode: String,
    pub max_sources_per_set: usize,
    pub selected_sets: Vec<&'static str>,
    pub workspace_root_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSourceAttachmentSummary {
    pub status: &'static str,
    pub source_set_count: usize,
    pub source_count: usize,
    pub receipt_source_count: usize,
    pub file_source_count: usize,
    pub directory_source_count: usize,
    pub estimated_tokens: usize,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSourceAttachmentSet {
    pub id: &'static str,
    pub label: &'static str,
    pub status: String,
    pub source_count: usize,
    pub sources: Vec<DxSourceAttachmentItem>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSourceAttachmentItem {
    pub id: String,
    pub label: String,
    pub kind: &'static str,
    pub attach_as: &'static str,
    pub path: String,
    pub detail: String,
    pub estimated_tokens: usize,
    pub binary_payload_embedded: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSourceAttachmentSafety {
    pub reads_managed_receipts: bool,
    pub writes_attachment_receipt: bool,
    pub runs_external_process: bool,
    pub runs_shell: bool,
    pub fetches_network: bool,
    pub embeds_binary_payloads: bool,
    pub mutates_workspace_sources: bool,
    pub dispatches_browser_input: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSourceAttachmentReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub attachment_schema: &'static str,
    pub source_count: usize,
    pub estimated_tokens: usize,
    pub next_action: String,
}

pub(crate) fn prepare_dx_source_attachment(
    request: DxSourceAttachmentRequest,
) -> DxSourceAttachment {
    let max_sources_per_set = request.max_sources_per_set.clamp(1, 12);
    let workspace_roots = request
        .workspace_roots
        .into_iter()
        .take(4)
        .collect::<Vec<_>>();
    let mut source_sets = Vec::new();

    if request.selection.workspace_roots {
        source_sets.push(workspace_root_set(&workspace_roots, max_sources_per_set));
    }
    if request.selection.metasearch_source_packs {
        source_sets.push(metasearch_source_pack_set(
            &workspace_roots,
            max_sources_per_set,
        ));
    }
    if request.selection.media_outputs {
        source_sets.push(media_output_set(&workspace_roots, max_sources_per_set));
    }
    if request.selection.forge_restore_previews {
        source_sets.push(forge_restore_preview_set(
            &workspace_roots,
            max_sources_per_set,
        ));
    }

    let source_count = source_sets
        .iter()
        .map(|set| set.sources.len())
        .sum::<usize>();
    let receipt_source_count = source_sets
        .iter()
        .flat_map(|set| set.sources.iter())
        .filter(|source| source.attach_as == "receipt")
        .count();
    let file_source_count = source_sets
        .iter()
        .flat_map(|set| set.sources.iter())
        .filter(|source| source.attach_as == "file")
        .count();
    let directory_source_count = source_sets
        .iter()
        .flat_map(|set| set.sources.iter())
        .filter(|source| source.attach_as == "directory")
        .count();
    let estimated_tokens = source_sets
        .iter()
        .flat_map(|set| set.sources.iter())
        .map(|source| source.estimated_tokens)
        .sum();
    let mut blockers = Vec::new();
    if workspace_roots.is_empty() {
        blockers.push("No visible workspace root is available for source attachment.".to_string());
    }
    if selected_set_labels(&request.selection).is_empty() {
        blockers.push("No DX source attachment sets were selected.".to_string());
    }
    if source_count == 0 {
        blockers
            .push("No attachable DX sources were discovered for the selected sets.".to_string());
    }
    let status = if blockers.is_empty() {
        "ready"
    } else if source_count > 0 {
        "partial"
    } else {
        "empty"
    };

    DxSourceAttachment {
        schema: DX_SOURCE_ATTACHMENT_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxSourceAttachmentRequestSummary {
            root_mode: request.root_mode,
            max_sources_per_set,
            selected_sets: selected_set_labels(&request.selection),
            workspace_root_count: workspace_roots.len(),
        },
        summary: DxSourceAttachmentSummary {
            status,
            source_set_count: source_sets.len(),
            source_count,
            receipt_source_count,
            file_source_count,
            directory_source_count,
            estimated_tokens,
            blockers,
        },
        source_sets,
        source_attachment_receipt: None,
        safety: DxSourceAttachmentSafety {
            reads_managed_receipts: true,
            writes_attachment_receipt: request.write_attachment_receipt,
            runs_external_process: false,
            runs_shell: false,
            fetches_network: false,
            embeds_binary_payloads: false,
            mutates_workspace_sources: false,
            dispatches_browser_input: false,
        },
        next_action: "Use this attachment receipt as the source manifest for the next Agent turn; keep binary media referenced by path, not pasted into model context."
            .to_string(),
    }
}

fn workspace_root_set(roots: &[PathBuf], max_sources: usize) -> DxSourceAttachmentSet {
    let sources = roots
        .iter()
        .take(max_sources)
        .enumerate()
        .map(|(index, root)| DxSourceAttachmentItem {
            id: format!("workspace-root-{index}"),
            label: display_name(root),
            kind: "workspace_root",
            attach_as: "directory",
            path: root.display().to_string(),
            detail: "Visible workspace root".to_string(),
            estimated_tokens: 0,
            binary_payload_embedded: false,
        })
        .collect::<Vec<_>>();

    attachment_set(
        "workspace",
        "Workspace",
        if roots.is_empty() {
            "No workspace root".to_string()
        } else {
            format!("{} root(s)", sources.len())
        },
        sources,
    )
}

fn metasearch_source_pack_set(roots: &[PathBuf], max_sources: usize) -> DxSourceAttachmentSet {
    let mut sources = Vec::new();
    for root in roots {
        let receipt_root = root
            .join("tools")
            .join("dx-metasearch")
            .join("source-packs");
        for receipt in latest_receipts(root, &receipt_root, max_sources) {
            if let Some(source) = metasearch_source_from_receipt(&receipt) {
                sources.push(source);
            }
        }
    }
    sources.truncate(max_sources);

    attachment_set(
        "metasearch",
        "Metasearch",
        discovered_status(roots, &sources, "No source-pack receipts"),
        sources,
    )
}

fn media_output_set(roots: &[PathBuf], max_sources: usize) -> DxSourceAttachmentSet {
    let mut sources = Vec::new();
    for root in roots {
        let receipt_root = root.join("tools").join("dx-media").join("executions");
        for receipt in latest_receipts(root, &receipt_root, max_sources) {
            sources.extend(
                media_sources_from_receipt(&receipt)
                    .into_iter()
                    .take(max_sources),
            );
        }
    }
    sources.truncate(max_sources);

    attachment_set(
        "media_outputs",
        "Media Outputs",
        discovered_status(roots, &sources, "No produced media outputs"),
        sources,
    )
}

fn forge_restore_preview_set(roots: &[PathBuf], max_sources: usize) -> DxSourceAttachmentSet {
    let mut sources = Vec::new();
    for root in roots {
        let receipt_root = root.join("tools").join("dx-forge").join("restores");
        for receipt in latest_receipts(root, &receipt_root, max_sources) {
            if let Some(source) = forge_restore_source_from_receipt(&receipt) {
                sources.push(source);
            }
        }
    }
    sources.truncate(max_sources);

    attachment_set(
        "forge_restore_previews",
        "Forge Restore Previews",
        discovered_status(roots, &sources, "No restore previews"),
        sources,
    )
}

fn attachment_set(
    id: &'static str,
    label: &'static str,
    status: String,
    sources: Vec<DxSourceAttachmentItem>,
) -> DxSourceAttachmentSet {
    DxSourceAttachmentSet {
        id,
        label,
        status,
        source_count: sources.len(),
        sources,
    }
}

fn discovered_status(
    roots: &[PathBuf],
    sources: &[DxSourceAttachmentItem],
    empty_label: &'static str,
) -> String {
    if roots.is_empty() {
        "No workspace".to_string()
    } else if sources.is_empty() {
        empty_label.to_string()
    } else {
        format!("{} source(s)", sources.len())
    }
}

fn metasearch_source_from_receipt(receipt: &ReceiptCandidate) -> Option<DxSourceAttachmentItem> {
    let value = read_receipt_json(&receipt.path)?;
    let source_pack = value.get("source_pack")?;
    let query = string_at(source_pack, &["query"]).unwrap_or_else(|| "metasearch".to_string());
    let item_count = usize_at(source_pack, &["item_count"]).unwrap_or_default();
    let estimated_tokens = usize_at(source_pack, &["estimated_tokens"]).unwrap_or_default();

    Some(DxSourceAttachmentItem {
        id: format!(
            "metasearch-source-pack-{}",
            stable_id_fragment(&receipt.label)
        ),
        label: format!("Search: {query}"),
        kind: "metasearch_source_pack",
        attach_as: "receipt",
        path: receipt.path.display().to_string(),
        detail: format!("{item_count} items - ~{estimated_tokens} tokens"),
        estimated_tokens,
        binary_payload_embedded: false,
    })
}

fn media_sources_from_receipt(receipt: &ReceiptCandidate) -> Vec<DxSourceAttachmentItem> {
    let Some(value) = read_receipt_json(&receipt.path) else {
        return Vec::new();
    };
    let Some(files) = value
        .get("produced_files")
        .or_else(|| {
            value
                .get("media_execution")
                .and_then(|execution| execution.get("produced_files"))
        })
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    files
        .iter()
        .filter_map(|file| {
            let path = string_at(file, &["path"])?;
            let exists = bool_at(file, &["exists"]).unwrap_or_else(|| Path::new(&path).is_file());
            if !exists {
                return None;
            }

            let label = Path::new(&path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("media output")
                .to_string();
            let media_kind =
                string_at(file, &["media_kind"]).unwrap_or_else(|| "media".to_string());
            let format = string_at(file, &["format"]).unwrap_or_else(|| "output".to_string());
            let size_bytes = u64_at(file, &["size_bytes"]).unwrap_or_default();

            Some(DxSourceAttachmentItem {
                id: format!("media-output-{}", stable_id_fragment(&path)),
                label,
                kind: "media_output",
                attach_as: "file",
                path,
                detail: format!("{media_kind} - {format} - {}", format_bytes(size_bytes)),
                estimated_tokens: 0,
                binary_payload_embedded: false,
            })
        })
        .collect()
}

fn forge_restore_source_from_receipt(receipt: &ReceiptCandidate) -> Option<DxSourceAttachmentItem> {
    let value = read_receipt_json(&receipt.path)?;
    let restore_root = string_at(&value, &["restore_destination_root"]).or_else(|| {
        string_at(
            &value,
            &["restore_execution", "restore", "restore_destination_root"],
        )
    })?;
    let restored_file_count = usize_at(&value, &["restored_file_count"])
        .or_else(|| {
            usize_at(
                &value,
                &["restore_execution", "restore", "restored_file_count"],
            )
        })
        .unwrap_or_default();
    let restored_bytes = u64_at(&value, &["restored_total_file_bytes"])
        .or_else(|| {
            u64_at(
                &value,
                &["restore_execution", "restore", "restored_total_file_bytes"],
            )
        })
        .unwrap_or_default();

    Some(DxSourceAttachmentItem {
        id: format!("forge-restore-{}", stable_id_fragment(&restore_root)),
        label: display_name(Path::new(&restore_root)),
        kind: "forge_restore_preview",
        attach_as: "directory",
        path: restore_root,
        detail: format!(
            "{restored_file_count} restored files - {}",
            format_bytes(restored_bytes)
        ),
        estimated_tokens: 0,
        binary_payload_embedded: false,
    })
}

#[derive(Clone)]
struct ReceiptCandidate {
    path: PathBuf,
    label: String,
    modified: SystemTime,
}

fn latest_receipts(
    workspace_root: &Path,
    receipt_root: &Path,
    limit: usize,
) -> Vec<ReceiptCandidate> {
    let Ok(entries) = fs::read_dir(receipt_root) else {
        return Vec::new();
    };

    let mut receipts = Vec::new();
    for entry in entries.flatten().take(128) {
        let path = entry.path();
        if path.is_file() && is_receipt_file(&path) {
            let modified = path
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let label = path
                .strip_prefix(workspace_root)
                .unwrap_or(path.as_path())
                .display()
                .to_string();
            receipts.push(ReceiptCandidate {
                path,
                label,
                modified,
            });
        }
    }

    receipts.sort_by(|left, right| {
        right
            .modified
            .partial_cmp(&left.modified)
            .unwrap_or(Ordering::Equal)
    });
    receipts.truncate(limit);
    receipts
}

fn read_receipt_json(path: &Path) -> Option<Value> {
    let mut file = File::open(path).ok()?;
    let mut buffer = Vec::new();
    file.by_ref()
        .take(MAX_RECEIPT_BYTES)
        .read_to_end(&mut buffer)
        .ok()?;
    serde_json::from_slice(&buffer).ok()
}

fn selected_set_labels(selection: &DxSourceAttachmentSelection) -> Vec<&'static str> {
    let mut labels = Vec::new();
    if selection.workspace_roots {
        labels.push("workspace");
    }
    if selection.metasearch_source_packs {
        labels.push("metasearch");
    }
    if selection.media_outputs {
        labels.push("media_outputs");
    }
    if selection.forge_restore_previews {
        labels.push("forge_restore_previews");
    }
    labels
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

fn string_at(value: &Value, path: &[&str]) -> Option<String> {
    value_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn bool_at(value: &Value, path: &[&str]) -> Option<bool> {
    value_at(value, path).and_then(Value::as_bool)
}

fn usize_at(value: &Value, path: &[&str]) -> Option<usize> {
    value_at(value, path)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn u64_at(value: &Value, path: &[&str]) -> Option<u64> {
    value_at(value, path).and_then(Value::as_u64)
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn stable_id_fragment(value: &str) -> String {
    let fragment = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(24)
        .collect::<String>()
        .to_ascii_lowercase();
    if fragment.is_empty() {
        "source".to_string()
    } else {
        fragment
    }
}

fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;

    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn is_receipt_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "jsonl" | "receipt")
    )
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
