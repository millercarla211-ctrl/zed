use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_www_launch_evidence::DxWwwLaunchEvidenceSnapshot;

use super::{bounded_items, metric_row, muted_card, signal_row};

pub(super) fn www_launch_evidence_state(
    snapshot: &DxWwwLaunchEvidenceSnapshot,
    cx: &App,
) -> AnyElement {
    let latest = bounded_items(&snapshot.latest_rows, 3, "No release evidence files");
    let missing = bounded_items(&snapshot.missing_rows, 3, "No missing release evidence");
    let next_commands = bounded_items(&snapshot.next_commands, 3, "No next command");

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
            "Project",
            snapshot.project_root.display().to_string(),
        ))
        .child(metric_row(
            "Release Root",
            if snapshot.release_root_exists {
                snapshot.release_root.display().to_string()
            } else {
                "missing".to_string()
            },
        ))
        .child(metric_row(
            "Artifacts",
            format!(
                "{} / {} present",
                snapshot.present_count, snapshot.expected_count
            ),
        ))
        .child(metric_row(
            "Formats",
            format!(
                "{} json / {} markdown",
                snapshot.json_count, snapshot.markdown_count
            ),
        ))
        .child(metric_row(
            "Results",
            format!(
                "{} ready / {} warning / {} blocked",
                snapshot.passed_count, snapshot.warning_count, snapshot.blocked_count
            ),
        ))
        .child(metric_row(
            "No Execution",
            format!("{} artifact(s)", snapshot.no_execution_count),
        ))
        .child(metric_row("Latest", latest))
        .child(metric_row("Missing", missing))
        .child(metric_row("Next", next_commands));

    if !snapshot.project_root_exists {
        stack = stack.child(muted_card(
            format!(
                "Missing DX-WWW project: {}",
                snapshot.project_root.display()
            ),
            cx,
        ));
    } else if !snapshot.release_root_exists {
        stack = stack.child(muted_card(
            format!(
                "No release evidence root yet: {}",
                snapshot.release_root.display()
            ),
            cx,
        ));
    }

    if let Some(issue) = snapshot.first_issue.as_ref() {
        stack = stack.child(signal_row(
            "dx-www-evidence-warning".into(),
            IconName::Warning,
            Color::Warning,
            issue.clone(),
        ));
    } else if snapshot.present_count < snapshot.expected_count {
        stack = stack.child(signal_row(
            "dx-www-evidence-partial".into(),
            IconName::Warning,
            Color::Warning,
            "DX-WWW release evidence is partial; keep runtime-green claims gated.".to_string(),
        ));
    }

    stack.into_any_element()
}
