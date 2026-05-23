use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use self::rows::dx_agent_receipt_row;
use super::super::{metric_row, muted_card, signal_row};

mod rows;

pub(in super::super) fn dx_agent_receipt_state(
    snapshot: &DxAgentBridgeSnapshot,
    cx: &App,
) -> AnyElement {
    let index = &snapshot.receipt_index;
    let inbox = &snapshot.receipt_inbox;
    let redacted_count = snapshot
        .receipts
        .iter()
        .filter(|receipt| receipt.metadata_redacted)
        .count();
    let unsafe_count = snapshot
        .receipts
        .iter()
        .filter(|receipt| !receipt.safe_to_render)
        .count();
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Index", index.status.clone()))
        .child(metric_row(
            "Root",
            dx_agent_receipt_root_state(index.receipt_root_present, snapshot.root_exists),
        ))
        .child(metric_row("Inbox", inbox.status.clone()))
        .child(metric_row(
            "Returned",
            format!("{} / {}", index.returned_receipt_count, index.receipt_count),
        ))
        .child(metric_row("Active", index.active_task_count.to_string()))
        .child(metric_row("Redacted", redacted_count.to_string()))
        .child(metric_row("Unsafe", unsafe_count.to_string()));

    if index.receipt_root_present == Some(false) {
        stack = stack.child(signal_row(
            "dx-agent-receipt-root-missing".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents receipt root was missing before the latest receipt refresh.".to_string(),
        ));
    }
    if inbox.receipt_dir_present == Some(false) {
        stack = stack.child(signal_row(
            "dx-agent-receipt-inbox-root-missing".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents receipt inbox reports a missing receipt directory.".to_string(),
        ));
    } else if inbox.malformed_count > 0 {
        stack = stack.child(signal_row(
            "dx-agent-receipt-inbox-malformed".into(),
            IconName::Warning,
            Color::Warning,
            format!(
                "DX Agents receipt inbox found {} malformed receipt(s).",
                inbox.malformed_count
            ),
        ));
    }

    if !index.present {
        stack = stack.child(muted_card("Run dx agents receipts list --json", cx));
    } else if let Some(error) = index.last_error.as_ref() {
        stack = stack.child(signal_row(
            "dx-agent-receipt-index-error".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents receipt index error: {error}"),
        ));
    } else if unsafe_count > 0 {
        stack = stack.child(signal_row(
            "dx-agent-receipt-unsafe-row".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents receipt index contains rows that are not safe to render.".to_string(),
        ));
    } else if let Some(path) = index.latest_receipt_path.as_ref() {
        stack = stack.child(metric_row("Latest", path.clone()));
    }

    if snapshot.receipts.is_empty() {
        stack = stack.child(muted_card("No renderable receipt rows", cx));
    } else {
        for (ix, receipt) in snapshot.receipts.iter().take(3).enumerate() {
            stack = stack.child(dx_agent_receipt_row(
                SharedString::from(format!("dx-agent-receipt-{ix}")),
                receipt,
                cx,
            ));
        }
    }

    stack
        .when(inbox.present, |this| {
            this.child(metric_row(
                "Inbox review",
                format!(
                    "{} latest, {} missing, {} stale, {} expired",
                    inbox.latest_count,
                    inbox.missing_latest_count,
                    inbox.stale_count,
                    inbox.expired_count
                ),
            ))
        })
        .child(
            Label::new(index.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

fn dx_agent_receipt_root_state(receipt_root_present: Option<bool>, root_exists: bool) -> String {
    match receipt_root_present {
        Some(true) => "present".to_string(),
        Some(false) => "missing before refresh".to_string(),
        None if root_exists => "present".to_string(),
        None => "missing".to_string(),
    }
}
