use gpui::AnyElement;

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::super::super::super::metric_row;

pub(super) fn dx_agent_bridge_overview_rows(snapshot: &DxAgentBridgeSnapshot) -> Vec<AnyElement> {
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
    ]
}
