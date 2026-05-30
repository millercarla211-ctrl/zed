use std::path::PathBuf;

use serde_json::{Value, json};

use super::source_digest::{
    DX_STYLE_GROUPED_CLASS_SOURCE_DIGEST_ALGORITHM, DX_STYLE_GROUPED_CLASS_SOURCE_DIGEST_PREFIX,
};

const DRY_RUN_EDIT_SUMMARY_LIMIT: usize = 3;
const DRY_RUN_EDIT_PREVIEW_CHARS: usize = 120;
const MAX_DRY_RUN_REPLACEMENT_TEXT_BYTES: usize = 4096;
const GROUPED_CLASS_DRY_RUN_RECEIPT_SCHEMA: &str = "dx.style.grouped-class-dry-run-receipt";

pub(super) struct TrustedDryRunReceipt {
    pub(super) path: PathBuf,
    pub(super) summary: StyleDryRunReceiptSummary,
    pub(super) source_paths: Vec<String>,
    pub(super) source_digest: Option<String>,
    pub(super) edit_spans: Vec<(usize, usize)>,
}

#[derive(Clone)]
pub(super) struct StyleDryRunReceiptSummary {
    pub(super) intent: String,
    pub(super) status: String,
    pub(super) edit_count: usize,
    pub(super) message: String,
    pub(super) edits: Vec<String>,
    pub(super) edit_previews: Vec<StyleDryRunEditPreview>,
}

#[derive(Clone)]
pub(super) struct StyleDryRunEditPreview {
    source_path: String,
    start_byte: usize,
    end_byte: usize,
    replacement: String,
    replacement_text: String,
}

pub(super) fn trusted_receipt(path: PathBuf, value: &Value) -> Option<TrustedDryRunReceipt> {
    let trusted = value.get("schema").and_then(Value::as_str)
        == Some(GROUPED_CLASS_DRY_RUN_RECEIPT_SCHEMA)
        && value.get("status").and_then(Value::as_str) == Some("ready")
        && value
            .pointer("/trust/source_digest_verified")
            .and_then(Value::as_bool)
            == Some(true)
        && value
            .pointer("/trust/source_span_trusted")
            .and_then(Value::as_bool)
            == Some(true)
        && value
            .pointer("/trust/dry_run_preview_ready")
            .and_then(Value::as_bool)
            == Some(true);
    trusted.then(|| TrustedDryRunReceipt {
        summary: receipt_summary(value),
        source_paths: receipt_source_paths(value),
        source_digest: receipt_source_digest(value),
        edit_spans: receipt_edit_spans(value),
        path,
    })
}

fn receipt_source_digest(value: &Value) -> Option<String> {
    let algorithm = value
        .get("source_digest_algorithm")
        .and_then(Value::as_str)
        .unwrap_or(DX_STYLE_GROUPED_CLASS_SOURCE_DIGEST_ALGORITHM);
    let digest = value.get("source_digest").and_then(Value::as_str)?;
    (algorithm == DX_STYLE_GROUPED_CLASS_SOURCE_DIGEST_ALGORITHM
        && digest.starts_with(DX_STYLE_GROUPED_CLASS_SOURCE_DIGEST_PREFIX))
    .then(|| digest.to_string())
}

impl StyleDryRunReceiptSummary {
    pub(super) fn to_json(&self) -> Value {
        json!({
            "intent": self.intent,
            "status": self.status,
            "edit_count": self.edit_count,
            "message": self.message,
            "edits": self.edits.clone(),
            "edit_previews": self.edit_previews.iter().map(StyleDryRunEditPreview::to_json).collect::<Vec<_>>(),
        })
    }
}

impl StyleDryRunEditPreview {
    fn to_json(&self) -> Value {
        json!({
            "source_path": self.source_path,
            "start_byte": self.start_byte,
            "end_byte": self.end_byte,
            "replacement": self.replacement,
            "replacement_text": self.replacement_text,
        })
    }
}

