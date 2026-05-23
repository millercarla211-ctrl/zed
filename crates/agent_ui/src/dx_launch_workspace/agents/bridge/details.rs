use gpui::AnyElement;
use ui::{Color, IconName};

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::super::super::{metric_row, signal_row};

pub(super) fn dx_agent_bridge_detail_rows(snapshot: &DxAgentBridgeSnapshot) -> Vec<AnyElement> {
    let mut rows = Vec::new();

    if let Some(receipt) = snapshot.latest_receipts.first() {
        rows.push(metric_row("Latest", receipt.clone()));
    }
    if snapshot.action_error.present && !snapshot.action_error.command.is_empty() {
        rows.push(metric_row(
            "Failed Command",
            snapshot.action_error.command.clone(),
        ));
    }
    if snapshot.action_error.redaction_requires_review {
        rows.push(signal_row(
            "dx-agent-action-error-redaction".into(),
            IconName::Warning,
            Color::Warning,
            snapshot.action_error.redaction_summary.clone(),
        ));
    }
    if let Some(command) = snapshot.contract_summary.commands.first() {
        rows.push(metric_row("Command", command.clone()));
    } else if snapshot.contract_summary.present {
        rows.push(metric_row(
            "Catalog Regen",
            snapshot.contract_summary.safe_regeneration_command.clone(),
        ));
    }
    if let Some(command) = snapshot.import_summary.recovery_commands.first() {
        rows.push(metric_row("Import Command", command.clone()));
    }
    if let Some(row) = snapshot.release_gate.acceptance_rows.first() {
        rows.push(metric_row("Gate Row", row.clone()));
    }

    rows
}
