mod dx_editor_toolchain;
mod formatting;
mod receipt_fields;
mod receipts;
mod restore;

use self::dx_editor_toolchain::dx_editor_toolchain_set;
use self::formatting::{display_name, format_bytes, short_hash, source_set_status};
use self::receipt_fields::{array_strings_at, bool_at, string_at, u64_at, usize_at};
use self::receipts::{ReceiptCandidate, latest_receipts, read_receipt_json};
use self::restore::forge_restore_warnings;
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const SOURCE_SET_CACHE_TTL: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(crate) struct DxSourceSetSnapshot {
    pub sets: Vec<DxSourceSet>,
    pub total_sources: usize,
}

#[derive(Clone, Default)]
pub(crate) struct DxSourceAttachmentSummary {
    pub workspace_roots: usize,
    pub managed_receipts: usize,
    pub produced_files: usize,
    pub restore_previews: usize,
    pub attachable_sources: usize,
}

impl DxSourceSetSnapshot {
    pub(crate) fn attachment_summary(&self) -> DxSourceAttachmentSummary {
        let mut summary = DxSourceAttachmentSummary::default();

        for source in self.sets.iter().flat_map(|set| set.sources.iter()) {
            match source.kind {
                DxSourceKind::WorkspaceRoot => summary.workspace_roots += 1,
                DxSourceKind::MetasearchSourcePack | DxSourceKind::ReducedContextReceipt => {
                    summary.managed_receipts += 1;
                    summary.attachable_sources += 1;
                }
                DxSourceKind::MediaOutput => {
                    summary.produced_files += 1;
                    summary.attachable_sources += 1;
                }
                DxSourceKind::ForgeRestorePreview => {
                    summary.restore_previews += 1;
                    summary.attachable_sources += 1;
                }
                DxSourceKind::DxToolchainConfig => {
                    summary.attachable_sources += 1;
                }
            }
        }

        summary
    }
}

#[derive(Clone)]
pub(crate) struct DxSourceSet {
    pub label: &'static str,
    pub status: String,
    pub sources: Vec<DxSourceItem>,
}

#[derive(Clone)]
pub(crate) struct DxSourceItem {
    pub label: String,
    pub detail: String,
    pub path: String,
    pub kind: DxSourceKind,
    pub receipt_drilldowns: Vec<DxSourceReceiptDrilldown>,
    pub proofs: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct DxSourceReceiptDrilldown {
    pub label: String,
    pub detail: String,
}

#[derive(Clone, Copy)]
pub(crate) enum DxSourceKind {
    WorkspaceRoot,
    MetasearchSourcePack,
    ReducedContextReceipt,
    MediaOutput,
    ForgeRestorePreview,
    DxToolchainConfig,
}

static SOURCE_SET_CACHE: OnceLock<Mutex<Option<(Instant, Vec<String>, DxSourceSetSnapshot)>>> =
    OnceLock::new();

pub(crate) fn source_set_snapshot(workspace_roots: &[String]) -> DxSourceSetSnapshot {
    let cache = SOURCE_SET_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, cached_roots, snapshot)) = cache.as_ref() {
            if cached_roots == workspace_roots
                && now.duration_since(*cached_at) <= SOURCE_SET_CACHE_TTL
            {
                return snapshot.clone();
            }
        }

        let snapshot = scan_source_sets(workspace_roots);
        *cache = Some((now, workspace_roots.to_vec(), snapshot.clone()));
        return snapshot;
    }

    scan_source_sets(workspace_roots)
}

fn scan_source_sets(workspace_roots: &[String]) -> DxSourceSetSnapshot {
    let workspace_roots = workspace_roots
        .iter()
        .take(4)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let sets = vec![
        workspace_root_set(&workspace_roots),
        dx_editor_toolchain_set(&workspace_roots),
        metasearch_source_pack_set(&workspace_roots),
        reduced_context_set(&workspace_roots),
        media_output_set(&workspace_roots),
        forge_restore_preview_set(&workspace_roots),
    ];
    let total_sources = sets.iter().map(|set| set.sources.len()).sum();

    DxSourceSetSnapshot {
        sets,
        total_sources,
    }
}

