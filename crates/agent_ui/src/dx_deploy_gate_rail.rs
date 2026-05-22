use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};

use crate::dx_deploy_capabilities::{
    DxDeployProviderGateQuickFix, DxDeployProviderGateReceiptSummary, DxDeployProviderGateRow,
};
use crate::dx_deploy_rail_ui::{metric_row, muted_label, signal_row};

pub(crate) fn deploy_provider_gate_state(
    receipt: &DxDeployProviderGateReceiptSummary,
    cx: &App,
) -> AnyElement {
    let title = receipt
        .zed_title
        .as_deref()
        .unwrap_or("Provider deploy gate");
    let mut stack = v_flex()
        .id("dx-deploy-gate")
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(title, deploy_provider_gate_status(receipt)));

    if let Some(summary) = receipt.zed_summary.as_ref() {
        stack = stack.child(muted_label(summary.clone()));
    }

    if let Some(schema) = receipt.schema_version.as_ref() {
        stack = stack.child(muted_label(schema.clone()));
    }

    if let Some(panel_kind) = receipt.zed_panel_kind.as_ref() {
        stack = stack.child(muted_label(panel_kind.clone()));
    }

    if let Some(command) = receipt.command.as_ref() {
        stack = stack.child(muted_label(command.clone()));
    }

    if receipt.blocker_count > 0 {
        stack = stack.child(signal_row(
            SharedString::from("dx-deploy-gate-blockers"),
            IconName::Warning,
            Color::Warning,
            format!("{} blocker(s)", receipt.blocker_count),
        ));
    }

    for row in receipt.rows.iter().take(3) {
        stack = stack.child(deploy_provider_gate_row(row, cx));
    }

    for quick_fix in receipt.quick_fixes.iter().take(3) {
        stack = stack.child(deploy_provider_gate_quick_fix(quick_fix, cx));
    }

    if let Some(next_action) = receipt.next_action.as_ref() {
        stack = stack.child(muted_label(next_action.clone()));
    }

    stack.into_any_element()
}

fn deploy_provider_gate_quick_fix(
    quick_fix: &DxDeployProviderGateQuickFix,
    cx: &App,
) -> AnyElement {
    v_flex()
        .id(SharedString::from(format!(
            "dx-deploy-gate-quick-fix-{}",
            quick_fix.id
        )))
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().editor_background)
        .child(metric_row(
            quick_fix.label.clone(),
            quick_fix.risk_level.clone(),
        ))
        .child(muted_label(deploy_quick_fix_detail(quick_fix)))
        .into_any_element()
}

fn deploy_quick_fix_detail(quick_fix: &DxDeployProviderGateQuickFix) -> String {
    let approval = if quick_fix.requires_user_approval {
        "requires approval"
    } else {
        "no approval required"
    };
    let receipt_write = if quick_fix.writes_receipts {
        "writes receipt"
    } else {
        "read-only"
    };

    format!("{} - {} - {}", quick_fix.command, approval, receipt_write)
}

fn deploy_provider_gate_status(receipt: &DxDeployProviderGateReceiptSummary) -> String {
    let mut parts = Vec::new();

    if let Some(provider) = receipt.provider.as_ref() {
        parts.push(provider.clone());
    }
    if let Some(status) = receipt.status.as_ref() {
        parts.push(status.clone());
    }
    match receipt.deploy_ready {
        Some(true) => parts.push("ready".to_string()),
        Some(false) => parts.push("not ready".to_string()),
        None => {}
    }
    if receipt.dry_run == Some(true) {
        parts.push("dry-run".to_string());
    }
    if receipt.deploy_ran == Some(false) {
        parts.push("no deploy ran".to_string());
    }
    if receipt.blocker_count > 0 {
        parts.push(format!("{} blocker(s)", receipt.blocker_count));
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

fn deploy_provider_gate_row(row: &DxDeployProviderGateRow, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .id(SharedString::from(format!("dx-deploy-gate-{}", row.id)))
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().editor_background)
        .child(metric_row(row.label.clone(), row.status.clone()));

    if let Some(detail) = row.detail.as_ref() {
        stack = stack.child(muted_label(detail.clone()));
    }

    stack.into_any_element()
}
