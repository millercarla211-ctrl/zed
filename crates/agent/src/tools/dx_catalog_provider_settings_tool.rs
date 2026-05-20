use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_catalog_agent_bridge};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, SharedString, Task};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Inspect the DX catalog provider-settings registration preview.
///
/// Returns a read-only JSON report for catalog artifact loading, provider eligibility,
/// approval flags, dry-run state, credential/runtime blockers, and compact model previews.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxCatalogProviderSettingsToolInput {
    /// Include per-provider registration details and compact model previews.
    pub include_providers: bool,
}

impl Default for DxCatalogProviderSettingsToolInput {
    fn default() -> Self {
        Self {
            include_providers: true,
        }
    }
}

pub struct DxCatalogProviderSettingsTool;

impl AgentTool for DxCatalogProviderSettingsTool {
    type Input = DxCatalogProviderSettingsToolInput;
    type Output = String;

    const NAME: &'static str = "inspect_dx_catalog_provider_settings";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Inspect DX catalog provider settings".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |_cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let mut preview = dx_catalog_agent_bridge::provider_settings_registration_preview();

            if !input.include_providers {
                if let Some(object) = preview.as_object_mut() {
                    object.remove("providers");
                }
            }

            let output = serde_json::to_string_pretty(&preview).map_err(|error| {
                format!("Failed to serialize DX catalog provider settings preview: {error}")
            })?;

            event_stream.update_fields(
                acp::ToolCallUpdateFields::new().title("Inspected DX catalog provider settings"),
            );

            Ok(output)
        })
    }
}
