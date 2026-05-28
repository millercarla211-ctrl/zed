use serde_json::Value;
use std::{
    cmp::Ordering,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::SystemTime,
};

const MAX_RECEIPT_BYTES: u64 = 1024 * 1024;

#[derive(Clone)]
pub(super) struct ReceiptCandidate {
    pub(super) path: PathBuf,
    pub(super) label: String,
    modified: SystemTime,
}

pub(super) fn latest_receipts(
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

pub(super) fn read_receipt_json(path: &Path) -> Option<Value> {
    let mut file = File::open(path).ok()?;
    let mut buffer = Vec::new();
    file.by_ref()
        .take(MAX_RECEIPT_BYTES + 1)
        .read_to_end(&mut buffer)
        .ok()?;
    if buffer.len() as u64 > MAX_RECEIPT_BYTES {
        return None;
    }
    serde_json::from_slice(&buffer).ok()
}

fn is_receipt_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "jsonl" | "receipt")
    )
}
