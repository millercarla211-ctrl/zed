use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use self::details::dx_agent_bridge_detail_rows;
use self::review::dx_agent_bridge_review_row;
use self::summary::dx_agent_bridge_summary_rows;
use super::super::{muted_card, signal_row};

mod details;
mod review;
mod summary;

pub(in super::super) fn dx_agent_bridge_state(
    snapshot: &DxAgentBridgeSnapshot,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .children(dx_agent_bridge_summary_rows(snapshot));

    if !snapshot.enabled {
        stack = stack.child(muted_card("Disabled in Zed settings", cx));
    } else if snapshot.action_error.present {
        let error = snapshot
            .action_error
            .error
            .clone()
            .unwrap_or_else(|| "DX Agents action failed".to_string());
        stack = stack.child(signal_row(
            "dx-agent-action-error".into(),
            IconName::Warning,
            Color::Warning,
            format!("Last DX Agents action failed: {error}"),
        ));
    } else if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!("Missing receipts: {}", snapshot.receipt_root.display()),
            cx,
        ));
    } else if !snapshot.contract_summary.present {
        stack = stack.child(muted_card("Run dx agents contract --json", cx));
    }
    if snapshot.enabled && snapshot.root_exists && !snapshot.import_summary.present {
        stack = stack.child(muted_card("Run dx agents import-summary --json", cx));
    }
    if snapshot.enabled && snapshot.root_exists && !snapshot.release_gate.present {
        stack = stack.child(muted_card("Run dx agents release-gate --json", cx));
    }

    stack = stack.children(dx_agent_bridge_detail_rows(snapshot));
    stack = stack.child(dx_agent_bridge_review_row(snapshot));

    stack.into_any_element()
}
