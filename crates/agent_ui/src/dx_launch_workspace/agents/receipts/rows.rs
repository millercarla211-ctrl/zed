use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_agent_bridge::DxAgentReceipt;

use super::super::super::list_labels::yes_no;
use super::super::super::metric_row;

pub(super) fn dx_agent_receipt_row(
    id: SharedString,
    receipt: &DxAgentReceipt,
    cx: &App,
) -> AnyElement {
    let state = if !receipt.safe_to_render {
        "Unsafe".to_string()
    } else if receipt.active_task {
        "Active".to_string()
    } else if receipt.metadata_redacted {
        format!("{} / redacted", receipt.status)
    } else {
        receipt.status.clone()
    };
    let detail = if receipt.command.is_empty() {
        format!("{} - {} bytes", receipt.kind, receipt.size_bytes)
    } else {
        format!(
            "{} - {} - {} bytes",
            receipt.kind, receipt.command, receipt.size_bytes
        )
    };
    let provider_model = match (
        receipt.provider_status.as_deref(),
        receipt.model_status.as_deref(),
    ) {
        (Some(provider), Some(model)) => Some(format!("Provider {provider}, model {model}")),
        (Some(provider), None) => Some(format!("Provider {provider}")),
        (None, Some(model)) => Some(format!("Model {model}")),
        (None, None) => None,
    };
    let actions = match (receipt.retry_supported, receipt.cancel_supported) {
        (Some(retry), Some(cancel)) => Some(format!(
            "Retry {}, cancel {}",
            yes_no(retry),
            yes_no(cancel)
        )),
        (Some(retry), None) => Some(format!("Retry {}", yes_no(retry))),
        (None, Some(cancel)) => Some(format!("Cancel {}", yes_no(cancel))),
        (None, None) => None,
    };
    let social_status = match (receipt.social_connected, receipt.social_needs_auth) {
        (Some(connected), Some(needs_auth)) => Some(format!(
            "Social connected {connected}, needs auth {needs_auth}"
        )),
        (Some(connected), None) => Some(format!("Social connected {connected}")),
        (None, Some(needs_auth)) => Some(format!("Social needs auth {needs_auth}")),
        (None, None) => None,
    };
    let automation_status = match (receipt.automation_enabled, receipt.automation_warning) {
        (Some(enabled), Some(warning)) => {
            Some(format!("Automations enabled {enabled}, warning {warning}"))
        }
        (Some(enabled), None) => Some(format!("Automations enabled {enabled}")),
        (None, Some(warning)) => Some(format!("Automation warnings {warning}")),
        (None, None) => None,
    };

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
