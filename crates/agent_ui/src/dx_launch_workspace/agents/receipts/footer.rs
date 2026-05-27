use gpui::{AnyElement, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::super::super::metric_row;
use super::labels::receipt_next_action_label;

pub(super) fn dx_agent_receipt_footer_rows(snapshot: &DxAgentBridgeSnapshot) -> Vec<AnyElement> {
    let index = &snapshot.receipt_index;
    let inbox = &snapshot.receipt_inbox;
    let mut rows = Vec::new();

    if inbox.present {
        rows.push(metric_row(
            "Inbox review",
            format!(
                "{} latest, {} missing, {} stale, {} expired",
                inbox.latest_count,
                inbox.missing_latest_count,
                inbox.stale_count,
                inbox.expired_count
            ),
        ));
    }

    rows.push(
        Label::new(receipt_next_action_label(&index.next_action))
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate()
            .into_any_element(),
    );
    rows
}
