use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use self::review::dx_agent_bridge_review_row;
use self::summary::dx_agent_bridge_summary_rows;
use super::super::{metric_row, muted_card, signal_row};

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

    if let Some(receipt) = snapshot.latest_receipts.first() {
        stack = stack.child(metric_row("Latest", receipt.clone()));
    }
    if snapshot.action_error.present && !snapshot.action_error.command.is_empty() {
        stack = stack.child(metric_row(
            "Failed Command",
            snapshot.action_error.command.clone(),
        ));
    }
    if snapshot.action_error.redaction_requires_review {
        stack = stack.child(signal_row(
            "dx-agent-action-error-redaction".into(),
            IconName::Warning,
            Color::Warning,
            snapshot.action_error.redaction_summary.clone(),
        ));
    }
    if let Some(command) = snapshot.contract_summary.commands.first() {
        stack = stack.child(metric_row("Command", command.clone()));
    } else if snapshot.contract_summary.present {
        stack = stack.child(metric_row(
            "Catalog Regen",
            snapshot.contract_summary.safe_regeneration_command.clone(),
        ));
    }
    if let Some(command) = snapshot.import_summary.recovery_commands.first() {
        stack = stack.child(metric_row("Import Command", command.clone()));
    }
    if let Some(row) = snapshot.release_gate.acceptance_rows.first() {
        stack = stack.child(metric_row("Gate Row", row.clone()));
    }

    stack = stack.child(dx_agent_bridge_review_row(snapshot));

    stack.into_any_element()
}
