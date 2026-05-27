use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::prelude::*;

use crate::dx_agent_bridge::DxAgentReceipt;

use super::super::super::metric_row;
use super::labels::{receipt_display_id, receipt_state_label};
use super::row_details::dx_agent_receipt_detail_rows;

pub(super) fn dx_agent_receipt_row(
    id: SharedString,
    receipt: &DxAgentReceipt,
    cx: &App,
) -> AnyElement {
    let state = receipt_state_label(receipt);

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(receipt_display_id(receipt), state))
        .children(dx_agent_receipt_detail_rows(receipt))
        .into_any_element()
}
