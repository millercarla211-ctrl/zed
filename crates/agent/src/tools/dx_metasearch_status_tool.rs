use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_metasearch_agent_bridge};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use futures::FutureExt as _;
use gpui::{App, SharedString, Task};
use http_client::HttpClientWithUrl;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use util::markdown::MarkdownInlineCode;

/// Inspect the local DX metasearch service, runtime health, and engine catalog.
///
/// Use this before targeted metasearch calls when the agent needs to know which engines are
/// available, unhealthy, disabled, or appropriate for exact-engine source searches.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxMetasearchStatusToolInput {
    /// Optional service base URL. Defaults to DX_METASEARCH_BASE_URL or http://127.0.0.1:8888.
    pub base_url: Option<String>,
    /// Include the compact engine catalog from /api/v1/engines.
    pub include_engines: bool,
    /// Maximum engines to include in the compact response. Defaults to 40 and caps at 200.
    pub engine_limit: Option<usize>,
}

impl Default for DxMetasearchStatusToolInput {
    fn default() -> Self {
        Self {
            base_url: None,
            include_engines: true,
            engine_limit: Some(40),
        }
    }
}

impl From<DxMetasearchStatusToolInput> for dx_metasearch_agent_bridge::DxMetasearchStatusRequest {
    fn from(input: DxMetasearchStatusToolInput) -> Self {
        Self {
            base_url: input.base_url,
            include_engines: input.include_engines,
            engine_limit: input.engine_limit,
        }
    }
}

pub struct DxMetasearchStatusTool {
    http_client: Arc<HttpClientWithUrl>,
}

impl DxMetasearchStatusTool {
    pub fn new(http_client: Arc<HttpClientWithUrl>) -> Self {
        Self { http_client }
    }
}

impl AgentTool for DxMetasearchStatusTool {
    type Input = DxMetasearchStatusToolInput;
    type Output = String;

    const NAME: &'static str = "inspect_dx_metasearch";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Fetch
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            if let Some(base_url) = input.base_url.as_deref() {
                format!("Inspect DX metasearch at {}", MarkdownInlineCode(base_url)).into()
            } else {
                "Inspect DX metasearch".into()
            }
        } else {
            "Inspect DX metasearch".into()
        }
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        let http_client = self.http_client.clone();

        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let permission_value = input
                .base_url
                .clone()
                .unwrap_or_else(|| "DX_METASEARCH_BASE_URL/default".to_string());
            let authorize = cx.update(|cx| {
                let context = crate::ToolPermissionContext::new(Self::NAME, vec![permission_value]);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            let inspect_task = cx.background_spawn({
                let http_client = http_client.clone();
                async move {
                    authorize.await.map_err(|error| error.to_string())?;
                    dx_metasearch_agent_bridge::inspect_metasearch_status(http_client, input.into())
                        .await
                }
            });

            let response = futures::select! {
                result = inspect_task.fuse() => {
                    result
                }
                _ = event_stream.cancelled_by_user().fuse() => {
                    return Err("DX metasearch inspection cancelled by user".to_string());
                }
            }?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Inspected DX metasearch: {} engine(s), status {}",
                response.engine_summary.catalog_count, response.service.status
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX metasearch status response: {error}")
            })
        })
    }
}
