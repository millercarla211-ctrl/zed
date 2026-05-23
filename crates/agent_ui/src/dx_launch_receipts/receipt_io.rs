use serde_json::Value;
use std::{fs::File, io::Read, path::Path};

const MAX_RECEIPT_BYTES: u64 = 128 * 1024;

pub(super) fn read_json_receipt(path: &Path) -> Result<Value, String> {
    let metadata = path
        .metadata()
        .map_err(|error| format!("Unable to inspect launch receipt metadata: {error}"))?;
    if metadata.len() > MAX_RECEIPT_BYTES {
        return Err(format!(
            "Launch receipt metadata is too large to render safely: {} bytes",
            metadata.len()
        ));
    }

    let mut contents = String::new();
    File::open(path)
        .and_then(|mut file| file.read_to_string(&mut contents))
        .map_err(|error| format!("Unable to read launch receipt metadata: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Unable to parse launch receipt metadata: {error}"))
}
