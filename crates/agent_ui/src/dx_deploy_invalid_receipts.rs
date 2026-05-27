use serde_json::Value;
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

const MAX_DEPLOY_RECEIPT_BYTES: u64 = 256 * 1024;
const MAX_INVALID_DEPLOY_RECEIPTS: usize = 5;

#[derive(Clone)]
pub(crate) struct DxDeployInvalidReceipt {
    pub label: String,
    pub detail: String,
}

pub(crate) fn note_invalid_receipt(
    invalid_receipts: &mut Vec<DxDeployInvalidReceipt>,
    label: &str,
    detail: String,
) {
    if invalid_receipts
        .iter()
        .any(|receipt| receipt.label == label)
        || invalid_receipts.len() >= MAX_INVALID_DEPLOY_RECEIPTS
    {
        return;
    }

    invalid_receipts.push(DxDeployInvalidReceipt {
        label: label.to_string(),
        detail,
    });
}

pub(crate) fn read_deploy_receipt_json(path: &Path) -> Result<Value, String> {
    read_limited_receipt_json(path, MAX_DEPLOY_RECEIPT_BYTES, "dx-deploy")
}

pub(crate) fn read_limited_receipt_json(
    path: &Path,
    max_bytes: u64,
    receipt_label: &str,
) -> Result<Value, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Unable to stat {receipt_label} receipt: {error}"))?;
    if metadata.len() > max_bytes {
        return Err(format!(
            "{receipt_label} receipt is too large for the launch rail: {} bytes",
            metadata.len()
        ));
    }

    let mut file = File::open(path)
        .map_err(|error| format!("Unable to open {receipt_label} receipt: {error}"))?;
    let mut buffer = Vec::new();
    file.by_ref()
        .take(max_bytes)
        .read_to_end(&mut buffer)
        .map_err(|error| format!("Unable to read {receipt_label} receipt: {error}"))?;
    serde_json::from_slice(&buffer)
        .map_err(|error| format!("Unable to parse {receipt_label} receipt: {error}"))
}
