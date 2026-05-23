use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_receipt_history::{DxToolHistoryBucket, DxToolHistoryReceiptSummary};

use super::super::{metric_row, signal_row, source_row};

pub(super) fn tool_history_bucket(
    id: SharedString,
    bucket: &DxToolHistoryBucket,
    cx: &App,
) -> AnyElement {
    let state = if !bucket.root_exists {
        format!("Missing: {}", bucket.root_label)
    } else if bucket.count == 0 {
        "No receipts".to_string()
    } else {
        format!("{} receipts", bucket.count)
    };
    let mut stack = v_flex()
        .id(id)
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(cx.theme().colors().border_variant)
        .px_2()
        .py_1()
        .child(metric_row(bucket.label, state));

    if bucket.root_exists {
        let bucket_id = bucket.label.to_ascii_lowercase().replace(' ', "-");
        for (ix, summary) in bucket.latest_summaries.iter().enumerate() {
            let row_id = format!("{bucket_id}-summary-{ix}");
            stack = stack.child(tool_history_summary_row(
                SharedString::from(row_id.clone()),
                row_id,
                summary,
                cx,
            ));
        }

        for (ix, label) in bucket.latest.iter().enumerate() {
            stack = stack.child(source_row(
                SharedString::from(format!("{bucket_id}-latest-{ix}")),
                IconName::FileTextOutlined,
                label.clone(),
                cx,
            ));
        }
    }

    stack.into_any_element()
}

fn tool_history_summary_row(
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
