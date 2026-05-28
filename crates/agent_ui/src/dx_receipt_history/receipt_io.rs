use serde_json::Value;
use std::{fs::File, io::Read, path::Path};

const MAX_RECEIPT_BYTES: u64 = 128 * 1024;

pub(super) fn read_json(path: &Path) -> Option<Value> {
    let file = File::open(path).ok()?;
    let mut reader = file.take(MAX_RECEIPT_BYTES + 1);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).ok()?;
    if (buffer.len() as u64) > MAX_RECEIPT_BYTES {
        return None;
    }
    serde_json::from_slice(&buffer).ok()
}
