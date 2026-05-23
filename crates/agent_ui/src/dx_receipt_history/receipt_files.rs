use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub(super) type LatestReceipt = (SystemTime, PathBuf, String);

pub(super) fn root_label(relative_root: &Path, workspace_roots: &[PathBuf]) -> String {
    if workspace_roots.len() == 1 {
        return workspace_roots[0].join(relative_root).display().to_string();
    }

    format!("{} workspaces", workspace_roots.len())
}

pub(super) fn count_receipt_files(root: &Path) -> usize {
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };

    entries
        .flatten()
        .take(192)
        .map(|entry| {
            let path = entry.path();
            if path.is_file() {
                usize::from(is_receipt_file(&path))
            } else if path.is_dir() {
                count_nested_receipt_files(&path)
            } else {
                0
            }
        })
        .sum()
}

pub(super) fn push_latest_receipts(
    workspace_root: &Path,
    root: &Path,
    receipts: &mut Vec<LatestReceipt>,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten().take(64) {
        let path = entry.path();
        if path.is_file() {
            push_receipt_label(workspace_root, &path, receipts);
        } else if path.is_dir() {
            push_nested_receipt_labels(workspace_root, &path, receipts);
        }
    }
}

fn count_nested_receipt_files(path: &Path) -> usize {
    if path.file_name().and_then(|file_name| file_name.to_str()) == Some("preview") {
        return 0;
    }

    fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .take(64)
                .filter(|entry| entry.path().is_file() && is_receipt_file(&entry.path()))
                .count()
        })
        .unwrap_or_default()
}

fn push_nested_receipt_labels(
    workspace_root: &Path,
    path: &Path,
    receipts: &mut Vec<LatestReceipt>,
) {
    if path.file_name().and_then(|file_name| file_name.to_str()) == Some("preview") {
        return;
    }

    let Ok(children) = fs::read_dir(path) else {
        return;
    };
    for child in children.flatten().take(64) {
        let path = child.path();
        if path.is_file() {
            push_receipt_label(workspace_root, &path, receipts);
        }
    }
}

fn push_receipt_label(workspace_root: &Path, path: &Path, receipts: &mut Vec<LatestReceipt>) {
    if !is_receipt_file(path) {
        return;
    }

    let modified = path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let label = path
        .strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string();
    receipts.push((modified, path.to_path_buf(), label));
}

fn is_receipt_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "jsonl" | "receipt")
    )
}
