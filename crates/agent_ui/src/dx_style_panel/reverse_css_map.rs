use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use serde_json::Value;

const DX_STYLE_REVERSE_CSS_MAP_SCHEMA: &str = "dx.style.grouped-class-reverse-css-map";
const MAX_REVERSE_CSS_MAP_RECEIPT_BYTES: u64 = 128 * 1024;

pub(super) struct ReverseCssMapSummary {
    pub(super) receipt_path: PathBuf,
    pub(super) status: String,
}

pub(super) fn reverse_css_map_summary(
    alias: &str,
    receipt_path: Option<&Path>,
) -> Option<ReverseCssMapSummary> {
    let receipt_path = receipt_path?;
    let text = read_text_limited(receipt_path)?;
    let value = serde_json::from_str::<Value>(&text).ok()?;
    if !trusted_reverse_css_map_receipt(&value) {
        return None;
    }
    let entry = value
        .get("entries")
        .and_then(Value::as_array)?
        .iter()
        .find(|entry| entry.get("alias").and_then(Value::as_str) == Some(alias))?;
    let reverse_status = entry
        .get("reverse_status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let utility_count = entry
        .get("utility_count")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    Some(ReverseCssMapSummary {
        receipt_path: receipt_path.to_path_buf(),
        status: format!("{reverse_status} ({utility_count} utilities)"),
    })
}

fn trusted_reverse_css_map_receipt(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(DX_STYLE_REVERSE_CSS_MAP_SCHEMA)
        && value
            .pointer("/trust/source_owned")
            .and_then(Value::as_bool)
            == Some(true)
        && value
            .pointer("/trust/source_mutation_enabled")
            .and_then(Value::as_bool)
            == Some(false)
        && value
            .pointer("/trust/editor_write_bridge_required")
            .and_then(Value::as_bool)
            == Some(true)
}

fn read_text_limited(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_REVERSE_CSS_MAP_RECEIPT_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_REVERSE_CSS_MAP_RECEIPT_BYTES {
        return None;
    }
    String::from_utf8(bytes).ok()
}
