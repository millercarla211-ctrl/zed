use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_receipt_history::DxToolHistoryReceiptSummary;

use super::super::{metric_row, signal_row};

pub(super) fn tool_history_summary_row(
    id: SharedString,
    row_id: String,
    summary: &DxToolHistoryReceiptSummary,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(summary.headline.clone(), summary.detail.clone()));

    if let Some(target_path) = summary.target_path.as_ref() {
        stack = stack.child(
            Label::new(format!("Target {target_path}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if let Some(preview_path) = summary.restore_destination_root.as_ref() {
        stack = stack.child(
            Label::new(format!("Preview {preview_path}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if summary.blocker_count > 0 {
        stack = stack.child(signal_row(
            SharedString::from(format!("{row_id}-blockers")),
            IconName::Warning,
            Color::Warning,
            format!("{} blocker(s)", summary.blocker_count),
        ));
    }

    stack = stack.child(
        Label::new(summary.label.clone())
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
    );

    stack.into_any_element()
}
