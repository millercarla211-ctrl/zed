use gpui::{AnyElement, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_launch_source_audit::DxLaunchSourceAuditSnapshot;

use super::super::{bounded_items, metric_row, yes_no};

pub(super) fn launch_source_audit_summary_rows(
    snapshot: &DxLaunchSourceAuditSnapshot,
) -> Vec<AnyElement> {
    let repo_rows = bounded_items(&snapshot.repo_rows, 3, "No repository rows");
    let blockers = bounded_items(&snapshot.blocker_rows, 3, "No source audit blockers");
    let deltas = bounded_items(&snapshot.delta_rows, 2, "No worker delta rows");

    vec![
        metric_row("Status", snapshot.status.clone()),
        Label::new(snapshot.operator_summary.clone())
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate()
            .into_any_element(),
        metric_row("Score", format!("{} / 100", snapshot.score)),
        metric_row(
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
        ),
        metric_row(
            "Repos",
            format!(
                "{} total, {} clean, {} active, {} risk",
                snapshot.repo_count,
                snapshot.source_clean_count,
                snapshot.active_output_count,
                snapshot.risk_review_count
            ),
        ),
        metric_row(
            "Reviews",
            format!(
                "{} owner, {} diff failures",
                snapshot.owner_review_count, snapshot.diff_check_failure_count
            ),
        ),
        metric_row(
            "DX Studio",
            format!(
                "{} / 100, checks {} / {}",
                snapshot.dx_studio_score,
                snapshot.dx_studio_passed_checks,
                snapshot.dx_studio_total_checks
            ),
        ),
        metric_row(
            "Templates",
            format!(
                "{} / {} scanned, node_modules {}",
                snapshot.template_roots_scanned,
                snapshot.template_roots_total,
                snapshot.template_node_modules_found
            ),
        ),
        metric_row("Rows", repo_rows),
        metric_row("Delta", deltas),
        metric_row("Blockers", blockers),
        metric_row("Next", snapshot.next_target.clone()),
    ]
}
