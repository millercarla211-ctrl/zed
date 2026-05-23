use gpui::{AnyElement, App, prelude::*};

use crate::dx_launch_readiness::DxLaunchReadinessSnapshot;

use self::{examples::launch_readiness_example_rows, status::launch_readiness_status_row};
use super::{bounded_items, metric_row};

mod examples;
mod status;
mod warnings;

pub(super) fn launch_readiness_state(snapshot: &DxLaunchReadinessSnapshot, cx: &App) -> AnyElement {
    let freshness = bounded_items(&snapshot.freshness_states, 4, "No cached states");
    let fallback_states = bounded_items(&snapshot.fallback_states, 4, "No fallback states");
    let recovery = bounded_items(&snapshot.recovery_commands, 3, "No recovery commands");

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
            "Import",
            format!(
                "{} packet(s), {}",
                snapshot.import_summary_count,
                snapshot.import_status_counts.summary()
            ),
        ))
        .child(metric_row(
            "Release Gate",
            format!(
                "{} packet(s), {}",
                snapshot.release_gate_count,
                snapshot.release_gate_status_counts.summary()
            ),
        ))
        .child(metric_row(
            "Fallback",
            format!(
                "{} packet(s), {}",
                snapshot.fallback_drill_count,
                snapshot.fallback_status_counts.summary()
            ),
        ))
        .child(metric_row(
            "Gate Rows",
            format!(
                "{} passed / {} warning / {} failed of {}",
                snapshot.passed_count,
                snapshot.warning_count,
                snapshot.failed_count,
                snapshot.acceptance_count
            ),
        ))
        .child(metric_row("Freshness", freshness))
        .child(metric_row(
            "Fallback States",
            format!(
                "{} state(s): {}",
                snapshot.fallback_state_count, fallback_states
            ),
        ))
        .child(metric_row(
            "Fanout",
            if snapshot.no_command_fanout {
                "none".to_string()
            } else {
                format!("{} row(s)", snapshot.command_fanout_count)
            },
        ))
        .child(metric_row("Recovery", recovery));

    stack = stack.child(launch_readiness_status_row(snapshot, cx));

    for row in launch_readiness_example_rows(snapshot) {
        stack = stack.child(row);
    }

    stack.into_any_element()
}
