use std::{
    cmp::Ordering,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::SystemTime,
};

use serde_json::Value;

use super::{MAX_RECEIPT_BYTES, local_file_labels::receipt_file_label};

pub(super) fn read_json(path: &Path) -> Option<Value> {
    let metadata = path.metadata().ok()?;
    if metadata.len() > MAX_RECEIPT_BYTES {
        return None;
    }
    let file = File::open(path).ok()?;
    let mut source = Vec::new();
    let mut limited = file.take(MAX_RECEIPT_BYTES + 1);
    limited.read_to_end(&mut source).ok()?;
    if u64::try_from(source.len()).unwrap_or(u64::MAX) > MAX_RECEIPT_BYTES {
        return None;
    }
    serde_json::from_slice(&source).ok()
}

pub(super) fn read_first_json(root: &Path, names: &[&str]) -> Option<Value> {
    names.iter().find_map(|name| read_json(&root.join(name)))
}

pub(super) fn latest_receipts(root: &Path, root_exists: bool) -> Vec<String> {
    if !root_exists {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut receipts = entries
        .flatten()
        .take(64)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let label = receipt_file_label(root, &path)?;
            let modified = path
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            Some((modified, label))
        })
        .collect::<Vec<_>>();
    receipts.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(Ordering::Equal));
    receipts
        .into_iter()
        .take(5)
        .map(|(_, label)| label)
        .collect()
}

pub(super) fn dx_home_from_receipt_root(receipt_root: &Path) -> Option<PathBuf> {
    receipt_root
        .ancestors()
        .find(|path| path.file_name().and_then(|name| name.to_str()) == Some(".dx"))
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}
