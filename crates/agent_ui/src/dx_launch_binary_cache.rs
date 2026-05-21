use dx_catalog::{
    DxReceiptCacheEntryKind, DxReceiptCacheHealth, DxReceiptCacheKindSummary,
    DxReceiptCacheManifest, read_receipt_cache_artifact,
};
use std::{
    env,
    path::{Path, PathBuf},
};

const DX_RECEIPT_CACHE_ARTIFACT_ENV: &str = "DX_RECEIPT_CACHE_ARTIFACT";
const DX_LAUNCH_RECEIPT_CACHE_ARTIFACT_ENV: &str = "DX_LAUNCH_RECEIPT_CACHE_ARTIFACT";
const DEFAULT_RECEIPT_CACHE_ARTIFACT: &str = r"G:\Dx\.dx\receipts\receipt-cache.dxrc";
const DEFAULT_LAUNCH_RECEIPT_CACHE_FILE: &str = "launch-receipts.dxrc";

#[derive(Clone)]
pub(crate) struct DxBinaryCacheInput {
    pub provider_catalog_path: PathBuf,
    pub provider_catalog_present: bool,
    pub provider_catalog_stale: bool,
    pub provider_count: usize,
    pub model_count: usize,
    pub launch_receipt_root: PathBuf,
    pub launch_latest_present: bool,
    pub launch_snapshot_count: usize,
    pub launch_malformed_count: usize,
    pub launch_stale_count: usize,
    pub launch_expired_count: usize,
    pub receipt_root: PathBuf,
    pub receipt_root_exists: bool,
    pub receipt_file_count: usize,
    pub token_receipt_count: usize,
    pub rlm_receipt_count: usize,
    pub serializer_receipt_count: usize,
}

#[derive(Clone)]
pub(crate) struct DxBinaryCacheSnapshot {
    pub status: String,
    pub operator_summary: String,
    pub rows: Vec<DxBinaryCacheRow>,
    pub next_action: String,
}

#[derive(Clone)]
pub(crate) struct DxBinaryCacheRow {
    pub label: String,
    pub state: String,
    pub path: String,
    pub detail: String,
    pub present: bool,
}

