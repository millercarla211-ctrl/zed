use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_launch_receipts::DxLaunchReceiptSummary;

use super::super::metric_row;

pub(super) fn launch_receipt_row(
    receipt: &DxLaunchReceiptSummary,
    label: &'static str,
    cx: &App,
) -> AnyElement {
    let detail = format!(
        "{} {} at {}",
        receipt.kind, receipt.file_name, receipt.receipt_path
    );
    let timing = receipt
        .age_ms
        .map(|age| format!("{age}ms old"))
        .unwrap_or_else(|| "unknown age".to_string());
    let next_action = receipt
        .next_action
        .as_deref()
        .unwrap_or("review_launch_receipt_metadata");

    v_flex()
        .id(SharedString::from(format!(
            "dx-launch-receipt-{}-{}",
            label, receipt.file_name
        )))
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(label, receipt.display_state()))
        .child(
            Label::new(detail)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(
            Label::new(format!("{timing}; next {next_action}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}
