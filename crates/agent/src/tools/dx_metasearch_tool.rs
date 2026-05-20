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

/// Search through the local DX metasearch service and return compact cited results.
///
/// Use this for multi-engine web, code, docs, news, science, image, video, and source discovery
/// when the user needs cited online context through the DX metasearch stack.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxMetasearchToolInput {
    /// Search term or question to send to the DX metasearch service.
    pub query: String,
    /// Optional categories, for example: general, it, science, news, images, videos, music, files.
    pub categories: Vec<String>,
    /// Optional exact metasearch engine names for narrow searches.
    pub engines: Vec<String>,
    /// Optional language code, for example "en".
    pub language: Option<String>,
    /// Safe search level: 0 off, 1 moderate, 2 strict.
    pub safe_search: Option<u8>,
    /// Search result page, starting at 1.
    pub page: Option<u32>,
    /// Optional time range such as day, week, month, or year.
    pub time_range: Option<String>,
    /// Maximum compact citations to return. Defaults to 8 and caps at 20.
    pub max_results: Option<usize>,
    /// Optional service base URL. Defaults to DX_METASEARCH_BASE_URL or http://127.0.0.1:8888.
    pub base_url: Option<String>,
}

impl Default for DxMetasearchToolInput {
    fn default() -> Self {
        Self {
            query: String::new(),
            categories: vec!["general".to_string()],
            engines: Vec::new(),
            language: Some("en".to_string()),
            safe_search: Some(1),
            page: Some(1),
            time_range: None,
            max_results: Some(8),
            base_url: None,
        }
    }
}

impl From<DxMetasearchToolInput> for dx_metasearch_agent_bridge::DxMetasearchRequest {
    fn from(input: DxMetasearchToolInput) -> Self {
        Self {
            query: input.query,
            categories: input.categories,
            engines: input.engines,
            language: input.language,
            safe_search: input.safe_search,
            page: input.page,
            time_range: input.time_range,
            max_results: input.max_results,
            base_url: input.base_url,
        }
    }
}

pub struct DxMetasearchTool {
    http_client: Arc<HttpClientWithUrl>,
}

impl DxMetasearchTool {
    pub fn new(http_client: Arc<HttpClientWithUrl>) -> Self {
        Self { http_client }
    }
}

impl AgentTool for DxMetasearchTool {
    type Input = DxMetasearchToolInput;
    type Output = String;

    const NAME: &'static str = "search_dx_metasearch";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Fetch
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            format!(
                "Search DX metasearch for {}",
                MarkdownInlineCode(&input.query)
            )
            .into()
        } else {
            "Search DX metasearch".into()
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
            let query = input.query.trim().to_string();
            if query.is_empty() {
                return Err("DX metasearch query is required.".to_string());
            }

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![query.clone()];
                permission_values.extend(input.categories.iter().cloned());
                permission_values.extend(input.engines.iter().cloned());
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(
                    format!("Search DX metasearch for {}", MarkdownInlineCode(&query)),
                    context,
                    cx,
                )
            });

            let search_task = cx.background_spawn({
                let http_client = http_client.clone();
                async move {
                    authorize.await.map_err(|error| error.to_string())?;
                    dx_metasearch_agent_bridge::search_metasearch(http_client, input.into()).await
                }
            });

            let response = futures::select! {
                result = search_task.fuse() => {
                    result
                }
                _ = event_stream.cancelled_by_user().fuse() => {
                    return Err("DX metasearch cancelled by user".to_string());
                }
            }?;

            event_stream.update_fields(
                acp::ToolCallUpdateFields::new()
                    .title(format!(
                        "Searched DX metasearch: {} result(s)",
                        response.summary.returned_result_count
                    ))
                    .content(
                        response
                            .results
                            .iter()
                            .map(|result| {
                                acp::ToolCallContent::Content(acp::Content::new(
                                    acp::ContentBlock::ResourceLink(
                                        acp::ResourceLink::new(
                                            result.title.clone(),
                                            result.url.clone(),
                                        )
                                        .title(format!("[{}] {}", result.citation_id, result.title))
                                        .description(
                                            format!("{} ({})", result.snippet, result.engine),
                                        ),
                                    ),
                                ))
                            })
                            .collect::<Vec<_>>(),
                    ),
            );

            serde_json::to_string_pretty(&response)
                .map_err(|error| format!("Failed to serialize DX metasearch response: {error}"))
        })
    }
}