pub(crate) fn binary_cache_snapshot(input: DxBinaryCacheInput) -> DxBinaryCacheSnapshot {
    let launch_cache_path = env_path(DX_LAUNCH_RECEIPT_CACHE_ARTIFACT_ENV).unwrap_or_else(|| {
        input
            .launch_receipt_root
            .join(DEFAULT_LAUNCH_RECEIPT_CACHE_FILE)
    });
    let receipt_cache_path = env_path(DX_RECEIPT_CACHE_ARTIFACT_ENV)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT_CACHE_ARTIFACT));
    let launch_artifact = read_receipt_cache_artifact_state(&launch_cache_path);
    let receipt_artifact = read_receipt_cache_artifact_state(&receipt_cache_path);

    let provider_row = DxBinaryCacheRow {
        label: "Provider catalog".to_string(),
        state: provider_catalog_state(input.provider_catalog_present, input.provider_catalog_stale)
            .to_string(),
        path: input.provider_catalog_path.display().to_string(),
        detail: format!(
            "{} provider(s), {} model(s)",
            input.provider_count, input.model_count
        ),
        present: input.provider_catalog_present,
    };

    let launch_json_ready = input.launch_latest_present && input.launch_malformed_count == 0;
    let launch_row = receipt_cache_artifact_row(
        "Launch receipts",
        &launch_cache_path,
        &launch_artifact,
        Some(DxReceiptCacheEntryKind::Launch),
    )
    .unwrap_or_else(|| {
        artifact_row(
            "Launch receipts",
            &launch_cache_path,
            launch_json_ready,
            input.launch_stale_count + input.launch_expired_count > 0,
            format!(
                "latest {}, {} snapshot(s), {} malformed",
                yes_no(input.launch_latest_present),
                input.launch_snapshot_count,
                input.launch_malformed_count
            ),
        )
    });

    let receipt_json_ready = input.receipt_root_exists && input.receipt_file_count > 0;
    let receipt_row = receipt_cache_artifact_row(
        "Receipt index",
        &receipt_cache_path,
        &receipt_artifact,
        None,
    )
    .unwrap_or_else(|| {
        artifact_row(
            "Receipt index",
            &receipt_cache_path,
            receipt_json_ready,
            false,
            format!(
                "{} receipt file(s) under {}",
                input.receipt_file_count,
                input.receipt_root.display()
            ),
        )
    });

    let metering_source_ready =
        input.token_receipt_count + input.rlm_receipt_count + input.serializer_receipt_count > 0;
    let receipt_cache_ready = matches!(&receipt_artifact, ReceiptCacheArtifactState::Ready(_));
    let metering_row = metering_row_from_artifact(&receipt_cache_path, &receipt_artifact)
        .unwrap_or_else(|| DxBinaryCacheRow {
            label: "Token/tool meters".to_string(),
            state: if receipt_cache_ready {
                "ready".to_string()
            } else if metering_source_ready {
                "json-ready".to_string()
            } else {
                "waiting".to_string()
            },
            path: receipt_cache_path.display().to_string(),
            detail: format!(
                "{} token / {} rlm / {} serializer receipt(s)",
                input.token_receipt_count, input.rlm_receipt_count, input.serializer_receipt_count
            ),
            present: receipt_cache_ready,
        });

    let rows = vec![provider_row, launch_row, receipt_row, metering_row];
    let binary_ready_count = rows.iter().filter(|row| row.state == "ready").count();
    let binary_backed_count = rows
        .iter()
        .filter(|row| binary_cache_state_from_artifact(&row.state))
        .count();
    let binary_attention_count = rows
        .iter()
        .filter(|row| binary_cache_state_needs_attention(&row.state))
        .count();
    let json_ready_count = rows
        .iter()
        .filter(|row| row.state == "json-ready" || row.state == "stale")
        .count();

    let status = if binary_ready_count == rows.len() {
        "ready"
    } else if binary_attention_count > 0 {
        "artifact-review"
    } else if binary_backed_count > 0 || json_ready_count > 0 {
        "json-authoritative"
    } else {
        "waiting"
    };
    let operator_summary = match status {
        "ready" => "Provider/catalog and receipt metadata are binary-backed.".to_string(),
        "artifact-review" => {
            "Receipt-cache artifacts need review; JSON receipt readers remain authoritative."
                .to_string()
        }
        "json-authoritative" => {
            "JSON receipt readers remain authoritative while missing binary artifacts are reported."
                .to_string()
        }
        _ => "Waiting for DX provider catalog or receipt metadata before binary cache handoff."
            .to_string(),
    };
    let next_action = if !input.receipt_root_exists {
        format!("Create DX receipt root at {}", input.receipt_root.display())
    } else if !input.launch_latest_present {
        "dx launch status --json".to_string()
    } else if binary_attention_count > 0 {
        "Regenerate metadata-only receipt cache artifacts".to_string()
    } else if binary_ready_count < rows.len() {
        "Materialize metadata-only receipt cache artifacts".to_string()
    } else {
        "Keep binary cache contracts stable".to_string()
    };

    DxBinaryCacheSnapshot {
        status: status.to_string(),
        operator_summary,
        rows,
        next_action,
    }
}

enum ReceiptCacheArtifactState {
    Missing,
    Ready(DxReceiptCacheManifest),
    Invalid(String),
}

fn read_receipt_cache_artifact_state(path: &Path) -> ReceiptCacheArtifactState {
    if !path.is_file() {
        return ReceiptCacheArtifactState::Missing;
    }

    match read_receipt_cache_artifact(path) {
        Ok(manifest) => ReceiptCacheArtifactState::Ready(manifest),
        Err(error) => ReceiptCacheArtifactState::Invalid(error.to_string()),
    }
}

fn receipt_cache_artifact_row(
    label: &str,
    path: &Path,
    artifact: &ReceiptCacheArtifactState,
    kind: Option<DxReceiptCacheEntryKind>,
) -> Option<DxBinaryCacheRow> {
    match artifact {
        ReceiptCacheArtifactState::Missing => None,
        ReceiptCacheArtifactState::Invalid(error) => Some(DxBinaryCacheRow {
            label: label.to_string(),
            state: "malformed".to_string(),
            path: path.display().to_string(),
            detail: error.clone(),
            present: true,
        }),
        ReceiptCacheArtifactState::Ready(manifest) => {
            let (state, detail) = if let Some(kind) = kind {
                let summary = manifest.kind_summary(kind);
                (
                    cache_health_state(summary.health()).to_string(),
                    kind_summary_detail(&summary),
                )
            } else {
                let summary = manifest.summary();
                (
                    cache_health_state(summary.health()).to_string(),
                    format!(
                        "{} entry(s), {} / {} root(s), {} malformed",
                        summary.entry_count,
                        summary.present_root_count,
                        summary.root_count,
                        summary.malformed_entry_count
                    ),
                )
            };

            Some(DxBinaryCacheRow {
                label: label.to_string(),
                state,
                path: path.display().to_string(),
                detail,
                present: true,
            })
        }
    }
}

