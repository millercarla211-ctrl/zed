use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_metasearch_agent_bridge};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use futures::FutureExt as _;
use gpui::{App, Entity, SharedString, Task};
use http_client::HttpClientWithUrl;
use paths::data_dir;
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use util::markdown::MarkdownInlineCode;

const DX_METASEARCH_SOURCE_PACK_LATEST_FILE_NAME: &str =
    "latest-dx-metasearch-source-pack-receipt.json";

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
    /// Persist the returned source pack to a managed receipt file after explicit authorization.
    pub write_source_pack_receipt: bool,
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub receipt_root_mode: DxMetasearchSourcePackReceiptRootMode,
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
            write_source_pack_receipt: false,
            receipt_root_mode: DxMetasearchSourcePackReceiptRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxMetasearchSourcePackReceiptRootMode {
    #[default]
    Workspace,
    ZedData,
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
    project: Entity<Project>,
    http_client: Arc<HttpClientWithUrl>,
}

impl DxMetasearchTool {
    pub fn new(project: Entity<Project>, http_client: Arc<HttpClientWithUrl>) -> Self {
        Self {
            project,
            http_client,
        }
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
        let project = self.project.clone();

        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let query = input.query.trim().to_string();
            if query.is_empty() {
                return Err("DX metasearch query is required.".to_string());
            }

            let source_pack_receipt_target = input.write_source_pack_receipt.then(|| {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxMetasearchSourcePackReceiptTarget::new(project_root, input.receipt_root_mode)
            });

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![query.clone()];
                permission_values.extend(input.categories.iter().cloned());
                permission_values.extend(input.engines.iter().cloned());
                if let Some(receipt_target) = &source_pack_receipt_target {
                    permission_values.push(path_string(&receipt_target.latest_path));
                    permission_values.push(path_string(&receipt_target.archive_path));
                }
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
            let mut response = response;

            if let Some(receipt_target) = source_pack_receipt_target {
                response.source_pack_receipt =
                    Some(receipt_target.write_receipt(&response).map_err(|error| {
                        format!("Failed to write DX metasearch source-pack receipt: {error}")
                    })?);
            }

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

struct DxMetasearchSourcePackReceiptTarget {
    root_mode: DxMetasearchSourcePackReceiptRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxMetasearchSourcePackReceiptTarget {
    fn new(
        project_root: Option<PathBuf>,
        root_mode: DxMetasearchSourcePackReceiptRootMode,
    ) -> Self {
        let use_workspace = matches!(root_mode, DxMetasearchSourcePackReceiptRootMode::Workspace)
            && project_root.is_some();
        let allowed_root = if use_workspace {
            project_root
                .as_ref()
                .expect("workspace root checked above")
                .join("tools")
        } else {
            data_dir().join("dx-metasearch")
        };
        let receipt_dir = if use_workspace {
            allowed_root.join("dx-metasearch").join("source-packs")
        } else {
            allowed_root.join("source-packs")
        };
        let latest_path = receipt_dir.join(DX_METASEARCH_SOURCE_PACK_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-metasearch-source-pack-{}.json",
            current_epoch_millis()
        ));

        Self {
            root_mode,
            project_root,
            allowed_root,
            receipt_dir,
            latest_path,
            archive_path,
        }
    }

    fn write_receipt(
        &self,
        response: &dx_metasearch_agent_bridge::DxMetasearchCompactResponse,
    ) -> Result<dx_metasearch_agent_bridge::DxMetasearchSourcePackReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_metasearch_agent_bridge::DX_METASEARCH_SOURCE_PACK_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxMetasearchTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "search": {
                "schema": response.schema,
                "query": response.query,
                "request": response.request,
                "summary": response.summary,
                "source": response.source,
            },
            "source_pack": response.source_pack,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "starts_metasearch_server": false,
                "deep_fetches_pages": false,
                "runs_serializer_or_rlm": false,
                "dispatches_browser_input": false,
            },
            "next_action": "Use the latest receipt for Agent source context, run extract_dx_metasearch_source for selected source IDs when short excerpts are not enough, then call prepare_dx_metasearch_context for a compact serializer/RLM-ready bundle."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("Failed to serialize source-pack receipt: {error}"))?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX metasearch receipt directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX metasearch latest receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX metasearch receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(dx_metasearch_agent_bridge::DxMetasearchSourcePackReceipt {
            schema: dx_metasearch_agent_bridge::DX_METASEARCH_SOURCE_PACK_RECEIPT_SCHEMA,
            status: "written",
            root_mode: self.root_mode_label().to_string(),
            receipt_dir: path_string(&self.receipt_dir),
            latest_path: path_string(&self.latest_path),
            archive_path: path_string(&self.archive_path),
            written_bytes: receipt_json.len(),
            source_pack_schema: response.source_pack.schema,
            item_count: response.source_pack.item_count,
            estimated_tokens: response.source_pack.estimated_tokens,
            next_action: "Use the latest receipt for Agent source context, extract selected source IDs when needed, then call prepare_dx_metasearch_context for a compact bundle.".to_string(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX metasearch receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxMetasearchSourcePackReceiptRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxMetasearchSourcePackReceiptRootMode::Workspace => "zed_data_fallback",
            DxMetasearchSourcePackReceiptRootMode::ZedData => "zed_data",
        }
    }
}

fn workspace_root_for_project(project: &Entity<Project>, cx: &App) -> Option<PathBuf> {
    project
        .read(cx)
        .visible_worktrees(cx)
        .next()
        .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf())
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}

fn current_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
