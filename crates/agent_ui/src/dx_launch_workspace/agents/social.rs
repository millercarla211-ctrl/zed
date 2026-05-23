use gpui::{AnyElement, App, SharedString, prelude::*};

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use self::rows::dx_agent_social_row;
use super::super::{metric_row, muted_card};
use super::social_actions::dx_agent_social_action_row;

mod rows;

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
