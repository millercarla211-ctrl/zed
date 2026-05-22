use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::dx_deploy_local_files::{is_receipt_file, relative_label};

pub(crate) struct DeployReceiptCandidate {
    pub modified: SystemTime,
    pub label: String,
    pub path: PathBuf,
}

pub(crate) fn count_receipt_files(path: &Path) -> usize {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };

    entries
        .flatten()
        .take(128)
        .map(|entry| count_receipt_entry(&entry.path(), false))
        .sum()
}

pub(crate) fn count_direct_receipt_files(path: &Path) -> usize {
    let Ok(entries) = fs::read_dir(path) else {
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

pub(crate) fn latest_receipt_candidates(
    workspace_root: &Path,
    receipt_root: &Path,
    limit: usize,
) -> Vec<DeployReceiptCandidate> {
    let Ok(entries) = fs::read_dir(receipt_root) else {
        return Vec::new();
    };

    let mut receipts = Vec::new();
    for entry in entries.flatten().take(64) {
        let path = entry.path();
        if path.is_file() {
            push_receipt_candidate(workspace_root, &path, &mut receipts);
        } else if let Ok(children) = fs::read_dir(&path) {
            for child in children.flatten().take(64) {
                push_receipt_candidate(workspace_root, &child.path(), &mut receipts);
            }
        }
    }

    receipts.sort_by(newest_first);
    receipts.truncate(limit);
    receipts
}

pub(crate) fn latest_direct_receipt_candidates(
    workspace_root: &Path,
    receipt_root: &Path,
    limit: usize,
) -> Vec<DeployReceiptCandidate> {
    let Ok(entries) = fs::read_dir(receipt_root) else {
        return Vec::new();
    };

    let mut receipts = Vec::new();
    for entry in entries.flatten().take(64) {
        push_receipt_candidate(workspace_root, &entry.path(), &mut receipts);
    }

    receipts.sort_by(newest_first);
    receipts.truncate(limit);
    receipts
}

pub(crate) fn newest_first(
    left: &DeployReceiptCandidate,
    right: &DeployReceiptCandidate,
) -> Ordering {
    right
        .modified
        .partial_cmp(&left.modified)
        .unwrap_or(Ordering::Equal)
}

fn count_receipt_entry(path: &Path, nested: bool) -> usize {
    if path.is_file() && is_receipt_file(path) {
        return 1;
    }

    if nested || !path.is_dir() {
        return 0;
    }

    fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .take(64)
                .map(|entry| count_receipt_entry(&entry.path(), true))
                .sum()
        })
        .unwrap_or_default()
}

fn push_receipt_candidate(
    workspace_root: &Path,
    path: &Path,
    receipts: &mut Vec<DeployReceiptCandidate>,
) {
    if !path.is_file() || !is_receipt_file(path) {
        return;
    }

    let modified = path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    receipts.push(DeployReceiptCandidate {
        modified,
        label: relative_label(workspace_root, path),
        path: path.to_path_buf(),
    });
}
