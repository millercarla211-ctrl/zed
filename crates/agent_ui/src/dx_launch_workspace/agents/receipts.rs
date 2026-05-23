use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::prelude::*;

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use self::footer::dx_agent_receipt_footer_rows;
use self::rows::dx_agent_receipt_row;
use self::status::dx_agent_receipt_warning_rows;
use self::summary::{
    dx_agent_receipt_counts, dx_agent_receipt_latest_row, dx_agent_receipt_summary_rows,
};
use super::super::muted_card;

mod footer;
mod labels;
mod row_details;
mod rows;
mod status;
mod summary;

pub(in super::super) fn dx_agent_receipt_state(
    snapshot: &DxAgentBridgeSnapshot,
    cx: &App,
) -> AnyElement {
    let index = &snapshot.receipt_index;
    let counts = dx_agent_receipt_counts(snapshot);
    let mut stack = v_flex()
        .gap_1()
        .children(dx_agent_receipt_summary_rows(snapshot, &counts));

    stack = stack.children(dx_agent_receipt_warning_rows(snapshot, counts.unsafe_count));

    if !index.present {
        stack = stack.child(muted_card("Run dx agents receipts list --json", cx));
    } else if let Some(latest_row) = dx_agent_receipt_latest_row(snapshot, &counts) {
        stack = stack.child(latest_row);
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
        .children(dx_agent_receipt_footer_rows(snapshot))
        .into_any_element()
}
