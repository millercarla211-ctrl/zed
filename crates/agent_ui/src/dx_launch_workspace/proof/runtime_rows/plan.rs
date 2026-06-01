use gpui::{AnyElement, App, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_runtime_proof_status::DxRuntimeProofPlanSummary;

use super::super::super::metric_row;
use super::super::super::proof_labels::runtime_proof_requirements_label;
use super::plan_details::runtime_proof_plan_detail_rows;

pub(in super::super) fn runtime_proof_plan_row(
    plan: &DxRuntimeProofPlanSummary,
    cx: &App,
) -> AnyElement {
    let requirements = runtime_proof_requirements_label(
        plan.requires_clean_git,
        plan.requires_diff_check,
        plan.requires_visual_evidence,
        plan.requires_import,
    );
    let stack = v_flex()
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
        )
        .children(runtime_proof_plan_detail_rows(plan));

    stack.into_any_element()
}
