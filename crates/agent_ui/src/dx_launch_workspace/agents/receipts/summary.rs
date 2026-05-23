use gpui::AnyElement;

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::super::super::metric_row;
use super::status::dx_agent_receipt_root_state;

pub(super) struct DxAgentReceiptCounts {
    pub(super) redacted_count: usize,
    pub(super) unsafe_count: usize,
}

pub(super) fn dx_agent_receipt_counts(snapshot: &DxAgentBridgeSnapshot) -> DxAgentReceiptCounts {
    DxAgentReceiptCounts {
        redacted_count: snapshot
            .receipts
            .iter()
            .filter(|receipt| receipt.metadata_redacted)
            .count(),
        unsafe_count: snapshot
            .receipts
            .iter()
            .filter(|receipt| !receipt.safe_to_render)
            .count(),
    }
}

pub(super) fn dx_agent_receipt_summary_rows(
    snapshot: &DxAgentBridgeSnapshot,
    counts: &DxAgentReceiptCounts,
) -> Vec<AnyElement> {
    let index = &snapshot.receipt_index;
    let inbox = &snapshot.receipt_inbox;

    vec![
        metric_row("Index", index.status.clone()),
        metric_row(
            "Root",
            dx_agent_receipt_root_state(index.receipt_root_present, snapshot.root_exists),
        ),
        metric_row("Inbox", inbox.status.clone()),
        metric_row(
            "Returned",
            format!("{} / {}", index.returned_receipt_count, index.receipt_count),
        ),
        metric_row("Active", index.active_task_count.to_string()),
        metric_row("Redacted", counts.redacted_count.to_string()),
        metric_row("Unsafe", counts.unsafe_count.to_string()),
    ]
}

pub(super) fn dx_agent_receipt_latest_row(
    snapshot: &DxAgentBridgeSnapshot,
    counts: &DxAgentReceiptCounts,
) -> Option<AnyElement> {
    let index = &snapshot.receipt_index;

    if !index.present || index.last_error.is_some() || counts.unsafe_count != 0 {
        return None;
    }

    index
        .latest_receipt_path
        .as_ref()
        .map(|path| metric_row("Latest", path.clone()))
}
