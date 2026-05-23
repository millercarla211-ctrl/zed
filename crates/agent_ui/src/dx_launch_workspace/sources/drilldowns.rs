use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_source_sets::{DxSourceItem, DxSourceReceiptDrilldown};

use super::super::signal_row;

pub(super) fn source_receipt_drilldown_rows(source: &DxSourceItem, cx: &App) -> Vec<AnyElement> {
    source
        .receipt_drilldowns
        .iter()
        .take(2)
        .enumerate()
        .map(|(ix, receipt)| {
            source_receipt_drilldown_row(
                SharedString::from(format!("source-receipt-{}-{ix}", source.path)),
                receipt,
                cx,
            )
        })
        .collect()
}

pub(super) fn source_receipt_drilldown_row(
    id: SharedString,
    receipt: &DxSourceReceiptDrilldown,
    cx: &App,
) -> AnyElement {
    let label_id = SharedString::from(format!("source-receipt-label-{}", receipt.detail));

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().editor_background)
        .child(signal_row(
            label_id,
            IconName::FileTextOutlined,
            Color::Muted,
            receipt.label.clone(),
        ))
        .child(
            Label::new(receipt.detail.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}
