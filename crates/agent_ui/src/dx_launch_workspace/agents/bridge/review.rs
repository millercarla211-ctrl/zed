use gpui::{AnyElement, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::super::super::signal_row;

pub(super) fn dx_agent_bridge_review_row(snapshot: &DxAgentBridgeSnapshot) -> AnyElement {
    if snapshot.action_error.redaction_requires_review {
        signal_row(
            "dx-agent-action-error-redaction".into(),
            IconName::Warning,
            Color::Warning,
            snapshot.action_error.redaction_summary.clone(),
        )
    } else if let Some(reason) = snapshot.release_gate.blocking_reasons.first() {
        signal_row(
            "dx-agent-release-gate-blocker".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents release gate blocker: {reason}"),
        )
    } else if snapshot.release_gate.present && !snapshot.release_gate.no_command_fanout {
        signal_row(
            "dx-agent-release-gate-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents release gate reports command fanout; keep bridge import blocked."
                .to_string(),
        )
    } else if let Some(reason) = snapshot.release_gate.warning_reasons.first() {
        signal_row(
            "dx-agent-release-gate-warning".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents release gate warning: {reason}"),
        )
    } else if let Some(reason) = snapshot.import_summary.blocking_reasons.first() {
        signal_row(
            "dx-agent-import-summary-blocker".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents import summary blocker: {reason}"),
        )
    } else if snapshot.import_summary.present && !snapshot.import_summary.no_command_fanout {
        signal_row(
            "dx-agent-import-summary-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents import summary reports command fanout; keep recovery controls disabled."
                .to_string(),
        )
    } else if let Some(reason) = snapshot.import_summary.warning_reasons.first() {
        signal_row(
            "dx-agent-import-summary-warning".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents import summary warning: {reason}"),
        )
    } else if snapshot.contract_summary.redaction_requires_review {
        signal_row(
            "dx-agent-contract-redaction-review".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents bridge contract reports redaction flags that need review.".to_string(),
        )
    } else if let Some(error) = snapshot.last_error.as_ref() {
        signal_row(
            "dx-agent-bridge-error".into(),
            IconName::Warning,
            Color::Warning,
            error.clone(),
        )
    } else {
        let next_action = if snapshot.release_gate.present {
            snapshot.release_gate.next_action.clone()
        } else if snapshot.import_summary.present {
            snapshot.import_summary.next_action.clone()
        } else if snapshot.contract_summary.present {
            snapshot.contract_summary.next_action.clone()
        } else {
            snapshot.next_action.clone()
        };
        Label::new(next_action)
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate()
            .into_any_element()
    }
}
