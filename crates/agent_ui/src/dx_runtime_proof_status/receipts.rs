use serde_json::Value;
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::SystemTime,
};

const MAX_RECEIPT_BYTES: u64 = 128 * 1024;

pub(super) fn count_receipt_files(root: &Path) -> usize {
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };

    entries
        .flatten()
        .take(128)
        .filter(|entry| {
            let path = entry.path();
            path.is_file() && is_receipt_file(&path)
        })
        .count()
}

pub(super) fn latest_receipt_paths(
    workspace_root: &Path,
    receipt_root: &Path,
) -> Vec<(SystemTime, PathBuf, String)> {
    let Ok(entries) = fs::read_dir(receipt_root) else {
        return Vec::new();
    };

    entries
        .flatten()
        .take(128)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() || !is_receipt_file(&path) {
                return None;
            }
            let modified = path
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let label = path
                .strip_prefix(workspace_root)
                .unwrap_or(&path)
                .display()
                .to_string();
            Some((modified, path, label))
        })
        .collect()
}

pub(super) fn read_json(path: &Path) -> Option<Value> {
    let mut file = File::open(path).ok()?;
    let mut buffer = Vec::new();
    file.by_ref()
        .take(MAX_RECEIPT_BYTES + 1)
        .read_to_end(&mut buffer)
        .ok()?;
    if buffer.len() > MAX_RECEIPT_BYTES as usize {
        return None;
    }
    serde_json::from_slice(&buffer).ok()
}

fn is_receipt_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "receipt")
    )
}