fn workspace_root_set(workspace_roots: &[PathBuf]) -> DxSourceSet {
    let sources = workspace_roots
        .iter()
        .map(|root| DxSourceItem {
            label: display_name(root),
            detail: "Workspace root".to_string(),
            path: root.display().to_string(),
            kind: DxSourceKind::WorkspaceRoot,
            receipt_drilldowns: Vec::new(),
            proofs: Vec::new(),
            warnings: Vec::new(),
        })
        .collect::<Vec<_>>();

    DxSourceSet {
        label: "Workspace",
        status: if sources.is_empty() {
            "No workspace root".to_string()
        } else {
            format!("{} root(s)", sources.len())
        },
        sources,
    }
}

fn metasearch_source_pack_set(workspace_roots: &[PathBuf]) -> DxSourceSet {
    let mut sources = Vec::new();
    for root in workspace_roots {
        let receipt_root = root
            .join("tools")
            .join("dx-metasearch")
            .join("source-packs");
        for receipt in latest_receipts(root, &receipt_root, 4) {
            if let Some(source) = metasearch_source_from_receipt(&receipt) {
                sources.push(source);
            }
        }
    }
    sources.truncate(4);

    DxSourceSet {
        label: "Metasearch",
        status: source_set_status(workspace_roots, &sources, "No source-pack receipts"),
        sources,
    }
}

fn media_output_set(workspace_roots: &[PathBuf]) -> DxSourceSet {
    let mut sources = Vec::new();
    for root in workspace_roots {
        let receipt_root = root.join("tools").join("dx-media").join("executions");
        for receipt in latest_receipts(root, &receipt_root, 4) {
            sources.extend(media_sources_from_receipt(&receipt).into_iter().take(4));
        }
    }
    sources.truncate(4);

    DxSourceSet {
        label: "Media Outputs",
        status: source_set_status(workspace_roots, &sources, "No produced media outputs"),
        sources,
    }
}

fn forge_restore_preview_set(workspace_roots: &[PathBuf]) -> DxSourceSet {
    let mut sources = Vec::new();
    for root in workspace_roots {
        let receipt_root = root.join("tools").join("dx-forge").join("restores");
        for receipt in latest_receipts(root, &receipt_root, 4) {
            if let Some(source) = forge_restore_source_from_receipt(&receipt) {
                sources.push(source);
            }
        }
    }
    sources.truncate(4);

    DxSourceSet {
        label: "Restore Previews",
        status: source_set_status(workspace_roots, &sources, "No restore previews"),
        sources,
    }
}

fn metasearch_source_from_receipt(receipt: &ReceiptCandidate) -> Option<DxSourceItem> {
    let value = read_receipt_json(&receipt.path)?;
    let source_pack = value.get("source_pack").or_else(|| {
        value
            .get("search")
            .and_then(|search| search.get("source_pack"))
    })?;
    let query = string_at(source_pack, &["query"]).unwrap_or_else(|| "metasearch".to_string());
    let item_count = usize_at(source_pack, &["item_count"]).unwrap_or_default();
    let estimated_tokens = usize_at(source_pack, &["estimated_tokens"]).unwrap_or_default();

    Some(DxSourceItem {
        label: format!("Search: {query}"),
        detail: format!("{item_count} items - ~{estimated_tokens} tokens"),
        path: receipt.label.clone(),
        kind: DxSourceKind::MetasearchSourcePack,
        receipt_drilldowns: vec![receipt_drilldown("Source-pack receipt", receipt)],
        proofs: Vec::new(),
        warnings: Vec::new(),
    })
}

