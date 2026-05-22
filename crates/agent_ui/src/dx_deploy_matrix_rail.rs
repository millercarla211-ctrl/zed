use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};

use crate::dx_deploy_capabilities::{
    DxDeployCapabilityMatrixSnapshot, DxDeployCapabilityRow, DxDeployCommandReceiptSummary,
};
use crate::dx_deploy_gate_rail::deploy_provider_gate_state;
use crate::dx_deploy_rail_ui::{metric_row, muted_card, muted_label, signal_row, source_row};

pub(crate) fn deploy_capability_matrix_state(
    snapshot: &DxDeployCapabilityMatrixSnapshot,
    cx: &App,
) -> AnyElement {
    if !snapshot.root_exists {
        return muted_card("No .dx deploy receipts", cx);
    }

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row(
            "DX receipts",
            snapshot.receipt_count.to_string(),
        ))
        .child(metric_row("Providers", deploy_provider_summary(snapshot)));

    if let Some(provider_gate) = snapshot.provider_gate.as_ref() {
        stack = stack.child(deploy_provider_gate_state(provider_gate, cx));
    }

    if let Some(status) = snapshot.status.as_ref() {
        stack = stack.child(deploy_command_receipt_row(
            SharedString::from("dx-deploy-status-receipt"),
            "Status",
            status,
            cx,
        ));
    }

    if let Some(plan) = snapshot.plan.as_ref() {
        stack = stack.child(deploy_command_receipt_row(
            SharedString::from("dx-deploy-plan-receipt"),
            "Plan",
            plan,
            cx,
        ));
    }

    if snapshot.providers.is_empty() {
        stack = stack.child(muted_card("Provider capability matrix missing", cx));
    } else {
        for (ix, provider) in snapshot.providers.iter().take(4).enumerate() {
            stack = stack.child(deploy_capability_provider_row(
                SharedString::from(format!("dx-deploy-provider-{ix}")),
                provider,
                cx,
            ));
        }
    }

    for (ix, label) in snapshot.latest_receipts.iter().take(2).enumerate() {
        stack = stack.child(source_row(
            SharedString::from(format!("dx-deploy-capability-receipt-{ix}")),
            IconName::FileTextOutlined,
            label.clone(),
            cx,
        ));
    }

    stack.into_any_element()
}

fn deploy_command_receipt_row(
    id: SharedString,
    title: &'static str,
    receipt: &DxDeployCommandReceiptSummary,
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
        .child(metric_row(title, deploy_command_receipt_status(receipt)));

    if let Some(schema) = receipt.schema_version.as_ref() {
        stack = stack.child(muted_label(schema.clone()));
    }

    if let Some(command) = receipt.command.as_ref() {
        stack = stack.child(muted_label(command.clone()));
    }

    if let Some(fixture) = receipt.fixture_label.as_ref() {
        stack = stack.child(muted_label(fixture.clone()));
    }

    if let Some(next_action) = receipt.next_action.as_ref() {
        stack = stack.child(muted_label(next_action.clone()));
    }

    if receipt.blocker_count > 0 {
        stack = stack.child(signal_row(
            SharedString::from(format!("dx-deploy-{title}-blockers")),
            IconName::Warning,
            Color::Warning,
            format!("{} blocker(s)", receipt.blocker_count),
        ));
    }

    stack.into_any_element()
}

fn deploy_command_receipt_status(receipt: &DxDeployCommandReceiptSummary) -> String {
    let mut parts = Vec::new();

    if let Some(status) = receipt.status.as_ref() {
        parts.push(status.clone());
    }
    if let Some(latest_plan_status) = receipt.latest_plan_status.as_ref() {
        parts.push(format!("plan {latest_plan_status}"));
    }
    if receipt.dry_run == Some(true) {
        parts.push("dry-run".to_string());
    }
    if receipt.deploy_ran == Some(false) {
        parts.push("no deploy ran".to_string());
    }
    if receipt.provider_count > 0 {
        parts.push(format!("{} provider(s)", receipt.provider_count));
    }
    if let Some(ready) = receipt.ready_plan_count {
        parts.push(format!("{ready} ready"));
    }
    if let Some(blocked) = receipt.blocked_plan_count {
        parts.push(format!("{blocked} blocked"));
    }
    if receipt.warning_count > 0 {
        parts.push(format!("{} warning(s)", receipt.warning_count));
    }

    if parts.is_empty() {
        receipt.label.clone()
    } else {
        parts.join(" - ")
    }
}

fn deploy_capability_provider_row(
    id: SharedString,
    provider: &DxDeployCapabilityRow,
    cx: &App,
) -> AnyElement {
    let mut detail = vec![
        provider.current_support.clone(),
        provider.write_support.clone(),
    ];
    if provider.dry_run {
        detail.push("dry-run".to_string());
    }
    if provider.needs_credentials_count > 0 {
        detail.push(format!(
            "{} credential hint(s)",
            provider.needs_credentials_count
        ));
    }

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
                    Icon::new(deploy_provider_icon(&provider.id))
                        .size(IconSize::XSmall)
                        .color(Color::Muted),
                )
                .child(
                    Label::new(provider.name.clone())
                        .size(LabelSize::XSmall)
                        .color(Color::Default)
                        .truncate(),
                ),
        )
        .child(muted_label(detail.join(" - ")))
        .child(muted_label(provider.target_kinds.join(", ")))
        .into_any_element()
}

fn deploy_provider_summary(snapshot: &DxDeployCapabilityMatrixSnapshot) -> String {
    if snapshot.providers.is_empty() {
        return "matrix missing".to_string();
    }

    let dry_run_count = snapshot
        .providers
        .iter()
        .filter(|provider| provider.dry_run)
        .count();
    let credential_count = snapshot
        .providers
        .iter()
        .filter(|provider| provider.needs_credentials_count > 0)
        .count();

    format!(
        "{} total / {} dry-run / {} need credentials",
        snapshot.providers.len(),
        dry_run_count,
        credential_count
    )
}

fn deploy_provider_icon(provider_id: &str) -> IconName {
    match provider_id {
        "vercel" => IconName::AiVercel,
        "cloudflare-workers" | "s3-r2" => IconName::Server,
        _ => IconName::Public,
    }
}
