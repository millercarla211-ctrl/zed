use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::dx_deploy_root_key::deploy_root_key;

use super::{
    CHECK_RECEIPT_RELATIVE_PATH, DX_FALLBACK_CHECK_RECEIPT, DxCheckPanelSnapshot,
    MAX_RECEIPT_BYTES,
    parser::{malformed_snapshot, missing_snapshot, panel_from_receipt_value},
};
pub(super) fn read_latest_check_panel(workspace_roots: &[String]) -> DxCheckPanelSnapshot {
    let candidates = check_receipt_candidates(workspace_roots);
    for candidate in &candidates {
        if candidate.is_file() {
            return read_check_receipt(candidate);
        }
    }

    missing_snapshot(
        candidates
            .first()
            .cloned()
            .unwrap_or_else(|| PathBuf::from(DX_FALLBACK_CHECK_RECEIPT)),
    )
}

fn check_receipt_candidates(workspace_roots: &[String]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for root in workspace_roots {
        let mut path = PathBuf::from(root);
        for component in CHECK_RECEIPT_RELATIVE_PATH {
            path.push(*component);
        }
        push_unique_path(&mut candidates, path);
    }

    push_unique_path(&mut candidates, PathBuf::from(DX_FALLBACK_CHECK_RECEIPT));
    candidates
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    let path_key = deploy_root_key(&path);
    if !paths
        .iter()
        .any(|existing| deploy_root_key(existing) == path_key)
    {
        paths.push(path);
    }
}

fn read_check_receipt(path: &Path) -> DxCheckPanelSnapshot {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return malformed_snapshot(
                path.to_path_buf(),
                format!("dx-check receipt metadata could not be read: {error}"),
            );
        }
    };

    if metadata.len() > MAX_RECEIPT_BYTES {
        return malformed_snapshot(
            path.to_path_buf(),
            format!(
                "dx-check receipt is too large for the launch rail: {} bytes",
                metadata.len()
            ),
        );
    }

    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            return malformed_snapshot(
                path.to_path_buf(),
                format!("dx-check receipt could not be read: {error}"),
            );
        }
    };

    let mut receipt = Vec::new();
    if let Err(error) = file.take(MAX_RECEIPT_BYTES + 1).read_to_end(&mut receipt) {
        return malformed_snapshot(
            path.to_path_buf(),
            format!("dx-check receipt could not be read: {error}"),
        );
    }

    if receipt.len() as u64 > MAX_RECEIPT_BYTES {
        return malformed_snapshot(
            path.to_path_buf(),
            format!(
                "dx-check receipt is too large for the launch rail: {} bytes",
                receipt.len()
            ),
        );
    }

    let parsed = match serde_json::from_slice::<Value>(&receipt) {
        Ok(parsed) => parsed,
        Err(error) => {
            return malformed_snapshot(
                path.to_path_buf(),
                format!("dx-check receipt JSON is malformed: {error}"),
            );
        }
    };

    panel_from_receipt_value(path.to_path_buf(), &parsed)
}
