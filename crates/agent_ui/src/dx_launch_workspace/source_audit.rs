use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_launch_source_audit::DxLaunchSourceAuditSnapshot;

use self::status::launch_source_audit_status_rows;
use self::warnings::launch_source_audit_warning;
use super::{bounded_items, metric_row, signal_row, yes_no};

mod status;
mod warnings;

pub(super) fn launch_source_audit_state(
    snapshot: &DxLaunchSourceAuditSnapshot,
    cx: &App,
) -> AnyElement {
    let repo_rows = bounded_items(&snapshot.repo_rows, 3, "No repository rows");
    let blockers = bounded_items(&snapshot.blocker_rows, 3, "No source audit blockers");
    let deltas = bounded_items(&snapshot.delta_rows, 2, "No worker delta rows");

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Status", snapshot.status.clone()))
        .child(
            Label::new(snapshot.operator_summary.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row("Score", format!("{} / 100", snapshot.score)))
        .child(metric_row(
            "Coordination",
            format!(
                "{} / ready {}",
                if snapshot.passed {
                    "passed"
                } else {
                    "not passed"
                },
                yes_no(snapshot.ready_for_commit_coordination)
            ),
        ))
        .child(metric_row(
            "Repos",
            format!(
                "{} total, {} clean, {} active, {} risk",
                snapshot.repo_count,
                snapshot.source_clean_count,
                snapshot.active_output_count,
                snapshot.risk_review_count
            ),
        ))
        .child(metric_row(
            "Reviews",
            format!(
                "{} owner, {} diff failures",
                snapshot.owner_review_count, snapshot.diff_check_failure_count
            ),
        ))
        .child(metric_row(
            "DX Studio",
            format!(
                "{} / 100, checks {} / {}",
                snapshot.dx_studio_score,
                snapshot.dx_studio_passed_checks,
                snapshot.dx_studio_total_checks
            ),
        ))
        .child(metric_row(
            "Templates",
            format!(
                "{} / {} scanned, node_modules {}",
                snapshot.template_roots_scanned,
                snapshot.template_roots_total,
                snapshot.template_node_modules_found
            ),
        ))
        .child(metric_row("Rows", repo_rows))
        .child(metric_row("Delta", deltas))
        .child(metric_row("Blockers", blockers))
        .child(metric_row("Next", snapshot.next_target.clone()));

    stack = stack.children(launch_source_audit_status_rows(snapshot, cx));

    if let Some((id, message)) = launch_source_audit_warning(snapshot) {
        stack = stack.child(signal_row(id, IconName::Warning, Color::Warning, message));
    }

    stack.into_any_element()
}
