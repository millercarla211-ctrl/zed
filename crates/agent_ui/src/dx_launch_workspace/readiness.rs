use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_launch_readiness::DxLaunchReadinessSnapshot;

use super::{bounded_items, metric_row, muted_card, signal_row};

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

    if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!(
                "Missing source-owned launch examples: {}",
                snapshot.root.display()
            ),
            cx,
        ));
    }

    if let Some(issue) = snapshot.first_issue.as_ref() {
        stack = stack.child(signal_row(
            "dx-launch-readiness-warning".into(),
            IconName::Warning,
            Color::Warning,
            issue.clone(),
        ));
    } else if snapshot.redaction_requires_review {
        stack = stack.child(signal_row(
            "dx-launch-readiness-redaction-review".into(),
            IconName::Warning,
            Color::Warning,
            "Launch readiness redaction flags need review.".to_string(),
        ));
    } else if !snapshot.no_command_fanout {
        stack = stack.child(signal_row(
            "dx-launch-readiness-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "Launch readiness packets report command fanout; keep import blocked.".to_string(),
        ));
    } else {
        stack = stack.child(
            Label::new(snapshot.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    for (ix, example) in snapshot.examples.iter().take(3).enumerate() {
        stack = stack.child(metric_row(
            format!("Example {}", ix + 1),
            format!("{}: {} ({})", example.label, example.status, example.detail),
        ));

        if let Some(next_action) = example.next_action.as_ref() {
            stack = stack.child(metric_row(format!("Next {}", ix + 1), next_action.clone()));
        }
    }

    stack.into_any_element()
}
