use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_catalog_agent_bridge};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, Entity, SharedString, Task};
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use util::markdown::MarkdownInlineCode;

/// Register writable DX catalog providers into native language-model settings.
///
/// The tool writes only provider settings that the catalog adapter bridge marks as safe and writable.
/// It never downloads models, calls provider APIs, stores secrets, or generates a catalog artifact.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxCatalogProviderSettingsRegistrationToolInput {
    /// Limit registration to these catalog provider IDs. Leave empty to register every eligible provider.
    pub provider_ids: Vec<String>,
    /// Return the exact registration receipt without writing native settings.
    pub dry_run: bool,
    /// Include per-provider receipt details and compact model previews.
    pub include_providers: bool,
}

impl Default for DxCatalogProviderSettingsRegistrationToolInput {
    fn default() -> Self {
        Self {
            provider_ids: Vec::new(),
            dry_run: true,
            include_providers: true,
        }
    }
}

impl DxCatalogProviderSettingsRegistrationToolInput {
    fn permission_values(&self) -> Vec<String> {
        let provider_ids = self
            .provider_ids
            .iter()
            .map(|provider_id| provider_id.trim())
            .filter(|provider_id| !provider_id.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        if provider_ids.is_empty() {
            vec!["all eligible DX catalog providers".to_string()]
        } else {
            provider_ids
        }
    }
}

pub struct DxCatalogProviderSettingsRegistrationTool {
    project: Entity<Project>,
}

impl DxCatalogProviderSettingsRegistrationTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxCatalogProviderSettingsRegistrationTool {
    type Input = DxCatalogProviderSettingsRegistrationToolInput;
    type Output = String;

    const NAME: &'static str = "register_dx_catalog_provider_settings";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Edit
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            let action = if input.dry_run {
                "Preview registration for"
            } else {
                "Register"
            };
            let provider_scope = if input.provider_ids.is_empty() {
                "all eligible DX catalog providers".to_string()
            } else {
                format!(
                    "{} DX catalog provider(s): {}",
                    input.provider_ids.len(),
                    MarkdownInlineCode(&input.provider_ids.join(", "))
                )
            };

            format!("{action} {provider_scope}").into()
        } else {
            "Register DX catalog provider settings".into()
        }
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        let project = self.project.clone();

        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let authorize = cx.update(|cx| {
                let context =
                    crate::ToolPermissionContext::new(Self::NAME, input.permission_values());
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;

            let fs = project.read_with(cx, |project, _cx| project.fs().clone());
            let mut result = cx.update(|cx| {
                dx_catalog_agent_bridge::register_provider_settings_from_catalog(
                    fs,
                    &input.provider_ids,
                    input.dry_run,
                    cx,
                )
            });

            if !input.include_providers {
                if let Some(object) = result.as_object_mut() {
                    object.remove("providers");
                }
            }

            let settings_write_queued = result
                .get("settings_write_queued")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let output = serde_json::to_string_pretty(&result).map_err(|error| {
                format!("Failed to serialize DX catalog provider settings receipt: {error}")
            })?;
            let title = if settings_write_queued {
                "Queued DX catalog provider settings registration"
            } else if input.dry_run {
                "Previewed DX catalog provider settings registration"
            } else {
                "Skipped DX catalog provider settings registration"
            };
            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(title.to_string()));

            Ok(output)
        })
    }
}
