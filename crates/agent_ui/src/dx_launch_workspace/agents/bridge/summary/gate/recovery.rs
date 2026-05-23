use gpui::AnyElement;

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::super::super::super::super::metric_row;

pub(super) fn dx_agent_bridge_gate_recovery_rows(
    snapshot: &DxAgentBridgeSnapshot,
) -> Vec<AnyElement> {
    vec![
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
    ]
}
