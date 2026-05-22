use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};

use crate::dx_deploy_launch_evidence_rail::deploy_launch_evidence_state;
use crate::dx_deploy_launch_gate::{DxDeployLaunchGateNotice, DxDeployLaunchGateSnapshot};
use crate::dx_deploy_rail_ui::{metric_row, muted_card, muted_label, signal_row};

pub(crate) fn deploy_launch_gate_state(
    snapshot: &DxDeployLaunchGateSnapshot,
    cx: &App,
) -> AnyElement {
    if !snapshot.receipt_found {
        return muted_card("No dx-check launch receipt", cx);
    }

    let mut stack = v_flex()
        .id("dx-deploy-launch-gate")
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row("Launch gate", launch_gate_status(snapshot)))
        .child(metric_row(
            "Approvals",
            launch_gate_approval_summary(snapshot),
        ));

    if let Some(schema) = snapshot.schema_version.as_ref() {
        stack = stack.child(muted_label(schema.clone()));
    }

    if let Some(command) = snapshot.command.as_ref() {
        stack = stack.child(muted_label(command.clone()));
    }

    if !snapshot.evidence_sources.is_empty() || snapshot.chain.is_some() {
        stack = stack.child(deploy_launch_evidence_state(
            &snapshot.evidence_sources,
            snapshot.chain.as_ref(),
        ));
    }

    if snapshot.blocker_count > 0 {
        stack = stack.child(signal_row(
            SharedString::from("dx-deploy-launch-gate-blockers"),
            IconName::Warning,
            Color::Warning,
            format!("{} blocker(s)", snapshot.blocker_count),
        ));
    }

    for (ix, blocker) in snapshot.blockers.iter().take(2).enumerate() {
        stack = stack.child(launch_gate_notice_row(
            SharedString::from(format!("dx-deploy-launch-gate-blocker-{ix}")),
            blocker,
        ));
    }

    if let Some(next_action) = snapshot.next_action.as_ref() {
        stack = stack.child(muted_label(next_action.clone()));
    }

    stack
        .child(muted_label(snapshot.label.clone()))
        .into_any_element()
}

fn launch_gate_status(snapshot: &DxDeployLaunchGateSnapshot) -> String {
    let mut parts = Vec::new();

    if let Some(status) = snapshot.status.as_ref() {
        parts.push(status.clone());
    }
    if let (Some(score), Some(max_score)) = (snapshot.score, snapshot.max_score) {
        parts.push(format!("{score}/{max_score}"));
    }
    if snapshot.launch_approved == Some(false) {
        parts.push("launch blocked".to_string());
    }
    if snapshot.warning_count > 0 {
        parts.push(format!("{} warning(s)", snapshot.warning_count));
    }

    if parts.is_empty() {
        "receipt ready".to_string()
    } else {
        parts.join(" - ")
    }
}

fn launch_gate_approval_summary(snapshot: &DxDeployLaunchGateSnapshot) -> String {
    [
        (
            "source",
            snapshot.source_status.as_ref(),
            snapshot.source_approved,
        ),
        (
            "runtime",
            snapshot.runtime_status.as_ref(),
            snapshot.runtime_approved,
        ),
        (
            "launch",
            snapshot.launch_status.as_ref(),
            snapshot.launch_approved,
        ),
    ]
    .into_iter()
    .map(|(label, status, approved)| {
        let state = status
            .cloned()
            .unwrap_or_else(|| approval_state_label(approved));
        format!("{label} {state}")
    })
    .collect::<Vec<_>>()
    .join(" / ")
}

fn approval_state_label(approved: Option<bool>) -> String {
    match approved {
        Some(true) => "approved".to_string(),
        Some(false) => "blocked".to_string(),
        None => "unknown".to_string(),
    }
}

fn launch_gate_notice_row(id: SharedString, notice: &DxDeployLaunchGateNotice) -> AnyElement {
    let label = notice
        .code
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "Launch blocker".to_string());
    let mut stack = v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .child(metric_row(label, notice.message.clone()));

    if let Some(next_action) = notice.next_action.as_ref() {
        stack = stack.child(muted_label(next_action.clone()));
    }

    stack.into_any_element()
}
