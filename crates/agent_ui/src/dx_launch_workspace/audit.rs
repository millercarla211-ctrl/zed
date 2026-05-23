use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_launch_audit::DxLaunchAuditSnapshot;

use self::warnings::launch_audit_warning;
use super::{bounded_items, metric_row, muted_card, signal_row};

mod warnings;

pub(super) fn launch_audit_state(snapshot: &DxLaunchAuditSnapshot, cx: &App) -> AnyElement {
    let command_rows = bounded_items(&snapshot.command_rows, 3, "No command rows");
    let fixture_rows = bounded_items(&snapshot.fixture_rows, 2, "No fixture rows");
    let smoke_rows = bounded_items(&snapshot.smoke_rows, 2, "No smoke rows");

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Status", snapshot.status.clone()))
        .child(
            Label::new(snapshot.operator_summary.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row(
            "Commands",
            format!(
                "{} total, {} startup, {} user-action",
                snapshot.command_count, snapshot.startup_poll_count, snapshot.user_action_count
            ),
        ))
        .child(metric_row(
            "Safety",
            format!(
                "{} metadata-only, {} writes, {} fanout",
                snapshot.metadata_only_count,
                snapshot.write_path_count,
                snapshot.command_fanout_count
            ),
        ))
        .child(metric_row(
            "Fixtures",
            format!(
                "{} total, {} matched",
                snapshot.fixture_count, snapshot.fixture_match_count
            ),
        ))
        .child(metric_row(
            "Smoke",
            format!(
                "{} passed / {} warning / {} failed of {}",
                snapshot.smoke_passed_count,
                snapshot.smoke_warning_count,
                snapshot.smoke_failed_count,
                snapshot.smoke_check_count
            ),
        ))
        .child(metric_row("Example", snapshot.example_status.clone()))
        .child(metric_row(
            "Example Agents",
            snapshot.example_agents.clone(),
        ))
        .child(metric_row(
            "Example Tokens",
            snapshot.example_tokens.clone(),
        ))
        .child(metric_row(
            "Example Discovery",
            snapshot.example_discovery.clone(),
        ))
        .child(metric_row("Commands", command_rows))
        .child(metric_row("Fixtures", fixture_rows))
        .child(metric_row("Smoke Rows", smoke_rows));

    if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!("Missing launch example root: {}", snapshot.root.display()),
            cx,
        ));
    }

    for (present, path, label) in [
        (snapshot.schemas_present, &snapshot.schemas_path, "schemas"),
        (
            snapshot.fixtures_present,
            &snapshot.fixtures_path,
            "fixtures",
        ),
        (snapshot.smoke_present, &snapshot.smoke_path, "smoke"),
        (snapshot.status_present, &snapshot.status_path, "status"),
    ] {
        if !present {
            stack = stack.child(muted_card(
                format!("Missing {label}: {}", path.display()),
                cx,
            ));
        }
    }

    if let Some((id, message)) = launch_audit_warning(snapshot) {
        stack = stack.child(signal_row(id, IconName::Warning, Color::Warning, message));
    } else {
        stack = stack.child(
            Label::new(snapshot.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    stack.into_any_element()
}
