use gpui::AnyElement;

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::super::super::super::metric_row;

pub(super) fn dx_agent_bridge_gate_rows(snapshot: &DxAgentBridgeSnapshot) -> Vec<AnyElement> {
    vec![
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
