use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_agent_bridge::{DxAgentModel, DxAgentProvider};

use super::super::super::metric_row;
use super::super::provider_labels::{
    model_detail_label, model_state_label, provider_detail_label, provider_state_label,
};

pub(super) fn dx_agent_provider_row(
    id: SharedString,
    provider: &DxAgentProvider,
    cx: &App,
) -> AnyElement {
    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(
            provider.display_name.clone(),
            provider_state_label(
                provider.active,
                provider.configured,
                provider.local,
                &provider.status,
            ),
        ))
        .child(
            Label::new(provider_detail_label(&provider.id, &provider.compatibility))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

pub(super) fn dx_agent_model_row(id: SharedString, model: &DxAgentModel, cx: &App) -> AnyElement {
    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().editor_background)
        .child(metric_row(
            model.model_id.clone(),
            model_state_label(model.active, &model.status),
        ))
        .child(
            Label::new(model_detail_label(
                &model.provider_id,
                &model.id,
                &model.compatibility,
            ))
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        )
        .into_any_element()
}
