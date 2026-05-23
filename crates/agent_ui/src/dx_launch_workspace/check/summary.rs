use gpui::AnyElement;

use crate::dx_check_panel::DxCheckPanelSnapshot;

use super::super::{
    check_labels::{
        check_duration_label, check_outcome_label, checked_paths_label,
        last_run_label_with_generated_at, skipped_checks_label,
    },
    metric_row,
};

pub(super) fn check_summary_rows(panel: &DxCheckPanelSnapshot) -> Vec<AnyElement> {
    let last_run_label =
        last_run_label_with_generated_at(&panel.last_run_label, panel.generated_at_unix_ms);
    let config_state = if panel.scoring_config_applies_to_score {
        "applied"
    } else {
        "not applied"
    };

    let mut rows = vec![
        metric_row("Panel", panel.title.clone()),
        metric_row("Schema", panel.source_schema.clone()),
        metric_row("Receipt", panel.status.clone()),
        metric_row(
            "Outcome",
            check_outcome_label(
                panel.pass_count,
                panel.fail_count,
                panel.warn_count,
                panel.skipped_count,
            ),
        ),
        metric_row("Duration", check_duration_label(panel.duration_ms)),
        metric_row("Checked", checked_paths_label(&panel.checked_paths)),
        metric_row(
            "Skipped",
            skipped_checks_label(&panel.skipped_expensive_checks),
        ),
        metric_row("Score", panel.score_label()),
        metric_row("Profile", panel.weight_profile.clone()),
        metric_row(
            "Config",
            format!("{} / {config_state}", panel.scoring_config_status),
        ),
        metric_row("Scoring", panel.scoring_config_summary.clone()),
        metric_row("Last Run", last_run_label),
        metric_row("Refresh", panel.refresh_command.clone()),
    ];

    if let Some(detail_command) = panel.detail_command.as_ref() {
        rows.push(metric_row("Details", detail_command.clone()));
    }

    for (ix, checked_path) in panel.checked_paths.iter().take(2).enumerate() {
        rows.push(metric_row(format!("Path {}", ix + 1), checked_path.clone()));
    }

    for (ix, skipped) in panel.skipped_expensive_checks.iter().take(2).enumerate() {
        rows.push(metric_row(format!("Skip {}", ix + 1), skipped.clone()));
    }

    rows
}
