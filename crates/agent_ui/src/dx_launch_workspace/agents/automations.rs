use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_agent_bridge::{DxAgentAutomation, DxAgentBridgeSnapshot};

use super::super::{metric_row, muted_card};
use super::actions::dx_agent_action_line;

pub(in super::super) fn dx_agent_automation_state(
    snapshot: &DxAgentBridgeSnapshot,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Count", snapshot.automation_count.to_string()))
        .child(metric_row("Active", snapshot.active_task_count.to_string()))
        .child(metric_row(
            "Command",
            "dx agents automate list --json".to_string(),
        ));

    if snapshot.automations.is_empty() {
        stack = stack.child(muted_card("Run automation list receipt", cx));
    } else {
        for (ix, automation) in snapshot.automations.iter().take(3).enumerate() {
            stack = stack.child(dx_agent_automation_row(
                SharedString::from(format!("dx-agent-automation-{ix}")),
                automation,
                cx,
            ));
        }
    }

    stack.into_any_element()
}

fn dx_agent_automation_row(
    id: SharedString,
    automation: &DxAgentAutomation,
    cx: &App,
) -> AnyElement {
    let state = if automation.enabled {
        automation.status.clone()
    } else {
        "paused".to_string()
    };

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(automation.id.clone(), state))
        .child(
            Label::new(format!(
                "{} schedule from {}",
                automation.schedule_kind, automation.source
            ))
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        )
        .when(!automation.next_action.is_empty(), |this| {
            this.child(
                Label::new(automation.next_action.clone())
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(
            dx_agent_action_line(&automation.actions),
            |this, action_line| {
                this.child(
                    Label::new(action_line)
                        .size(LabelSize::XSmall)
                        .color(Color::Muted)
                        .truncate(),
                )
            },
        )
        .into_any_element()
}
