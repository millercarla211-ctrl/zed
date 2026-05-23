use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};

use crate::dx_receipt_history::DxToolHistoryBucket;

use super::super::{metric_row, source_row};
use super::summary_rows::tool_history_summary_row;

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
