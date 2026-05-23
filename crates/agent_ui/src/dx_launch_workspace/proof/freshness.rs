use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_proof_freshness::{DxProofFreshnessBucket, DxProofFreshnessSnapshot};

use super::super::metric_row;

pub(in super::super) fn proof_freshness_state(
    snapshot: &DxProofFreshnessSnapshot,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex().gap_1();

    for (ix, bucket) in snapshot.buckets.iter().enumerate() {
        stack = stack.child(proof_freshness_bucket_row(
            SharedString::from(format!("dx-proof-freshness-{ix}")),
            bucket,
            cx,
        ));
    }

    stack.into_any_element()
}

fn proof_freshness_bucket_row(
    id: SharedString,
    bucket: &DxProofFreshnessBucket,
    cx: &App,
) -> AnyElement {
    let state = if bucket.count == 0 {
        bucket.status.clone()
    } else {
        format!("{} - {}", bucket.count, bucket.status)
    };
    let mut stack = v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(bucket.label, state))
        .child(
            Label::new(bucket.description)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );

    if !bucket.latest.is_empty() {
        for label in bucket.latest.iter().take(2) {
            stack = stack.child(
                Label::new(label.clone())
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            );
        }
    } else if !bucket.root_exists {
        stack = stack.child(
            Label::new(bucket.root_label)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    stack.into_any_element()
}
