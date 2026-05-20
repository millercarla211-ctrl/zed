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

/// Fetch and compact a selected DX metasearch source-pack URL.
///
/// Use this after `search_dx_metasearch` returns a source pack when a short result excerpt is not
/// enough and the agent needs a bounded, token-aware page extract for cited context.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxMetasearchSourceExtractToolInput {
    /// Source URL from a DX metasearch source-pack item.
    pub url: String,
    /// Optional source-pack ID such as S1.
    pub source_id: Option<String>,
    /// Optional title from the source-pack item.
    pub title: Option<String>,
    /// Optional engine name from the source-pack item.
    pub engine: Option<String>,
    /// Optional source category from the source-pack item.
    pub category: Option<String>,
    /// Maximum readable text characters to return. Defaults to 4000 and caps at 12000.
    pub max_chars: Option<usize>,
}

impl Default for DxMetasearchSourceExtractToolInput {
    fn default() -> Self {
        Self {
            url: String::new(),
            source_id: None,
            title: None,
            engine: None,
            category: None,
            max_chars: Some(4_000),
        }
    }
}

impl From<DxMetasearchSourceExtractToolInput>
    for dx_metasearch_agent_bridge::DxMetasearchSourceExtractRequest
{
    fn from(input: DxMetasearchSourceExtractToolInput) -> Self {
        Self {
            url: input.url,
            source_id: input.source_id,
            title: input.title,
            engine: input.engine,
            category: input.category,
            max_chars: input.max_chars,
        }
    }
}

pub struct DxMetasearchSourceExtractTool {
    http_client: Arc<HttpClientWithUrl>,
}

impl DxMetasearchSourceExtractTool {
    pub fn new(http_client: Arc<HttpClientWithUrl>) -> Self {
        Self { http_client }
    }
}

impl AgentTool for DxMetasearchSourceExtractTool {
    type Input = DxMetasearchSourceExtractToolInput;
    type Output = String;

    const NAME: &'static str = "extract_dx_metasearch_source";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Fetch
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            if let Some(source_id) = input.source_id.as_deref() {
                format!("Extract DX source {}", MarkdownInlineCode(source_id)).into()
            } else if let Some(title) = input.title.as_deref() {
                format!("Extract DX source {}", MarkdownInlineCode(title)).into()
            } else if !input.url.trim().is_empty() {
                format!("Extract DX source {}", MarkdownInlineCode(&input.url)).into()
            } else {
                "Extract DX source".into()
            }
        } else {
            "Extract DX source".into()
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
            let url = input.url.trim().to_string();
            if url.is_empty() {
                return Err("DX metasearch source URL is required.".to_string());
            }

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![url.clone()];
                if let Some(source_id) = input.source_id.clone() {
                    permission_values.push(source_id);
                }
                if let Some(title) = input.title.clone() {
                    permission_values.push(title);
                }
                if let Some(engine) = input.engine.clone() {
                    permission_values.push(engine);
                }
                if let Some(category) = input.category.clone() {
                    permission_values.push(category);
                }
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            let extract_task = cx.background_spawn({
                let http_client = http_client.clone();
                async move {
                    authorize.await.map_err(|error| error.to_string())?;
                    dx_metasearch_agent_bridge::extract_metasearch_source(http_client, input.into())
                        .await
                }
            });

            let response = futures::select! {
                result = extract_task.fuse() => {
                    result
                }
                _ = event_stream.cancelled_by_user().fuse() => {
                    return Err("DX metasearch source extraction cancelled by user".to_string());
                }
            }?;

            let title = response
                .request
                .title
                .clone()
                .or_else(|| response.request.source_id.clone())
                .unwrap_or_else(|| response.request.url.clone());
            event_stream.update_fields(
                acp::ToolCallUpdateFields::new()
                    .title(format!(
                        "Extracted DX source: {} char(s)",
                        response.content.extracted_chars
                    ))
                    .content(vec![acp::ToolCallContent::Content(acp::Content::new(
                        acp::ContentBlock::ResourceLink(
                            acp::ResourceLink::new(title, response.request.url.clone())
                                .description(format!(
                                    "{} char(s), about {} token(s), kind {}",
                                    response.content.extracted_chars,
                                    response.content.estimated_tokens,
                                    response.content.kind
                                )),
                        ),
                    ))]),
            );

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX metasearch source extract response: {error}")
            })
        })
    }
}
