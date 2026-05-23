use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_agent_bridge::DxAgentReceipt;

use super::super::super::metric_row;
use super::labels::{
    receipt_action_label, receipt_automation_label, receipt_detail_label,
    receipt_provider_model_label, receipt_social_label, receipt_state_label,
};

pub(super) fn dx_agent_receipt_row(
    id: SharedString,
    receipt: &DxAgentReceipt,
    cx: &App,
) -> AnyElement {
    let state = receipt_state_label(receipt);
    let detail = receipt_detail_label(receipt);
    let provider_model = receipt_provider_model_label(receipt);
    let actions = receipt_action_label(receipt);
    let social_status = receipt_social_label(receipt);
    let automation_status = receipt_automation_label(receipt);

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(receipt.id.clone(), state))
        .child(
            Label::new(detail)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .when(!receipt.task_state.is_empty(), |this| {
            this.child(
                Label::new(format!("Task: {}", receipt.task_state))
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(receipt.duration_state.as_ref(), |this, duration| {
            this.child(
                Label::new(format!("Duration: {duration}"))
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(provider_model, |this, provider_model| {
            this.child(
                Label::new(provider_model)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(actions, |this, actions| {
            this.child(
                Label::new(actions)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(social_status, |this, social_status| {
            this.child(
                Label::new(social_status)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(automation_status, |this, automation_status| {
            this.child(
                Label::new(automation_status)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when(!receipt.schema_version.is_empty(), |this| {
            this.child(
                Label::new(receipt.schema_version.clone())
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(receipt.last_error.as_ref(), |this, error| {
            this.child(
                Label::new(format!("Error: {error}"))
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when(
            receipt.last_error.is_none() && !receipt.next_action.is_empty(),
            |this| {
                this.child(
                    Label::new(receipt.next_action.clone())
                        .size(LabelSize::XSmall)
                        .color(Color::Muted)
                        .truncate(),
                )
            },
        )
        .into_any_element()
}
