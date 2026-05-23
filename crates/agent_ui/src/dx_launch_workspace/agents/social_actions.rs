use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_agent_bridge::DxAgentSocialActionSummary;

use super::super::metric_row;

pub(super) fn dx_agent_social_action_row(
    id: SharedString,
    receipt: &DxAgentSocialActionSummary,
    cx: &App,
) -> AnyElement {
    let connected = if receipt.connected.unwrap_or(false) {
        "connected"
    } else {
        "not connected"
    };
    let detail = if receipt.action == "connect" {
        let support = if receipt.connect_supported {
            "supported"
        } else {
            "unsupported"
        };
        let qr = if receipt.qr_supported {
            "QR ready"
        } else {
            "QR unavailable"
        };
        let link = if receipt.link_supported {
            "link ready"
        } else {
            "link unavailable"
        };
        format!(
            "{} connect {}, via {}, {}, {}, {}",
            receipt.label, support, receipt.connect_method, qr, link, connected
        )
    } else {
        let support = if receipt.disconnect_supported {
            "supported"
        } else {
            "not needed"
        };
        let revoke = if receipt.manual_revoke_required {
            "provider revoke"
        } else {
            "no revoke"
        };
        format!(
            "{} disconnect {}, {}, {}, config {}",
            receipt.label, support, revoke, connected, receipt.safe_config_state
        )
    };

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(
            format!("Last {}", receipt.action),
            receipt.status.clone(),
        ))
        .child(
            Label::new(detail)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(
            Label::new(receipt.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}
