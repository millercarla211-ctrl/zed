use gpui::{AnyElement, App, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_runtime_proof_status::{DxRuntimeProofPlanSummary, DxRuntimeProofReceiptSummary};

use super::super::metric_row;
use super::super::proof_labels::{
    runtime_proof_evidence_detail, runtime_proof_receipt_state_label,
    runtime_proof_requirements_label,
};

pub(super) fn runtime_proof_plan_row(plan: &DxRuntimeProofPlanSummary, cx: &App) -> AnyElement {
    let requirements = runtime_proof_requirements_label(
        plan.requires_clean_git,
        plan.requires_diff_check,
        plan.requires_visual_evidence,
        plan.requires_import,
    );
    let mut stack = v_flex()
        .id("dx-runtime-proof-latest-plan")
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(
            "Plan",
            format!("{} - {} step(s)", plan.status, plan.checklist_step_count),
        ))
        .child(
            Label::new(format!(
                "{} required - {}",
                plan.required_step_count, requirements
            ))
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        )
        .child(
            Label::new(plan.label.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );

    if let Some(command) = plan.expected_final_command.as_ref() {
        stack = stack.child(
            Label::new(format!("Command {command}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if plan.minimum_evidence_lines_for_pass > 0 || !plan.accepted_evidence_examples.is_empty() {
        stack = stack.child(
            Label::new(runtime_proof_evidence_detail(
                plan.minimum_evidence_lines_for_pass,
                &plan.accepted_evidence_examples,
            ))
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        );
    }

    if plan.blocker_count > 0 {
        stack = stack.child(
            Label::new(format!("{} blocker(s)", plan.blocker_count))
                .size(LabelSize::XSmall)
                .color(Color::Warning)
                .truncate(),
        );
    } else if let Some(next_action) = plan.next_action.as_ref() {
        stack = stack.child(
            Label::new(next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    stack.into_any_element()
}

pub(super) fn runtime_proof_receipt_row(
    id: &'static str,
    label: &'static str,
    receipt: &DxRuntimeProofReceiptSummary,
    cx: &App,
) -> AnyElement {
    let state = runtime_proof_receipt_state_label(
        receipt.runtime_green_candidate,
        receipt.can_claim_runtime_green,
        &receipt.validation_status,
        receipt.blocker_count,
    );
    let mut stack = v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(label, state))
        .child(
            Label::new(format!(
                "{} evidence - operator {}",
                receipt.evidence_count, receipt.operator_status
            ))
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        )
        .child(
            Label::new(receipt.label.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );

    if let Some(headline) = receipt.headline.as_ref() {
        stack = stack.child(
            Label::new(headline.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if let Some(summary) = receipt.proof_summary.as_ref() {
        stack = stack.child(
            Label::new(format!("Summary {summary}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if let Some(command) = receipt.final_command.as_ref() {
        stack = stack.child(
            Label::new(format!("Command {command}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if let Some(source) = receipt.source.as_ref() {
        stack = stack.child(
            Label::new(format!("Source {source}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if let Some(evidence) = receipt.evidence_samples.first() {
        stack = stack.child(
            Label::new(format!("Evidence {evidence}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    stack.into_any_element()
}
