use gpui::AnyElement;

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::super::super::metric_row;

pub(super) fn dx_agent_bridge_summary_rows(snapshot: &DxAgentBridgeSnapshot) -> Vec<AnyElement> {
    vec![
        metric_row("Bridge", snapshot.status.clone()),
        metric_row("CLI", snapshot.cli_path.clone()),
        metric_row(
            "Accounts",
            format!(
                "{} connected / {} configured",
                snapshot.connected_accounts_summary.connected,
                snapshot.connected_accounts_summary.configured
            ),
        ),
        metric_row("Automations", snapshot.automation_count.to_string()),
        metric_row("Tasks", snapshot.active_task_count.to_string()),
        metric_row(
            "Catalog",
            if snapshot.catalog.present && !snapshot.catalog.stale {
                "fast".to_string()
            } else {
                "fallback".to_string()
            },
        ),
        metric_row("Contract", snapshot.contract_summary.status.clone()),
        metric_row(
            "Commands",
            snapshot.contract_summary.command_count.to_string(),
        ),
        metric_row(
            "Receipts",
            snapshot.contract_summary.receipt_count.to_string(),
        ),
        metric_row(
            "Contract Catalog",
            format!(
                "{} / {} receipt(s)",
                snapshot.contract_summary.provider_catalog_source,
                snapshot.contract_summary.provider_catalog_receipt_count
            ),
        ),
        metric_row(
            "Redaction",
            if snapshot.contract_summary.redaction_requires_review {
                "review".to_string()
            } else {
                snapshot.contract_summary.redaction_summary.clone()
            },
        ),
        metric_row("Import", snapshot.import_summary.status.clone()),
        metric_row(
            "Release",
            format!(
                "{} / {} warning(s) / {} blocker(s)",
                snapshot.import_summary.release_gate_status,
                snapshot.import_summary.release_gate_warning_count,
                snapshot.import_summary.release_gate_failed_count
            ),
        ),
        metric_row(
            "Action Map",
            format!(
                "{} / {}",
                snapshot.import_summary.action_map_status,
                snapshot.import_summary.recovery_counts.label()
            ),
        ),
        metric_row(
            "Recovery",
            format!(
                "{} / {} fixture(s) / {}",
                snapshot.import_summary.recovery_controls_status,
                snapshot.import_summary.recovery_fixture_count,
                snapshot.import_summary.recovery_counts.label()
            ),
        ),
        metric_row(
            "Command Fanout",
            if snapshot.import_summary.no_command_fanout {
                "none".to_string()
            } else {
                "review".to_string()
            },
        ),
        metric_row(
            "Last Action",
            if snapshot.action_error.present {
                snapshot.action_error.status.clone()
            } else {
                "ready".to_string()
            },
        ),
        metric_row("Gate", snapshot.release_gate.status.clone()),
        metric_row(
            "Acceptance",
            format!(
                "{} / {} passed",
                snapshot.release_gate.passed_count, snapshot.release_gate.acceptance_count
            ),
        ),
        metric_row(
            "Gate Warnings",
            format!(
                "{} warning(s) / {} blocker(s)",
                snapshot.release_gate.warning_count, snapshot.release_gate.failed_count
            ),
        ),
        metric_row(
            "Gate Receipts",
            format!(
                "{} / {}",
                snapshot.release_gate.receipt_count, snapshot.release_gate.receipt_inbox_status
            ),
        ),
        metric_row(
            "Gate Packets",
            format!(
                "{} packet(s) / {} fixture(s)",
                snapshot.release_gate.packet_count, snapshot.release_gate.fixture_family_count
            ),
        ),
        metric_row(
            "Gate Retention",
            format!(
                "{} / {} overflow",
                snapshot.release_gate.retention_preview_status,
                snapshot.release_gate.retained_run_overflow_count
            ),
        ),
        metric_row(
            "Gate Action",
            format!(
                "{} / {}",
                snapshot.release_gate.action_map_status,
                snapshot.release_gate.recovery_counts.label()
            ),
        ),
        metric_row(
            "Gate Recovery",
            format!(
                "{} via {}, {} fixture(s), {}",
                snapshot.release_gate.recovery_controls_status,
                snapshot.release_gate.recovery_render_first,
                snapshot.release_gate.recovery_fixture_count,
                snapshot.release_gate.recovery_counts.label()
            ),
        ),
        metric_row(
            "Gate Fanout",
            if snapshot.release_gate.no_command_fanout {
                "none".to_string()
            } else {
                "review".to_string()
            },
        ),
        metric_row("Receipt Index", snapshot.receipt_index.status.clone()),
        metric_row(
            "Receipt Rows",
            format!(
                "{} / {}",
                snapshot.receipt_index.returned_receipt_count, snapshot.receipt_index.receipt_count
            ),
        ),
    ]
}
