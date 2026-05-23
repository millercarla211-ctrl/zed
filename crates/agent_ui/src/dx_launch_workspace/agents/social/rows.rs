use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_agent_bridge::DxAgentSocialAccount;

use super::super::super::metric_row;
use super::super::actions::dx_agent_action_line;

pub(super) fn dx_agent_social_row(
    id: SharedString,
    account: &DxAgentSocialAccount,
    cx: &App,
) -> AnyElement {
    let state = if account.connected {
        "Connected".to_string()
    } else if account.qr_connect_supported {
        "QR ready".to_string()
    } else if account.configured {
        "Configured".to_string()
    } else {
        "Needs setup".to_string()
    };

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(account.platform.clone(), state))
        .child(
            Label::new(format!("{} - {}", account.label, account.status))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .when(!account.next_action.is_empty(), |this| {
            this.child(
                Label::new(account.next_action.clone())
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(
            dx_agent_action_line(&account.actions),
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
