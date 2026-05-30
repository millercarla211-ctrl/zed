use serde_json::{Value, json};

use super::receipt_review::TrustedDryRunReceipt;

const RECEIPT_MISMATCH_REASON_LIMIT: usize = 3;

pub(super) struct StyleApplyGateInput<'a> {
    pub(super) token: &'a str,
    pub(super) source_path: &'a str,
    pub(super) workspace_root: Option<&'a str>,
    pub(super) span_start: usize,
    pub(super) span_end: usize,
    pub(super) source_digest: Option<&'a str>,
}

#[derive(Clone)]
pub(super) struct StyleReceiptMismatchSummary {
    pub(super) checked_receipt_count: usize,
    pub(super) reasons: Vec<String>,
    closest_candidate: Option<StyleReceiptCandidateSummary>,
}

#[derive(Clone)]
struct StyleReceiptCandidateSummary {
    receipt_path: String,
    match_score: u8,
    reason: String,
}

impl StyleReceiptMismatchSummary {
    pub(super) fn to_json(&self) -> Value {
        json!({
            "checked_receipt_count": self.checked_receipt_count,
            "reasons": self.reasons.clone(),
            "closest_candidate": self.closest_candidate.as_ref().map(StyleReceiptCandidateSummary::to_json),
        })
    }
}

impl StyleReceiptCandidateSummary {
    fn to_json(&self) -> Value {
        json!({
            "receipt_path": self.receipt_path,
            "match_score": self.match_score,
            "reason": self.reason,
        })
    }
}

pub(super) fn receipt_matches_active_source(
    receipt: &TrustedDryRunReceipt,
    input: &StyleApplyGateInput<'_>,
) -> bool {
    !input.token.is_empty()
        && receipt
            .source_paths
            .iter()
            .any(|path| path_matches(path, input.source_path))
        && receipt
            .edit_spans
            .iter()
            .any(|(start, end)| *start <= input.span_start && input.span_end <= *end)
        && receipt
            .source_digest
            .as_deref()
            .zip(input.source_digest)
            .is_some_and(|(receipt_digest, active_digest)| receipt_digest == active_digest)
}

pub(super) fn receipt_mismatch_summary(
    receipts: &[TrustedDryRunReceipt],
    input: &StyleApplyGateInput<'_>,
) -> StyleReceiptMismatchSummary {
    let mut reasons = Vec::new();
    for receipt in receipts.iter().take(RECEIPT_MISMATCH_REASON_LIMIT) {
        reasons.push(receipt_mismatch_reason(receipt, input));
    }

    StyleReceiptMismatchSummary {
        checked_receipt_count: receipts.len(),
        reasons,
        closest_candidate: closest_receipt_candidate(receipts, input),
    }
}

fn closest_receipt_candidate(
    receipts: &[TrustedDryRunReceipt],
    input: &StyleApplyGateInput<'_>,
) -> Option<StyleReceiptCandidateSummary> {
    receipts
        .iter()
        .max_by_key(|receipt| receipt_match_score(receipt, input))
        .map(|receipt| StyleReceiptCandidateSummary {
            receipt_path: receipt.path.display().to_string(),
            match_score: receipt_match_score(receipt, input),
            reason: receipt_mismatch_reason(receipt, input),
        })
}

fn receipt_match_score(receipt: &TrustedDryRunReceipt, input: &StyleApplyGateInput<'_>) -> u8 {
    let path_score = receipt
        .source_paths
        .iter()
        .any(|path| path_matches(path, input.source_path)) as u8;
    let span_score = receipt
        .edit_spans
        .iter()
        .any(|(start, end)| *start <= input.span_start && input.span_end <= *end)
        as u8;
    let digest_score = receipt
        .source_digest
        .as_deref()
        .zip(input.source_digest)
        .is_some_and(|(receipt_digest, active_digest)| receipt_digest == active_digest)
        as u8;
    path_score + span_score + digest_score
}

fn receipt_mismatch_reason(
    receipt: &TrustedDryRunReceipt,
    input: &StyleApplyGateInput<'_>,
) -> String {
    if input.token.is_empty() {
        return "active token is empty".to_string();
    }
    if !receipt
        .source_paths
        .iter()
        .any(|path| path_matches(path, input.source_path))
    {
        return format!("{}: source path mismatch", receipt.path.display());
    }
    if !receipt
        .edit_spans
        .iter()
        .any(|(start, end)| *start <= input.span_start && input.span_end <= *end)
    {
        return format!("{}: cursor token span mismatch", receipt.path.display());
    }
    if receipt
        .source_digest
        .as_deref()
        .zip(input.source_digest)
        .is_none()
    {
        return format!("{}: source digest missing", receipt.path.display());
    }
    if !receipt
        .source_digest
        .as_deref()
        .zip(input.source_digest)
        .is_some_and(|(receipt_digest, active_digest)| receipt_digest == active_digest)
    {
        return format!("{}: source digest mismatch", receipt.path.display());
    }
    format!("{}: match state unavailable", receipt.path.display())
}

fn path_matches(receipt_path: &str, active_path: &str) -> bool {
    let receipt_path = normalize_path(receipt_path);
    let active_path = normalize_path(active_path);
    if receipt_path == active_path {
        return true;
    }
    if receipt_path.contains(':') || receipt_path.starts_with('/') {
        return false;
    }
    active_path.ends_with(&format!("/{receipt_path}"))
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}