fn media_sources_from_receipt(receipt: &ReceiptCandidate) -> Vec<DxSourceItem> {
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
            let sha256 = string_at(file, &["sha256"]);
            let mut proofs = vec!["Output exists on disk".to_string()];
            if let Some(sha256) = sha256 {
                proofs.push(format!("sha256 {}", short_hash(&sha256)));
            }
            proofs.push(format!("Receipt {}", receipt.label));

            let mut warnings = Vec::new();
            if size_bytes == 0 {
                warnings.push("Produced file is empty".to_string());
            }

            Some(DxSourceItem {
                label,
                detail: format!("{media_kind} - {format} - {}", format_bytes(size_bytes)),
                path,
                kind: DxSourceKind::MediaOutput,
                receipt_drilldowns: vec![receipt_drilldown("Execution receipt", receipt)],
                proofs,
                warnings,
            })
        })
        .collect()
}

fn reduced_context_set(workspace_roots: &[PathBuf]) -> DxSourceSet {
    let mut sources = Vec::new();
    for root in workspace_roots {
        let receipt_root = root
            .join("tools")
            .join("dx-serializer-rlm")
            .join("reduced-context");
        for receipt in latest_receipts(root, &receipt_root, 4) {
            if let Some(source) = reduced_context_from_receipt(&receipt) {
                sources.push(source);
            }
        }
    }
    sources.truncate(4);

    DxSourceSet {
        label: "Reduced Context",
        status: source_set_status(workspace_roots, &sources, "No reduced-context receipts"),
        sources,
    }
}

fn reduced_context_from_receipt(receipt: &ReceiptCandidate) -> Option<DxSourceItem> {
    let value = read_receipt_json(&receipt.path)?;
    let reduced_context = value
        .get("reduced_context")
        .or_else(|| value.get("serializer_rlm_reduced_context"))?;
    let reduction = reduced_context.get("reduction").unwrap_or(&Value::Null);
    let gate = reduced_context.get("gate").unwrap_or(&Value::Null);
    let source_count = usize_at(reduction, &["source_count"]).unwrap_or_default();
    let tokens = usize_at(reduction, &["selected_estimated_tokens"]).unwrap_or_default();
    let status = string_at(reduction, &["status"]).unwrap_or_else(|| "reduced context".to_string());
    let reducer = string_at(gate, &["reducer"]).unwrap_or_else(|| "serializer/RLM".to_string());

    Some(DxSourceItem {
        label: format!("Reduced context: {reducer}"),
        detail: format!("{source_count} sources - ~{tokens} tokens - {status}"),
        path: receipt.label.clone(),
        kind: DxSourceKind::ReducedContextReceipt,
        receipt_drilldowns: vec![receipt_drilldown("Reduced-context receipt", receipt)],
        proofs: Vec::new(),
        warnings: Vec::new(),
    })
}

fn forge_restore_source_from_receipt(receipt: &ReceiptCandidate) -> Option<DxSourceItem> {
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
    let warnings = forge_restore_warnings(&value);
    let safety = if warnings.is_empty() {
        "preview only".to_string()
    } else {
        format!(
            "{} restore warning{}",
            warnings.len(),
            if warnings.len() == 1 { "" } else { "s" }
        )
    };

    Some(DxSourceItem {
        label: display_name(Path::new(&restore_root)),
        detail: format!(
            "{restored_file_count} restored files - {} - {safety}",
            format_bytes(restored_bytes),
        ),
        path: restore_root,
        kind: DxSourceKind::ForgeRestorePreview,
        receipt_drilldowns: vec![receipt_drilldown("Restore receipt", receipt)],
        proofs: Vec::new(),
        warnings,
    })
}

fn receipt_drilldown(label: &'static str, receipt: &ReceiptCandidate) -> DxSourceReceiptDrilldown {
    let size = receipt
        .path
        .metadata()
        .map(|metadata| format_bytes(metadata.len()))
        .unwrap_or_else(|_| "unknown size".to_string());

    DxSourceReceiptDrilldown {
        label: label.to_string(),
        detail: format!("{} - {size}", receipt.label),
    }
}