pub(super) fn receipt_summary(value: &Value) -> StyleDryRunReceiptSummary {
    StyleDryRunReceiptSummary {
        intent: value
            .pointer("/patch_preview/intent")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        status: value
            .pointer("/patch_preview/status")
            .and_then(Value::as_str)
            .unwrap_or("ready")
            .to_string(),
        edit_count: value
            .pointer("/patch_preview/edits")
            .and_then(Value::as_array)
            .map_or(0, Vec::len),
        message: value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("trusted dry-run receipt is ready for review")
            .to_string(),
        edits: receipt_edit_summaries(value),
        edit_previews: receipt_edit_previews(value),
    }
}

fn receipt_edit_summaries(value: &Value) -> Vec<String> {
    value
        .pointer("/patch_preview/edits")
        .and_then(Value::as_array)
        .map(|edits| {
            edits
                .iter()
                .take(DRY_RUN_EDIT_SUMMARY_LIMIT)
                .map(receipt_edit_summary)
                .collect()
        })
        .unwrap_or_default()
}

fn receipt_edit_summary(edit: &Value) -> String {
    let source_file = edit
        .pointer("/source_file/path")
        .and_then(Value::as_str)
        .unwrap_or("unknown source");
    let start = source_position_label(edit.pointer("/span/start"));
    let end = source_position_label(edit.pointer("/span/end"));
    let replacement = edit
        .get("replacement_text")
        .and_then(Value::as_str)
        .map(compact_replacement_preview)
        .unwrap_or_else(|| "missing replacement".to_string());
    format!("{source_file}:{start}-{end} -> {replacement}")
}

fn receipt_edit_previews(value: &Value) -> Vec<StyleDryRunEditPreview> {
    value
        .pointer("/patch_preview/edits")
        .and_then(Value::as_array)
        .map(|edits| {
            edits
                .iter()
                .take(DRY_RUN_EDIT_SUMMARY_LIMIT)
                .filter_map(receipt_edit_preview)
                .collect()
        })
        .unwrap_or_default()
}

fn receipt_edit_preview(edit: &Value) -> Option<StyleDryRunEditPreview> {
    let source_path = edit.pointer("/source_file/path").and_then(Value::as_str)?;
    let start_byte = edit
        .pointer("/span/start/byte_offset")
        .and_then(Value::as_u64)? as usize;
    let end_byte = edit
        .pointer("/span/end/byte_offset")
        .and_then(Value::as_u64)? as usize;
    (start_byte <= end_byte).then_some(())?;
    let replacement_text = edit.get("replacement_text").and_then(Value::as_str)?;
    if replacement_text.is_empty() || replacement_text.len() > MAX_DRY_RUN_REPLACEMENT_TEXT_BYTES {
        return None;
    }
    let replacement = compact_replacement_preview(replacement_text);

    Some(StyleDryRunEditPreview {
        source_path: source_path.to_string(),
        start_byte,
        end_byte,
        replacement,
        replacement_text: replacement_text.to_string(),
    })
}

fn receipt_source_paths(value: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(path) = value.pointer("/source_file/path").and_then(Value::as_str) {
        paths.push(path.to_string());
    }
    if let Some(edits) = value
        .pointer("/patch_preview/edits")
        .and_then(Value::as_array)
    {
        for edit in edits {
            if let Some(path) = edit.pointer("/source_file/path").and_then(Value::as_str) {
                paths.push(path.to_string());
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn receipt_edit_spans(value: &Value) -> Vec<(usize, usize)> {
    value
        .pointer("/patch_preview/edits")
        .and_then(Value::as_array)
        .map(|edits| {
            edits
                .iter()
                .filter_map(|edit| {
                    let start = edit
                        .pointer("/span/start/byte_offset")
                        .and_then(Value::as_u64)? as usize;
                    let end = edit
                        .pointer("/span/end/byte_offset")
                        .and_then(Value::as_u64)? as usize;
                    (start <= end).then_some((start, end))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn source_position_label(position: Option<&Value>) -> String {
    let line = position
        .and_then(|position| position.get("line"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let column = position
        .and_then(|position| position.get("column"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    format!("{line}:{column}")
}

fn compact_replacement_preview(value: &str) -> String {
    let trimmed = value.trim();
    let mut preview = trimmed
        .chars()
        .take(DRY_RUN_EDIT_PREVIEW_CHARS)
        .collect::<String>();
    if trimmed.chars().count() > DRY_RUN_EDIT_PREVIEW_CHARS {
        preview.push_str("...");
    }
    preview
}
