use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_agent_bridge::{DxAgentBridgeSnapshot, DxAgentModel, DxAgentProvider};

use super::super::{metric_row, muted_card};

pub(in super::super) fn dx_agent_provider_state(
    snapshot: &DxAgentBridgeSnapshot,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row(
            "Providers",
            snapshot.providers.len().to_string(),
        ))
        .child(metric_row("Models", snapshot.models.len().to_string()))
        .child(metric_row(
            "Catalog path",
            snapshot.catalog.path.display().to_string(),
        ))
        .child(metric_row(
            "Fast cache",
            if snapshot.catalog.present && !snapshot.catalog.stale {
                "ready"
            } else {
                "stale/missing"
            },
        ));

    if !snapshot.show_managed_providers {
        return stack
            .child(muted_card("Managed provider rows hidden by settings", cx))
            .into_any_element();
    }

    if let Some(source_hash) = snapshot.catalog.source_hash.as_ref() {
        stack = stack.child(metric_row("Source hash", source_hash.clone()));
    }

    if snapshot.providers.is_empty() {
        stack = stack.child(muted_card(
            format!("Run {}", snapshot.catalog.safe_regeneration_command),
            cx,
        ));
    } else {
        for (ix, provider) in snapshot.providers.iter().take(3).enumerate() {
            stack = stack.child(dx_agent_provider_row(
                SharedString::from(format!("dx-agent-provider-{ix}")),
                provider,
                cx,
            ));
        }
    }

    for (ix, model) in snapshot.models.iter().take(2).enumerate() {
        stack = stack.child(dx_agent_model_row(
            SharedString::from(format!("dx-agent-model-{ix}")),
            model,
            cx,
        ));
    }

    stack.into_any_element()
}

fn dx_agent_provider_row(id: SharedString, provider: &DxAgentProvider, cx: &App) -> AnyElement {
    let state = if provider.active {
        "Active".to_string()
    } else if provider.configured {
        "Configured".to_string()
    } else if provider.local {
        "Local".to_string()
    } else {
        provider.status.clone()
    };
    let compatibility = provider.compatibility.join(", ");

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(provider.display_name.clone(), state))
        .child(
            Label::new(if compatibility.is_empty() {
                provider.id.clone()
            } else {
                format!("{} - {}", provider.id, compatibility)
            })
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        )
        .into_any_element()
}

fn dx_agent_model_row(id: SharedString, model: &DxAgentModel, cx: &App) -> AnyElement {
    let state = if model.active {
        "Active".to_string()
    } else {
        model.status.clone()
    };
    let compatibility = model.compatibility.join(", ");

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().editor_background)
        .child(metric_row(model.model_id.clone(), state))
        .child(
            Label::new(if compatibility.is_empty() {
                format!("{} / {}", model.provider_id, model.id)
            } else {
                format!("{} / {} - {}", model.provider_id, model.id, compatibility)
            })
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        )
        .into_any_element()
}
