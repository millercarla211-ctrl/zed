use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};

use crate::dx_deploy_launch_gate_rail::deploy_launch_gate_state;
use crate::dx_deploy_matrix_rail::deploy_capability_matrix_state;
use crate::dx_deploy_rail_ui::{metric_row, muted_card, muted_label, signal_row, source_row};
use crate::dx_deploy_targets::{
    DxDeployReceiptBucket, DxDeployReceiptSummary, DxDeployTarget, DxDeployTargetSnapshot,
};

pub(crate) fn deploy_target_state(snapshot: &DxDeployTargetSnapshot, cx: &App) -> AnyElement {
    if snapshot.workspace_root_count == 0 {
        return muted_card("No workspace", cx);
    }

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Targets", snapshot.targets.len().to_string()))
        .child(metric_row(
            "Deploy receipts",
            snapshot.receipt_count.to_string(),
        ))
        .child(deploy_launch_gate_state(&snapshot.launch_gate, cx))
        .child(deploy_capability_matrix_state(
            &snapshot.capability_matrix,
            cx,
        ))
        .child(deploy_receipt_bucket_stack(snapshot, cx));

    for (ix, target) in snapshot.targets.iter().take(3).enumerate() {
        stack = stack.child(deploy_target_row(
            SharedString::from(format!("dx-deploy-target-{ix}")),
            target,
            cx,
        ));
    }

    if snapshot.targets.is_empty() {
        stack = stack.child(muted_card("No deploy target config", cx));
    }

    if snapshot.receipt_root_exists {
        for (ix, label) in snapshot.latest_receipts.iter().take(2).enumerate() {
            stack = stack.child(source_row(
                SharedString::from(format!("dx-deploy-receipt-{ix}")),
                IconName::FileTextOutlined,
                label.clone(),
                cx,
            ));
        }
    }

    stack.into_any_element()
}

fn deploy_receipt_bucket_stack(snapshot: &DxDeployTargetSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex().gap_1().child(metric_row(
        "Proof buckets",
        format!("{} tracked", snapshot.receipt_buckets.len()),
    ));

    for (ix, bucket) in snapshot.receipt_buckets.iter().enumerate() {
        stack = stack.child(deploy_receipt_bucket_row(
            SharedString::from(format!("dx-deploy-receipt-bucket-{ix}")),
            bucket,
            cx,
        ));
    }

    stack.into_any_element()
}

fn deploy_receipt_bucket_row(
    id: SharedString,
    bucket: &DxDeployReceiptBucket,
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
        .child(metric_row(bucket.label, state));

    if !bucket.root_exists {
        stack = stack.child(muted_label(bucket.root_label));
    } else {
        if let Some(summary) = bucket.latest_summary.as_ref() {
            stack = stack
                .child(
                    Label::new(summary.headline.clone())
                        .size(LabelSize::XSmall)
                        .color(Color::Default)
                        .truncate(),
                )
                .child(muted_label(deploy_receipt_summary_detail(summary)));

            if summary.blocker_count > 0 {
                stack = stack.child(signal_row(
                    SharedString::from(format!("dx-deploy-{}-blockers", bucket.label)),
                    IconName::Warning,
                    Color::Warning,
                    format!("{} blocker(s)", summary.blocker_count),
                ));
            }
        }

        if let Some(label) = bucket.latest.first() {
            stack = stack.child(muted_label(label.clone()));
        }
    }

    stack.into_any_element()
}

fn deploy_receipt_summary_detail(summary: &DxDeployReceiptSummary) -> String {
    let mut details = Vec::new();

    if let Some(status) = summary.status.as_ref() {
        details.push(format!("Status {status}"));
    }

    if let Some(target) = summary.target.as_ref() {
        details.push(format!("Target {target}"));
    }

    if let Some(url) = summary.url.as_ref() {
        details.push(url.clone());
    }

    if details.is_empty() {
        summary.label.clone()
    } else {
        details.join(" - ")
    }
}

fn deploy_target_row(id: SharedString, target: &DxDeployTarget, cx: &App) -> AnyElement {
    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(
            h_flex()
                .gap_1()
                .min_w_0()
                .items_center()
                .child(
                    Icon::new(deploy_platform_icon(target.platform))
                        .size(IconSize::XSmall)
                        .color(Color::Muted),
                )
                .child(
                    Label::new(target.label.clone())
                        .size(LabelSize::XSmall)
                        .color(Color::Default)
                        .truncate(),
                ),
        )
        .child(muted_label(target.detail.clone()))
        .child(muted_label(target.path.clone()))
        .into_any_element()
}

fn deploy_platform_icon(platform: &str) -> IconName {
    match platform {
        "Vercel" => IconName::AiVercel,
        "Cloudflare" => IconName::Server,
        "Docker" => IconName::Box,
        _ => IconName::Public,
    }
}
