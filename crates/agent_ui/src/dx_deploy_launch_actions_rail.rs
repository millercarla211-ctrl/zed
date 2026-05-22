use gpui::{AnyElement, SharedString, prelude::*};

use crate::dx_deploy_launch_action_labels::launch_action_detail_parts;
use crate::dx_deploy_launch_actions::DxDeployLaunchAction;
use crate::dx_deploy_rail_ui::{metric_row, muted_label};

pub(crate) fn deploy_launch_action_state(
    actions: &[DxDeployLaunchAction],
    total_count: usize,
) -> AnyElement {
    let available_count = total_count.max(actions.len());
    let mut stack = v_flex()
        .id("dx-deploy-launch-actions")
        .gap_0p5()
        .min_w_0()
        .child(metric_row(
            "Actions",
            format!("{} shown of {available_count} available", actions.len()),
        ));

    for (ix, action) in actions.iter().take(5).enumerate() {
        stack = stack.child(launch_action_row(
            SharedString::from(format!("dx-deploy-launch-action-{ix}")),
            action,
        ));
    }

    stack.into_any_element()
}

fn launch_action_row(id: SharedString, action: &DxDeployLaunchAction) -> AnyElement {
    let detail = launch_action_detail_parts(
        action.id.as_deref(),
        action.risk_level.as_deref(),
        action.requires_user_approval,
        action.writes_receipts,
        action.command.is_some(),
    )
    .join(" - ");

    let mut stack = v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .child(metric_row(action.label.clone(), detail));

    if let Some(command) = action.command.as_ref() {
        stack = stack.child(muted_label(command.clone()));
    }

    if let Some(next_action) = action.next_action.as_ref() {
        stack = stack.child(muted_label(next_action.clone()));
    }

    stack.into_any_element()
}
