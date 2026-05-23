use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_launch_receipts::DxLaunchReceiptReviewSnapshot;

use super::super::{muted_card, signal_row};
use super::rows::launch_receipt_row;

pub(super) fn launch_receipt_status_rows(
    snapshot: &DxLaunchReceiptReviewSnapshot,
    cx: &App,
) -> Vec<AnyElement> {
    let mut rows = Vec::new();

    if !snapshot.root_exists {
        rows.push(muted_card(
            format!(
                "Missing launch receipt directory: {}",
                snapshot.root.display()
            ),
            cx,
        ));
    } else if !snapshot.latest_present {
        rows.push(muted_card(
            format!(
                "No cached launch latest receipt at {}",
                snapshot.latest_path.display()
            ),
            cx,
        ));
    } else if let Some(latest) = snapshot.latest.as_ref() {
        rows.push(launch_receipt_row(latest, "Latest Receipt", cx));

        if latest.malformed {
            rows.push(signal_row(
                "dx-launch-receipt-latest-malformed".into(),
                IconName::Warning,
                Color::Warning,
                "Run dx launch receipts --json to inspect malformed launch receipt metadata."
                    .to_string(),
            ));
        } else if latest.freshness_state == "stale" || latest.freshness_state == "expired" {
            rows.push(signal_row(
                "dx-launch-receipt-latest-stale".into(),
                IconName::Warning,
                Color::Warning,
                format!(
                    "Cached launch status receipt is {}; run dx launch status --json before trusting it.",
                    latest.freshness_state
                ),
            ));
        } else if !latest.schema_matches_launch_status() {
            rows.push(signal_row(
                "dx-launch-receipt-schema-review".into(),
                IconName::Warning,
                Color::Warning,
                "Latest launch receipt does not advertise dx.launch.status.v1.".to_string(),
            ));
        }
    }

    if let Some(error) = snapshot.last_error.as_ref() {
        rows.push(signal_row(
            "dx-launch-receipt-warning".into(),
            IconName::Warning,
            Color::Warning,
            error.clone(),
        ));
    } else {
        rows.push(
            Label::new(snapshot.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate()
                .into_any_element(),
        );
    }

    if let Some(snapshot_receipt) = snapshot.snapshots.first() {
        rows.push(launch_receipt_row(snapshot_receipt, "Latest Snapshot", cx));
    }

    rows
}
