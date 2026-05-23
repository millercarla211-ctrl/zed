use gpui::AnyElement;

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::super::super::super::metric_row;

pub(super) fn dx_agent_bridge_import_rows(snapshot: &DxAgentBridgeSnapshot) -> Vec<AnyElement> {
    vec![
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
    ]
}
