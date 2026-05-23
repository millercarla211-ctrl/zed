mod artifacts;
mod meters;
mod paths;
mod rows;
mod states;
mod summary;

use self::artifacts::{ReceiptCacheArtifactState, read_receipt_cache_artifact_state};
use self::paths::{launch_receipt_cache_path, receipt_cache_artifact_path};
use self::rows::{launch_receipts_row, metering_row, provider_catalog_row, receipt_index_row};
use self::summary::{binary_cache_next_action, binary_cache_operator_summary, binary_cache_status};
use std::path::PathBuf;

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
    let launch_cache_path = launch_receipt_cache_path(&input.launch_receipt_root);
    let receipt_cache_path = receipt_cache_artifact_path();
    let launch_artifact = read_receipt_cache_artifact_state(&launch_cache_path);
    let receipt_artifact = read_receipt_cache_artifact_state(&receipt_cache_path);
    let receipt_cache_ready = matches!(&receipt_artifact, ReceiptCacheArtifactState::Ready(_));
    let rows = vec![
        provider_catalog_row(&input),
        launch_receipts_row(&input, &launch_cache_path, &launch_artifact),
        receipt_index_row(&input, &receipt_cache_path, &receipt_artifact),
        metering_row(
            &input,
            &receipt_cache_path,
            &receipt_artifact,
            receipt_cache_ready,
        ),
    ];
    let status = binary_cache_status(&rows);

    DxBinaryCacheSnapshot {
        status: status.to_string(),
        operator_summary: binary_cache_operator_summary(status),
        next_action: binary_cache_next_action(&input, &rows),
        rows,
    }
}
