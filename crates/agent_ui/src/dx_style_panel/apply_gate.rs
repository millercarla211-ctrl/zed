use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use super::editor_write_bridge::{
    StyleEditorWriteBridgeSnapshot, style_editor_write_bridge_snapshot,
};
pub(super) use super::receipt_match::StyleApplyGateInput;
use super::receipt_match::{
    StyleReceiptMismatchSummary, receipt_matches_active_source, receipt_mismatch_summary,
};
use super::receipt_review::{
    StyleDryRunReceiptSummary, TrustedDryRunReceipt, trusted_receipt as trusted_receipt_from_value,
};

const DX_STYLE_PROJECT_RECEIPT_ROOT: &str = r"G:\Dx\style\.dx\receipts\style";
const DX_STYLE_HUB_RECEIPT_ROOT: &str = r"G:\Dx\.dx\receipts\style";
const MAX_DRY_RUN_RECEIPT_BYTES: u64 = 128 * 1024;
const DRY_RUN_RECEIPT_SCAN_LIMIT: usize = 64;

#[derive(Clone)]
pub(super) struct StyleApplyGateSnapshot {
    pub(super) state: String,
    pub(super) reason: String,
    pub(super) trusted_dry_run_receipt_present: bool,
    pub(super) editor_write_bridge_ready: bool,
    pub(super) receipt_path: Option<String>,
    pub(super) receipt_summary: Option<StyleDryRunReceiptSummary>,
    pub(super) receipt_match: String,
    pub(super) receipt_mismatch: Option<StyleReceiptMismatchSummary>,
    pub(super) editor_write_bridge: StyleEditorWriteBridgeSnapshot,
    pub(super) can_enable_apply: bool,
}

impl StyleApplyGateSnapshot {
    pub(super) fn to_json(&self) -> Value {
        json!({
            "state": self.state,
            "reason": self.reason,
            "trusted_dry_run_receipt_present": self.trusted_dry_run_receipt_present,
            "editor_write_bridge_ready": self.editor_write_bridge_ready,
            "receipt_path": self.receipt_path,
            "receipt_summary": self.receipt_summary.as_ref().map(StyleDryRunReceiptSummary::to_json),
            "receipt_match": self.receipt_match,
            "receipt_mismatch": self.receipt_mismatch.as_ref().map(StyleReceiptMismatchSummary::to_json),
            "editor_write_bridge": self.editor_write_bridge.to_json(),
            "can_enable_apply": self.can_enable_apply,
        })
    }
}

pub(super) fn style_apply_gate(input: Option<StyleApplyGateInput<'_>>) -> StyleApplyGateSnapshot {
    let Some(input) = input else {
        return blocked(
            "needs_static_style_token",
            "Place the cursor on a static class/className token before reviewing source edits.",
            "no_active_token",
            None,
            None,
            None,
            false,
        );
    };
    if input.token.is_empty() {
        return blocked(
            "needs_static_style_token",
            "Place the cursor on a static class/className token before reviewing source edits.",
            "empty_active_token",
            None,
            None,
            None,
            false,
        );
    }
    if input.source_digest.is_none() {
        return blocked(
            "needs_active_source_digest",
            "The active source digest is unavailable, so no dry-run patch can be matched safely.",
            "active_digest_missing",
            None,
            None,
            None,
            false,
        );
    }

    let receipts = trusted_dry_run_receipts();
    if receipts.is_empty() {
        return blocked(
            "needs_trusted_dry_run_receipt",
            "No trusted DX Style grouped-class dry-run receipt was found.",
            "no_trusted_receipt",
            None,
            None,
            None,
            false,
        );
    }

    let mismatch_summary = receipt_mismatch_summary(&receipts, &input);
    let receipt = latest_matching_trusted_dry_run_receipt(&input, receipts);
    let Some(receipt) = receipt else {
        return blocked(
            "needs_matching_active_source_receipt",
            "Trusted DX Style receipts exist, but none match the active source path, token span, and digest.",
            "active_source_mismatch",
            Some(mismatch_summary),
            None,
            None,
            true,
        );
    };

    let editor_write_bridge = style_editor_write_bridge_snapshot();
    if !editor_write_bridge.can_apply {
        return blocked(
            "needs_editor_write_bridge",
            &editor_write_bridge.reason,
            "active_source_matched",
            None,
            Some(receipt.path),
            Some(receipt.summary),
            true,
        );
    }

    StyleApplyGateSnapshot {
        state: "ready_for_explicit_apply".to_string(),
        reason: "Trusted dry-run receipt and editor write bridge are ready.".to_string(),
        trusted_dry_run_receipt_present: true,
        editor_write_bridge_ready: editor_write_bridge.can_apply,
        receipt_path: Some(receipt.path.display().to_string()),
        receipt_summary: Some(receipt.summary),
        receipt_match: "active_source_matched".to_string(),
        receipt_mismatch: None,
        editor_write_bridge,
        can_enable_apply: true,
    }
}

fn blocked(
    state: &str,
    reason: &str,
    receipt_match: &str,
    receipt_mismatch: Option<StyleReceiptMismatchSummary>,
    receipt_path: Option<PathBuf>,
    receipt_summary: Option<StyleDryRunReceiptSummary>,
    trusted_receipt: bool,
) -> StyleApplyGateSnapshot {
    StyleApplyGateSnapshot {
        state: state.to_string(),
        reason: reason.to_string(),
        trusted_dry_run_receipt_present: trusted_receipt,
        editor_write_bridge_ready: false,
        receipt_path: receipt_path.map(|path| path.display().to_string()),
        receipt_summary,
        receipt_match: receipt_match.to_string(),
        receipt_mismatch,
        editor_write_bridge: style_editor_write_bridge_snapshot(),
        can_enable_apply: false,
    }
}

fn trusted_dry_run_receipts() -> Vec<TrustedDryRunReceipt> {
    [DX_STYLE_PROJECT_RECEIPT_ROOT, DX_STYLE_HUB_RECEIPT_ROOT]
        .into_iter()
        .flat_map(|root| trusted_receipts_in(Path::new(root)))
        .collect()
}

fn latest_matching_trusted_dry_run_receipt(
    input: &StyleApplyGateInput<'_>,
    receipts: Vec<TrustedDryRunReceipt>,
) -> Option<TrustedDryRunReceipt> {
    receipts
        .into_iter()
        .filter(|receipt| receipt_matches_active_source(receipt, input))
        .max_by_key(|receipt| {
            fs::metadata(&receipt.path)
                .and_then(|metadata| metadata.modified())
                .ok()
        })
}

fn trusted_receipts_in(root: &Path) -> Vec<TrustedDryRunReceipt> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && has_receipt_extension(path))
        .collect::<Vec<_>>();
    paths.sort_by(|left, right| receipt_modified(right).cmp(&receipt_modified(left)));

    paths
        .into_iter()
        .take(DRY_RUN_RECEIPT_SCAN_LIMIT)
        .filter_map(trusted_receipt_from_path)
        .collect()
}

fn has_receipt_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "jsonl" | "receipt")
    )
}

fn receipt_modified(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

fn trusted_receipt_from_path(path: PathBuf) -> Option<TrustedDryRunReceipt> {
    let Some(text) = read_text_limited(&path) else {
        return None;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return None;
    };

    trusted_receipt_from_value(path, &value)
}

fn read_text_limited(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_DRY_RUN_RECEIPT_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_DRY_RUN_RECEIPT_BYTES {
        return None;
    }

    String::from_utf8(bytes).ok()
}
