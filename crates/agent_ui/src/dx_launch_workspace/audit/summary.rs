use gpui::{AnyElement, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_launch_audit::DxLaunchAuditSnapshot;

use super::super::{bounded_items, metric_row};

pub(super) fn launch_audit_summary_rows(snapshot: &DxLaunchAuditSnapshot) -> Vec<AnyElement> {
    let command_rows = bounded_items(&snapshot.command_rows, 3, "No command rows");
    let fixture_rows = bounded_items(&snapshot.fixture_rows, 2, "No fixture rows");
    let smoke_rows = bounded_items(&snapshot.smoke_rows, 2, "No smoke rows");

    vec![
        metric_row("Status", snapshot.status.clone()),
        Label::new(snapshot.operator_summary.clone())
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate()
            .into_any_element(),
        metric_row(
            "Commands",
            format!(
                "{} total, {} startup, {} user-action",
                snapshot.command_count, snapshot.startup_poll_count, snapshot.user_action_count
            ),
        ),
        metric_row(
            "Safety",
            format!(
                "{} metadata-only, {} writes, {} fanout",
                snapshot.metadata_only_count,
                snapshot.write_path_count,
                snapshot.command_fanout_count
            ),
        ),
        metric_row(
            "Fixtures",
            format!(
                "{} total, {} matched",
                snapshot.fixture_count, snapshot.fixture_match_count
            ),
        ),
        metric_row(
            "Smoke",
            format!(
                "{} passed / {} warning / {} failed of {}",
                snapshot.smoke_passed_count,
                snapshot.smoke_warning_count,
                snapshot.smoke_failed_count,
                snapshot.smoke_check_count
            ),
        ),
        metric_row("Example", snapshot.example_status.clone()),
        metric_row("Example Agents", snapshot.example_agents.clone()),
        metric_row("Example Tokens", snapshot.example_tokens.clone()),
        metric_row("Example Discovery", snapshot.example_discovery.clone()),
        metric_row("Commands", command_rows),
        metric_row("Fixtures", fixture_rows),
        metric_row("Smoke Rows", smoke_rows),
    ]
}
