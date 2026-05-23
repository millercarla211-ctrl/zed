use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_launch_receipts::{DxLaunchReceiptReviewSnapshot, DxLaunchReceiptSummary};

use super::{metric_row, muted_card, signal_row};

pub(super) fn launch_receipt_review_state(
    snapshot: &DxLaunchReceiptReviewSnapshot,
    cx: &App,
) -> AnyElement {
    let latest_state = snapshot
        .latest
        .as_ref()
        .map(DxLaunchReceiptSummary::display_state)
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

    if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!(
                "Missing launch receipt directory: {}",
                snapshot.root.display()
            ),
            cx,
        ));
    } else if !snapshot.latest_present {
        stack = stack.child(muted_card(
            format!(
                "No cached launch latest receipt at {}",
                snapshot.latest_path.display()
            ),
            cx,
        ));
    } else if let Some(latest) = snapshot.latest.as_ref() {
        stack = stack.child(launch_receipt_row(latest, "Latest Receipt", cx));

        if latest.malformed {
            stack = stack.child(signal_row(
                "dx-launch-receipt-latest-malformed".into(),
                IconName::Warning,
                Color::Warning,
                "Run dx launch receipts --json to inspect malformed launch receipt metadata."
                    .to_string(),
            ));
        } else if latest.freshness_state == "stale" || latest.freshness_state == "expired" {
            stack = stack.child(signal_row(
                "dx-launch-receipt-latest-stale".into(),
                IconName::Warning,
                Color::Warning,
                format!(
                    "Cached launch status receipt is {}; run dx launch status --json before trusting it.",
                    latest.freshness_state
                ),
            ));
        } else if !latest.schema_matches_launch_status() {
            stack = stack.child(signal_row(
                "dx-launch-receipt-schema-review".into(),
                IconName::Warning,
                Color::Warning,
                "Latest launch receipt does not advertise dx.launch.status.v1.".to_string(),
            ));
        }
    }

    if let Some(error) = snapshot.last_error.as_ref() {
        stack = stack.child(signal_row(
            "dx-launch-receipt-warning".into(),
            IconName::Warning,
            Color::Warning,
            error.clone(),
        ));
    } else {
        stack = stack.child(
            Label::new(snapshot.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if let Some(snapshot_receipt) = snapshot.snapshots.first() {
        stack = stack.child(launch_receipt_row(snapshot_receipt, "Latest Snapshot", cx));
    }

    stack.into_any_element()
}

fn launch_receipt_row(
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
