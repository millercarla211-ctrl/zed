use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_launch_source_audit::DxLaunchSourceAuditSnapshot;

use super::{bounded_items, metric_row, muted_card, signal_row, yes_no};

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

    if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!("Missing source audit root: {}", snapshot.root.display()),
            cx,
        ));
    } else if !snapshot.latest_present {
        stack = stack.child(muted_card(
            format!(
                "No source audit latest receipt at {}",
                snapshot.latest_path.display()
            ),
            cx,
        ));
    } else if !snapshot.schema_valid {
        stack = stack.child(signal_row(
            "dx-source-audit-invalid".into(),
            IconName::Warning,
            Color::Warning,
            snapshot
                .last_error
                .clone()
                .unwrap_or_else(|| "Source audit receipt schema is not valid.".to_string()),
        ));
    }

    if !snapshot.markdown_present {
        stack = stack.child(muted_card(
            format!(
                "Missing source audit markdown summary: {}",
                snapshot.markdown_path.display()
            ),
            cx,
        ));
    }

    if !snapshot.dx_studio_qa_present {
        stack = stack.child(muted_card(
            format!(
                "Missing DX Studio QA receipt: {}",
                snapshot.dx_studio_qa_path.display()
            ),
            cx,
        ));
    }

    if let Some(issue) = snapshot.first_issue.as_ref() {
        stack = stack.child(signal_row(
            "dx-source-audit-warning".into(),
            IconName::Warning,
            Color::Warning,
            issue.clone(),
        ));
    } else if snapshot.risk_review_count > 0 {
        stack = stack.child(signal_row(
            "dx-source-audit-risk".into(),
            IconName::Warning,
            Color::Warning,
            "Source audit is blocked by risk-review state in another launch repo.".to_string(),
        ));
    } else if !snapshot.template_trust_passed {
        stack = stack.child(signal_row(
            "dx-source-audit-template-trust".into(),
            IconName::Warning,
            Color::Warning,
            "Template trust scan is not passing.".to_string(),
        ));
    } else if !snapshot.dx_studio_passed {
        stack = stack.child(signal_row(
            "dx-source-audit-www-qa".into(),
            IconName::Warning,
            Color::Warning,
            "DX Studio WWW QA receipt is not passing.".to_string(),
        ));
    }

    stack.into_any_element()
}
