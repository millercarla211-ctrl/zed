use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};

use self::rows::{check_blocker_row, check_quick_fix_rows, check_section_row, check_warning_rows};
use crate::dx_check_score::DxCheckScoreSnapshot;

use super::check_labels::{
    check_duration_label, check_outcome_label, checked_paths_label,
    last_run_label_with_generated_at, skipped_checks_label,
};
use super::{metric_row, muted_card, signal_row, source_row};

mod rows;

pub(super) fn check_score_state(snapshot: &DxCheckScoreSnapshot, cx: &App) -> AnyElement {
    let panel = &snapshot.panel;
    let last_run_label =
        last_run_label_with_generated_at(&panel.last_run_label, panel.generated_at_unix_ms);
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Panel", panel.title.clone()))
        .child(metric_row("Schema", panel.source_schema.clone()))
        .child(metric_row("Receipt", panel.status.clone()))
        .child(metric_row(
            "Outcome",
            check_outcome_label(
                panel.pass_count,
                panel.fail_count,
                panel.warn_count,
                panel.skipped_count,
            ),
        ))
        .child(metric_row(
            "Duration",
            check_duration_label(panel.duration_ms),
        ))
        .child(metric_row(
            "Checked",
            checked_paths_label(&panel.checked_paths),
        ))
        .child(metric_row(
            "Skipped",
            skipped_checks_label(&panel.skipped_expensive_checks),
        ))
        .child(metric_row("Score", panel.score_label()))
        .child(metric_row("Profile", panel.weight_profile.clone()))
        .child(metric_row(
            "Config",
            format!(
                "{} / {}",
                panel.scoring_config_status,
                if panel.scoring_config_applies_to_score {
                    "applied"
                } else {
                    "not applied"
                }
            ),
        ))
        .child(metric_row("Scoring", panel.scoring_config_summary.clone()))
        .child(metric_row("Last Run", last_run_label))
        .child(metric_row("Refresh", panel.refresh_command.clone()));

    if let Some(detail_command) = panel.detail_command.as_ref() {
        stack = stack.child(metric_row("Details", detail_command.clone()));
    }

    for (ix, checked_path) in panel.checked_paths.iter().take(2).enumerate() {
        stack = stack.child(metric_row(format!("Path {}", ix + 1), checked_path.clone()));
    }

    for (ix, skipped) in panel.skipped_expensive_checks.iter().take(2).enumerate() {
        stack = stack.child(metric_row(format!("Skip {}", ix + 1), skipped.clone()));
    }

    if !panel.receipt_present {
        stack = stack.child(muted_card(
            format!("Missing receipt: {}", panel.receipt_path.display()),
            cx,
        ));
    } else if let Some(error) = panel.receipt_error.as_ref() {
        stack = stack.child(signal_row(
            "dx-check-panel-error".into(),
            IconName::Warning,
            Color::Warning,
            error.clone(),
        ));
    }

    for section in panel.sections.iter().take(5) {
        stack = stack.child(check_section_row(section));
    }

    for (ix, blocker) in panel.blockers.iter().take(2).enumerate() {
        stack = stack.child(check_blocker_row(ix, blocker));
    }

    for (ix, warning) in panel.warnings.iter().take(2).enumerate() {
        stack = stack.children(check_warning_rows(ix, warning));
    }

    for (ix, fix) in panel.quick_fixes.iter().take(2).enumerate() {
        stack = stack.children(check_quick_fix_rows(ix, fix));
    }

    stack = stack
        .child(metric_row("Next", panel.next_action.clone()))
        .child(metric_row(
            "Rail score",
            format!("{}/100 {}", snapshot.score, snapshot.state),
        ));

    for item in snapshot.items.iter().take(4) {
        stack = stack.child(metric_row(item.label, item.state.clone()));
    }

    for (ix, blocker) in snapshot.blockers.iter().take(1).enumerate() {
        stack = stack.child(source_row(
            SharedString::from(format!("dx-check-blocker-{ix}")),
            IconName::ListTodo,
            blocker.clone(),
            cx,
        ));
    }

    stack.into_any_element()
}
