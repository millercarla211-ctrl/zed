use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::prelude::*;

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use self::rows::{dx_agent_model_row, dx_agent_provider_row};
use super::super::{metric_row, muted_card};

mod rows;

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