fn metering_row_from_artifact(
    path: &Path,
    artifact: &ReceiptCacheArtifactState,
) -> Option<DxBinaryCacheRow> {
    let ReceiptCacheArtifactState::Ready(manifest) = artifact else {
        return None;
    };

    let token = manifest.kind_summary(DxReceiptCacheEntryKind::Tokens);
    let rlm = manifest.kind_summary(DxReceiptCacheEntryKind::Rlm);
    let serializer = manifest.kind_summary(DxReceiptCacheEntryKind::Serializer);
    let state = combined_meter_health(&[&token, &rlm, &serializer]);

    Some(DxBinaryCacheRow {
        label: "Token/tool meters".to_string(),
        state: cache_health_state(state).to_string(),
        path: path.display().to_string(),
        detail: format!(
            "tokens: {}; rlm: {}; serializer: {}",
            kind_summary_detail(&token),
            kind_summary_detail(&rlm),
            kind_summary_detail(&serializer)
        ),
        present: true,
    })
}

fn combined_meter_health(summaries: &[&DxReceiptCacheKindSummary]) -> DxReceiptCacheHealth {
    if summaries
        .iter()
        .any(|summary| summary.health() == DxReceiptCacheHealth::Malformed)
    {
        return DxReceiptCacheHealth::Malformed;
    }
    if summaries.iter().all(|summary| summary.entry_count == 0) {
        return DxReceiptCacheHealth::Empty;
    }
    if summaries
        .iter()
        .any(|summary| summary.health() == DxReceiptCacheHealth::Expired)
    {
        return DxReceiptCacheHealth::Expired;
    }
    if summaries
        .iter()
        .any(|summary| summary.health() == DxReceiptCacheHealth::Stale)
    {
        return DxReceiptCacheHealth::Stale;
    }
    DxReceiptCacheHealth::Ready
}

fn kind_summary_detail(summary: &DxReceiptCacheKindSummary) -> String {
    format!(
        "{} entry(s), {} fresh, {} stale, {} malformed",
        summary.entry_count,
        summary.fresh_entry_count,
        summary.stale_entry_count,
        summary.malformed_entry_count
    )
}

fn cache_health_state(health: DxReceiptCacheHealth) -> &'static str {
    match health {
        DxReceiptCacheHealth::Ready => "ready",
        DxReceiptCacheHealth::Partial => "partial",
        DxReceiptCacheHealth::Stale => "stale",
        DxReceiptCacheHealth::Expired => "expired",
        DxReceiptCacheHealth::Malformed => "malformed",
        DxReceiptCacheHealth::MissingRoots => "missing-roots",
        DxReceiptCacheHealth::Empty => "empty",
        DxReceiptCacheHealth::Unknown => "unknown",
    }
}

fn binary_cache_state_from_artifact(state: &str) -> bool {
    matches!(
        state,
        "ready" | "partial" | "stale" | "expired" | "malformed" | "missing-roots" | "empty"
    )
}

fn binary_cache_state_needs_attention(state: &str) -> bool {
    matches!(
        state,
        "partial" | "expired" | "malformed" | "missing-roots" | "unknown"
    )
}

fn artifact_row(
    label: &str,
    path: &Path,
    json_ready: bool,
    source_stale: bool,
    detail: String,
) -> DxBinaryCacheRow {
    let present = path.is_file();
    let state = if present {
        "ready"
    } else if json_ready && source_stale {
        "stale"
    } else if json_ready {
        "json-ready"
    } else {
        "waiting"
    };

    DxBinaryCacheRow {
        label: label.to_string(),
        state: state.to_string(),
        path: path.display().to_string(),
        detail,
        present,
    }
}

fn provider_catalog_state(present: bool, stale: bool) -> &'static str {
    if present && stale {
        "stale"
    } else if present {
        "ready"
    } else {
        "waiting"
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
