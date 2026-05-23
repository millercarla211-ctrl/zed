use gpui::{AnyElement, App, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_launch_receipts::DxLaunchReceiptReviewSnapshot;

use self::status::launch_receipt_status_rows;
use super::metric_row;

mod rows;
mod status;

pub(super) fn launch_receipt_review_state(
    snapshot: &DxLaunchReceiptReviewSnapshot,
    cx: &App,
) -> AnyElement {
    let latest_state = snapshot
        .latest
        .as_ref()
        .map(|receipt| receipt.display_state())
        .unwrap_or_else(|| "missing".to_string());

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Review", snapshot.status.clone()))
        .child(
            Label::new(snapshot.operator_summary.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row("Latest", latest_state))
        .child(metric_row("Snapshots", snapshot.snapshot_count.to_string()))
        .child(metric_row(
            "Malformed",
            snapshot.malformed_count.to_string(),
        ))
        .child(metric_row(
            "Stale/Expired",
            format!("{} / {}", snapshot.stale_count, snapshot.expired_count),
        ))
        .child(metric_row("Schema", snapshot.schema_version.clone()))
        .child(metric_row(
            "Thresholds",
            format!(
                "{}ms stale / {}ms expired",
                snapshot.stale_after_ms, snapshot.expired_after_ms
            ),
        ))
        .child(metric_row("Command", snapshot.command.clone()));

    stack = stack.children(launch_receipt_status_rows(snapshot, cx));

    stack.into_any_element()
}
