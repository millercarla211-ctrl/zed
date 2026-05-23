use gpui::{AnyElement, App, prelude::*};
use ui::{IconName, prelude::*};

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::{metric_row, muted_card, signal_row};

mod actions;
mod automations;
mod provider_labels;
mod providers;
mod receipts;
mod social;
mod social_actions;
pub(super) use automations::dx_agent_automation_state;
pub(super) use providers::dx_agent_provider_state;
pub(super) use receipts::dx_agent_receipt_state;
pub(super) use social::dx_agent_social_state;

pub(super) fn dx_agent_bridge_state(snapshot: &DxAgentBridgeSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Bridge", snapshot.status.clone()))
        .child(metric_row("CLI", snapshot.cli_path.clone()))
        .child(metric_row(
            "Accounts",
            format!(
                "{} connected / {} configured",
                snapshot.connected_accounts_summary.connected,
                snapshot.connected_accounts_summary.configured
            ),
        ))
        .child(metric_row(
            "Automations",
            snapshot.automation_count.to_string(),
        ))
        .child(metric_row("Tasks", snapshot.active_task_count.to_string()))
        .child(metric_row(
            "Catalog",
            if snapshot.catalog.present && !snapshot.catalog.stale {
                "fast".to_string()
            } else {
                "fallback".to_string()
            },
        ))
        .child(metric_row(
            "Contract",
            snapshot.contract_summary.status.clone(),
        ))
        .child(metric_row(
            "Commands",
            snapshot.contract_summary.command_count.to_string(),
        ))
        .child(metric_row(
            "Receipts",
            snapshot.contract_summary.receipt_count.to_string(),
        ))
        .child(metric_row(
            "Contract Catalog",
            format!(
                "{} / {} receipt(s)",
                snapshot.contract_summary.provider_catalog_source,
                snapshot.contract_summary.provider_catalog_receipt_count
            ),
        ))
        .child(metric_row(
            "Redaction",
            if snapshot.contract_summary.redaction_requires_review {
                "review".to_string()
            } else {
                snapshot.contract_summary.redaction_summary.clone()
            },
        ))
        .child(metric_row("Import", snapshot.import_summary.status.clone()))
        .child(metric_row(
            "Release",
            format!(
                "{} / {} warning(s) / {} blocker(s)",
                snapshot.import_summary.release_gate_status,
                snapshot.import_summary.release_gate_warning_count,
                snapshot.import_summary.release_gate_failed_count
            ),
        ))
        .child(metric_row(
            "Action Map",
            format!(
                "{} / {}",
                snapshot.import_summary.action_map_status,
                snapshot.import_summary.recovery_counts.label()
            ),
        ))
        .child(metric_row(
            "Recovery",
            format!(
                "{} / {} fixture(s) / {}",
                snapshot.import_summary.recovery_controls_status,
                snapshot.import_summary.recovery_fixture_count,
                snapshot.import_summary.recovery_counts.label()
            ),
        ))
        .child(metric_row(
            "Command Fanout",
            if snapshot.import_summary.no_command_fanout {
                "none".to_string()
            } else {
                "review".to_string()
            },
        ))
        .child(metric_row(
            "Last Action",
            if snapshot.action_error.present {
                snapshot.action_error.status.clone()
            } else {
                "ready".to_string()
            },
        ))
        .child(metric_row("Gate", snapshot.release_gate.status.clone()))
        .child(metric_row(
            "Acceptance",
            format!(
                "{} / {} passed",
                snapshot.release_gate.passed_count, snapshot.release_gate.acceptance_count
            ),
        ))
        .child(metric_row(
            "Gate Warnings",
            format!(
                "{} warning(s) / {} blocker(s)",
                snapshot.release_gate.warning_count, snapshot.release_gate.failed_count
            ),
        ))
        .child(metric_row(
            "Gate Receipts",
            format!(
                "{} / {}",
                snapshot.release_gate.receipt_count, snapshot.release_gate.receipt_inbox_status
            ),
        ))
        .child(metric_row(
            "Gate Packets",
            format!(
                "{} packet(s) / {} fixture(s)",
                snapshot.release_gate.packet_count, snapshot.release_gate.fixture_family_count
            ),
        ))
        .child(metric_row(
            "Gate Retention",
            format!(
                "{} / {} overflow",
                snapshot.release_gate.retention_preview_status,
                snapshot.release_gate.retained_run_overflow_count
            ),
        ))
        .child(metric_row(
            "Gate Action",
            format!(
                "{} / {}",
                snapshot.release_gate.action_map_status,
                snapshot.release_gate.recovery_counts.label()
            ),
        ))
        .child(metric_row(
            "Gate Recovery",
            format!(
                "{} via {}, {} fixture(s), {}",
                snapshot.release_gate.recovery_controls_status,
                snapshot.release_gate.recovery_render_first,
                snapshot.release_gate.recovery_fixture_count,
                snapshot.release_gate.recovery_counts.label()
            ),
        ))
        .child(metric_row(
            "Gate Fanout",
            if snapshot.release_gate.no_command_fanout {
                "none".to_string()
            } else {
                "review".to_string()
            },
        ))
        .child(metric_row(
            "Receipt Index",
            snapshot.receipt_index.status.clone(),
        ))
        .child(metric_row(
            "Receipt Rows",
            format!(
                "{} / {}",
                snapshot.receipt_index.returned_receipt_count, snapshot.receipt_index.receipt_count
            ),
        ));

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

    if let Some(reason) = snapshot.release_gate.blocking_reasons.first() {
        stack = stack.child(signal_row(
            "dx-agent-release-gate-blocker".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents release gate blocker: {reason}"),
        ));
    } else if snapshot.release_gate.present && !snapshot.release_gate.no_command_fanout {
        stack = stack.child(signal_row(
            "dx-agent-release-gate-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents release gate reports command fanout; keep bridge import blocked."
                .to_string(),
        ));
    } else if let Some(reason) = snapshot.release_gate.warning_reasons.first() {
        stack = stack.child(signal_row(
            "dx-agent-release-gate-warning".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents release gate warning: {reason}"),
        ));
    } else if let Some(reason) = snapshot.import_summary.blocking_reasons.first() {
        stack = stack.child(signal_row(
            "dx-agent-import-summary-blocker".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents import summary blocker: {reason}"),
        ));
    } else if snapshot.import_summary.present && !snapshot.import_summary.no_command_fanout {
        stack = stack.child(signal_row(
            "dx-agent-import-summary-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents import summary reports command fanout; keep recovery controls disabled."
                .to_string(),
        ));
    } else if let Some(reason) = snapshot.import_summary.warning_reasons.first() {
        stack = stack.child(signal_row(
            "dx-agent-import-summary-warning".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents import summary warning: {reason}"),
        ));
    } else if snapshot.contract_summary.redaction_requires_review {
        stack = stack.child(signal_row(
            "dx-agent-contract-redaction-review".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents bridge contract reports redaction flags that need review.".to_string(),
        ));
    } else if let Some(error) = snapshot.last_error.as_ref() {
        stack = stack.child(signal_row(
            "dx-agent-bridge-error".into(),
            IconName::Warning,
            Color::Warning,
            error.clone(),
        ));
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
        stack = stack.child(
            Label::new(next_action)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    stack.into_any_element()
}
