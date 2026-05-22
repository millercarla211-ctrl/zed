use serde_json::Value;
use std::{
    fs::File,
    io::{Read, Result},
    path::Path,
};

const MAX_JSON_BYTES: u64 = 64 * 1024;

pub(crate) fn read_json_limited(path: &Path) -> Option<Value> {
    let mut file = File::open(path).ok()?;
    let mut buffer = Vec::new();
    read_limited(&mut file, &mut buffer).ok()?;
    serde_json::from_slice(&buffer).ok()
}

pub(crate) fn relative_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub(crate) fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

pub(crate) fn is_receipt_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "jsonl" | "receipt")
    )
}

fn read_limited(file: &mut File, buffer: &mut Vec<u8>) -> Result<usize> {
    file.by_ref().take(MAX_JSON_BYTES).read_to_end(buffer)
}
