use serde_json::Value;
use std::{fs::File, io::Read, path::Path};

const MAX_RECEIPT_BYTES: u64 = 128 * 1024;

pub(super) fn read_json_receipt(path: &Path) -> Result<Value, String> {
    let metadata = path
        .metadata()
        .map_err(|error| format!("Unable to inspect launch status receipt: {error}"))?;
    if metadata.len() > MAX_RECEIPT_BYTES {
        return Err(format!(
            "Launch status receipt is too large to render safely: {} bytes",
            metadata.len()
        ));
    }

    let mut file = File::open(path)
        .map_err(|error| format!("Unable to read launch status receipt: {error}"))?;
    let mut buffer = Vec::new();
    file.by_ref()
        .take(MAX_RECEIPT_BYTES + 1)
        .read_to_end(&mut buffer)
        .map_err(|error| format!("Unable to read launch status receipt: {error}"))?;
    if buffer.len() as u64 > MAX_RECEIPT_BYTES {
        return Err(format!(
            "Launch status receipt grew beyond the safe render limit: {} bytes",
            buffer.len()
        ));
    }

    serde_json::from_slice(&buffer)
        .map_err(|error| format!("Unable to parse launch status receipt: {error}"))
}
