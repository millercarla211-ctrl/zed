use dx_catalog::{DxReceiptCacheManifest, read_receipt_cache_artifact};
use std::path::Path;

pub(super) enum ReceiptCacheArtifactState {
    Missing,
    Ready(DxReceiptCacheManifest),
    Invalid(String),
}

pub(super) fn read_receipt_cache_artifact_state(path: &Path) -> ReceiptCacheArtifactState {
    if !path.is_file() {
        return ReceiptCacheArtifactState::Missing;
    }

    match read_receipt_cache_artifact(path) {
        Ok(manifest) => ReceiptCacheArtifactState::Ready(manifest),
        Err(error) => ReceiptCacheArtifactState::Invalid(error.to_string()),
    }
}
