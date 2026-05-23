use gpui::{AnyElement, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_launch_readiness::DxLaunchReadinessSnapshot;

use super::super::{bounded_items, metric_row};

pub(super) fn launch_readiness_summary_rows(
    snapshot: &DxLaunchReadinessSnapshot,
) -> Vec<AnyElement> {
    let freshness = bounded_items(&snapshot.freshness_states, 4, "No cached states");
    let fallback_states = bounded_items(&snapshot.fallback_states, 4, "No fallback states");
    let recovery = bounded_items(&snapshot.recovery_commands, 3, "No recovery commands");

    vec![
        metric_row("Status", snapshot.status.clone()),
        Label::new(snapshot.operator_summary.clone())
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate()
            .into_any_element(),
        metric_row(
            "Import",
            format!(
                "{} packet(s), {}",
                snapshot.import_summary_count,
                snapshot.import_status_counts.summary()
            ),
        ),
        metric_row(
            "Release Gate",
            format!(
                "{} packet(s), {}",
                snapshot.release_gate_count,
                snapshot.release_gate_status_counts.summary()
            ),
        ),
        metric_row(
            "Fallback",
            format!(
                "{} packet(s), {}",
                snapshot.fallback_drill_count,
                snapshot.fallback_status_counts.summary()
            ),
        ),
        metric_row(
            "Gate Rows",
            format!(
                "{} passed / {} warning / {} failed of {}",
                snapshot.passed_count,
                snapshot.warning_count,
                snapshot.failed_count,
                snapshot.acceptance_count
            ),
        ),
        metric_row("Freshness", freshness),
        metric_row(
            "Fallback States",
            format!(
                "{} state(s): {}",
                snapshot.fallback_state_count, fallback_states
            ),
        ),
        metric_row(
            "Fanout",
            if snapshot.no_command_fanout {
                "none".to_string()
            } else {
                format!("{} row(s)", snapshot.command_fanout_count)
            },
        ),
        metric_row("Recovery", recovery),
    ]
}
