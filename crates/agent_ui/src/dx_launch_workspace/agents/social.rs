use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_agent_bridge::{DxAgentBridgeSnapshot, DxAgentSocialAccount};

use super::super::{metric_row, muted_card};
use super::actions::dx_agent_action_line;
use super::social_actions::dx_agent_social_action_row;

pub(in super::super) fn dx_agent_social_state(
    snapshot: &DxAgentBridgeSnapshot,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row(
            "Supported",
            snapshot.connected_accounts_summary.supported.to_string(),
        ))
        .child(metric_row(
            "Needs auth",
            snapshot.connected_accounts_summary.needs_auth.to_string(),
        ))
        .child(metric_row(
            "QR-ready",
            snapshot
                .connected_accounts_summary
                .qr_connect_supported
                .to_string(),
        ));

    if snapshot.social_accounts.is_empty() {
        stack = stack.child(muted_card("Run social list receipt", cx));
    } else {
        for (ix, account) in snapshot.social_accounts.iter().take(3).enumerate() {
            stack = stack.child(dx_agent_social_row(
                SharedString::from(format!("dx-agent-social-{ix}")),
                account,
                cx,
            ));
        }
    }

    if snapshot.social_connect.present {
        stack = stack.child(dx_agent_social_action_row(
            SharedString::from("dx-agent-social-connect-receipt"),
            &snapshot.social_connect,
            cx,
        ));
    }

    if snapshot.social_disconnect.present {
        stack = stack.child(dx_agent_social_action_row(
            SharedString::from("dx-agent-social-disconnect-receipt"),
            &snapshot.social_disconnect,
            cx,
        ));
    }

    stack.into_any_element()
}

fn dx_agent_social_row(id: SharedString, account: &DxAgentSocialAccount, cx: &App) -> AnyElement {
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
