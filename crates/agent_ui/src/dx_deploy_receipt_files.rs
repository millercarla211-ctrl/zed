use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::dx_deploy_local_files::{is_receipt_file, relative_label};

const DEPLOY_RECEIPT_ROOT_ENTRY_LIMIT: usize = 128;
const DEPLOY_RECEIPT_NESTED_ENTRY_LIMIT: usize = 64;
const DEPLOY_RECEIPT_LATEST_ROOT_ENTRY_LIMIT: usize = 64;
const DEPLOY_RECEIPT_LATEST_NESTED_ENTRY_LIMIT: usize = 64;
const DEPLOY_RECEIPT_LATEST_CANDIDATE_LIMIT: usize = 16;

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
        .take(DEPLOY_RECEIPT_ROOT_ENTRY_LIMIT)
        .map(|entry| count_receipt_entry(&entry.path(), false))
        .sum()
}

pub(crate) fn count_direct_receipt_files(path: &Path) -> usize {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };

    entries
        .flatten()
        .take(DEPLOY_RECEIPT_ROOT_ENTRY_LIMIT)
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

    let candidate_limit = limit.min(DEPLOY_RECEIPT_LATEST_CANDIDATE_LIMIT);
    let mut receipts = Vec::new();
    for entry in entries
        .flatten()
        .take(DEPLOY_RECEIPT_LATEST_ROOT_ENTRY_LIMIT)
    {
        let path = entry.path();
        if path.is_file() {
            push_bounded_receipt_candidate(workspace_root, &path, &mut receipts, candidate_limit);
        } else if let Ok(children) = fs::read_dir(&path) {
            for child in children
                .flatten()
                .take(DEPLOY_RECEIPT_LATEST_NESTED_ENTRY_LIMIT)
            {
                push_bounded_receipt_candidate(
                    workspace_root,
                    &child.path(),
                    &mut receipts,
                    candidate_limit,
                );
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

    let candidate_limit = limit.min(DEPLOY_RECEIPT_LATEST_CANDIDATE_LIMIT);
    let mut receipts = Vec::new();
    for entry in entries
        .flatten()
        .take(DEPLOY_RECEIPT_LATEST_ROOT_ENTRY_LIMIT)
    {
        push_bounded_receipt_candidate(
            workspace_root,
            &entry.path(),
            &mut receipts,
            candidate_limit,
        );
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
                .take(DEPLOY_RECEIPT_NESTED_ENTRY_LIMIT)
                .map(|entry| count_receipt_entry(&entry.path(), true))
                .sum()
        })
        .unwrap_or_default()
}

fn push_bounded_receipt_candidate(
    workspace_root: &Path,
    path: &Path,
    receipts: &mut Vec<DeployReceiptCandidate>,
    candidate_limit: usize,
) {
    if candidate_limit == 0 || !path.is_file() || !is_receipt_file(path) {
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
    if receipts.len() > candidate_limit {
        receipts.sort_by(newest_first);
        receipts.truncate(candidate_limit);
    }
}
