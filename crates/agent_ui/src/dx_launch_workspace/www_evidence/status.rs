use gpui::{AnyElement, App, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_www_launch_evidence::DxWwwLaunchEvidenceSnapshot;

use super::super::{bounded_items, metric_row, muted_card};

pub(super) fn www_launch_evidence_status_rows(
    snapshot: &DxWwwLaunchEvidenceSnapshot,
    cx: &App,
) -> Vec<AnyElement> {
    let latest = bounded_items(&snapshot.latest_rows, 3, "No release evidence files");
    let missing = bounded_items(&snapshot.missing_rows, 3, "No missing release evidence");
    let next_commands = bounded_items(&snapshot.next_commands, 3, "No next command");
    let release_root = if snapshot.release_root_exists {
        snapshot.release_root.display().to_string()
    } else {
        "missing".to_string()
    };

    let mut rows = vec![
        metric_row("Status", snapshot.status.clone()),
        Label::new(snapshot.operator_summary.clone())
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate()
            .into_any_element(),
        metric_row("Project", snapshot.project_root.display().to_string()),
        metric_row("Release Root", release_root),
        metric_row(
            "Artifacts",
            format!(
                "{} / {} present",
                snapshot.present_count, snapshot.expected_count
            ),
        ),
        metric_row(
            "Formats",
            format!(
                "{} json / {} markdown",
                snapshot.json_count, snapshot.markdown_count
            ),
        ),
        metric_row(
            "Results",
            format!(
                "{} ready / {} warning / {} blocked",
                snapshot.passed_count, snapshot.warning_count, snapshot.blocked_count
            ),
        ),
        metric_row(
            "No Execution",
            format!("{} artifact(s)", snapshot.no_execution_count),
        ),
        metric_row("Latest", latest),
        metric_row("Missing", missing),
        metric_row("Next", next_commands),
    ];

    if !snapshot.project_root_exists {
        rows.push(muted_card(
            format!(
                "Missing DX-WWW project: {}",
                snapshot.project_root.display()
            ),
            cx,
        ));
    } else if !snapshot.release_root_exists {
        rows.push(muted_card(
            format!(
                "No release evidence root yet: {}",
                snapshot.release_root.display()
            ),
            cx,
        ));
    }

    rows
}
