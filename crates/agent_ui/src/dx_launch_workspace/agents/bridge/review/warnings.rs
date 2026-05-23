use gpui::{AnyElement, SharedString};
use ui::{Color, IconName};

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::super::super::super::signal_row;

pub(super) fn dx_agent_bridge_warning_row(snapshot: &DxAgentBridgeSnapshot) -> Option<AnyElement> {
    if snapshot.action_error.redaction_requires_review {
        Some(warning_row(
            "dx-agent-action-error-redaction",
            snapshot.action_error.redaction_summary.clone(),
        ))
    } else if let Some(reason) = snapshot.release_gate.blocking_reasons.first() {
        Some(warning_row(
            "dx-agent-release-gate-blocker",
            format!("DX Agents release gate blocker: {reason}"),
        ))
    } else if snapshot.release_gate.present && !snapshot.release_gate.no_command_fanout {
        Some(warning_row(
            "dx-agent-release-gate-fanout-review",
            "DX Agents release gate reports command fanout; keep bridge import blocked."
                .to_string(),
        ))
    } else if let Some(reason) = snapshot.release_gate.warning_reasons.first() {
        Some(warning_row(
            "dx-agent-release-gate-warning",
            format!("DX Agents release gate warning: {reason}"),
        ))
    } else if let Some(reason) = snapshot.import_summary.blocking_reasons.first() {
        Some(warning_row(
            "dx-agent-import-summary-blocker",
            format!("DX Agents import summary blocker: {reason}"),
        ))
    } else if snapshot.import_summary.present && !snapshot.import_summary.no_command_fanout {
        Some(warning_row(
            "dx-agent-import-summary-fanout-review",
            "DX Agents import summary reports command fanout; keep recovery controls disabled."
                .to_string(),
        ))
    } else if let Some(reason) = snapshot.import_summary.warning_reasons.first() {
        Some(warning_row(
            "dx-agent-import-summary-warning",
            format!("DX Agents import summary warning: {reason}"),
        ))
    } else if snapshot.contract_summary.redaction_requires_review {
        Some(warning_row(
            "dx-agent-contract-redaction-review",
            "DX Agents bridge contract reports redaction flags that need review.".to_string(),
        ))
    } else if let Some(error) = snapshot.last_error.as_ref() {
        Some(warning_row("dx-agent-bridge-error", error.clone()))
    } else {
        None
    }
}

fn warning_row(id: &'static str, label: impl Into<SharedString>) -> AnyElement {
    signal_row(id.into(), IconName::Warning, Color::Warning, label)
}
