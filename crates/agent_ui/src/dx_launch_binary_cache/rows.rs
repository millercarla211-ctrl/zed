use super::{
    DxBinaryCacheInput, DxBinaryCacheRow,
    artifacts::ReceiptCacheArtifactState,
    meters::{kind_summary_detail, metering_row_from_artifact},
    states::{cache_health_state, provider_catalog_state},
};
use dx_catalog::DxReceiptCacheEntryKind;
use std::path::Path;

pub(super) fn provider_catalog_row(input: &DxBinaryCacheInput) -> DxBinaryCacheRow {
    DxBinaryCacheRow {
        label: "Provider catalog".to_string(),
        state: provider_catalog_state(input.provider_catalog_present, input.provider_catalog_stale)
            .to_string(),
        path: input.provider_catalog_path.display().to_string(),
        detail: format!(
            "{} provider(s), {} model(s)",
            input.provider_count, input.model_count
        ),
        present: input.provider_catalog_present,
    }
}

pub(super) fn launch_receipts_row(
    input: &DxBinaryCacheInput,
    path: &Path,
    artifact: &ReceiptCacheArtifactState,
) -> DxBinaryCacheRow {
    receipt_cache_artifact_row(
        "Launch receipts",
        path,
        artifact,
        Some(DxReceiptCacheEntryKind::Launch),
    )
    .unwrap_or_else(|| {
        let launch_json_ready = input.launch_latest_present && input.launch_malformed_count == 0;
        artifact_row(
            "Launch receipts",
            path,
            launch_json_ready,
            input.launch_stale_count + input.launch_expired_count > 0,
            format!(
                "latest {}, {} snapshot(s), {} malformed",
                yes_no(input.launch_latest_present),
                input.launch_snapshot_count,
                input.launch_malformed_count
            ),
        )
    })
}

pub(super) fn receipt_index_row(
    input: &DxBinaryCacheInput,
    path: &Path,
    artifact: &ReceiptCacheArtifactState,
) -> DxBinaryCacheRow {
    receipt_cache_artifact_row("Receipt index", path, artifact, None).unwrap_or_else(|| {
        artifact_row(
            "Receipt index",
            path,
            input.receipt_root_exists && input.receipt_file_count > 0,
            false,
            format!(
                "{} receipt file(s) under {}",
                input.receipt_file_count,
                input.receipt_root.display()
            ),
        )
    })
}

pub(super) fn metering_row(
    input: &DxBinaryCacheInput,
    path: &Path,
    artifact: &ReceiptCacheArtifactState,
    receipt_cache_ready: bool,
) -> DxBinaryCacheRow {
    metering_row_from_artifact(path, artifact).unwrap_or_else(|| {
        let metering_source_ready =
            input.token_receipt_count + input.rlm_receipt_count + input.serializer_receipt_count
                > 0;
        DxBinaryCacheRow {
            label: "Token/tool meters".to_string(),
            state: if receipt_cache_ready {
                "ready".to_string()
            } else if metering_source_ready {
                "json-ready".to_string()
            } else {
                "waiting".to_string()
            },
            path: path.display().to_string(),
            detail: format!(
                "{} token / {} rlm / {} serializer receipt(s)",
                input.token_receipt_count, input.rlm_receipt_count, input.serializer_receipt_count
            ),
            present: receipt_cache_ready,
        }
    })
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

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
