use gpui::{AnyElement, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_runtime_proof_status::DxRuntimeProofPlanSummary;

use super::super::super::proof_labels::runtime_proof_evidence_detail;

pub(super) fn runtime_proof_plan_detail_rows(plan: &DxRuntimeProofPlanSummary) -> Vec<AnyElement> {
    let mut rows = Vec::new();

    if let Some(command) = plan.expected_final_command.as_ref() {
        rows.push(detail_label(format!("Command {command}"), Color::Muted));
    }

    if plan.minimum_evidence_lines_for_pass > 0 || !plan.accepted_evidence_examples.is_empty() {
        rows.push(detail_label(
            runtime_proof_evidence_detail(
                plan.minimum_evidence_lines_for_pass,
                &plan.accepted_evidence_examples,
            ),
            Color::Muted,
        ));
    }

    if plan.blocker_count > 0 {
        rows.push(detail_label(
            format!("{} blocker(s)", plan.blocker_count),
            Color::Warning,
        ));
    } else if let Some(next_action) = plan.next_action.as_ref() {
        rows.push(detail_label(next_action.clone(), Color::Muted));
    }

    rows
}

fn detail_label(text: String, color: Color) -> AnyElement {
    Label::new(text)
        .size(LabelSize::XSmall)
        .color(color)
        .truncate()
        .into_any_element()
}
