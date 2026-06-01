use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};

use crate::dx_receipts::DxReceiptSnapshot;

use super::super::{metric_row, muted_card, source_row};

pub(in crate::dx_launch_workspace) fn receipt_source_state(
    snapshot: &DxReceiptSnapshot,
    cx: &mut App,
) -> AnyElement {
    if !snapshot.root_exists {
        return muted_card(
            format!("Receipts not found: {}", snapshot.root.display()),
            cx,
        );
    }

    let total = snapshot
        .buckets
        .iter()
        .map(|bucket| bucket.count)
        .sum::<usize>();
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Receipt files", total.to_string()));

    if snapshot.latest.is_empty() {
        stack = stack.child(muted_card("Waiting for first DX receipt", cx));
    } else {
        for (ix, label) in snapshot.latest.iter().enumerate() {
            stack = stack.child(source_row(
                SharedString::from(format!("latest-receipt-{ix}")),
                IconName::FileTextOutlined,
                label.clone(),
                cx,
            ));
        }
    }

    stack.into_any_element()
}
