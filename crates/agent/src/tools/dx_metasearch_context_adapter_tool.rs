use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_metasearch_context_adapter};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, Entity, SharedString, Task};
use paths::data_dir;
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use util::markdown::MarkdownInlineCode;

const DX_METASEARCH_CONTEXT_LATEST_FILE_NAME: &str = "latest-dx-metasearch-context-receipt.json";

/// Prepare metasearch source packs and extracts for serializer/RLM compaction.
///
/// Use this after `search_dx_metasearch` and `extract_dx_metasearch_source` to create a compact,
/// citation-preserving context bundle before handing sources to another Agent turn or future
/// external serializer/RLM execution.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxMetasearchContextAdapterToolInput {
    /// Optional `zed.dx.metasearch.source_pack.v1` object, search response, or stringified JSON.
    pub source_pack: Option<Value>,
    /// Optional `zed.dx.metasearch.source_extract.v1` objects or stringified JSON values.
    pub source_extracts: Vec<Value>,
    /// Optional question or task this context bundle should support.
    pub question: Option<String>,
    /// Approximate token budget for the compact context. Defaults to 1600 and caps at 8000.
    pub token_budget: Option<usize>,
    /// Persist the compact context bundle to a managed receipt file after explicit authorization.
    pub write_context_receipt: bool,
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub receipt_root_mode: DxMetasearchContextReceiptRootMode,
}

impl Default for DxMetasearchContextAdapterToolInput {
    fn default() -> Self {
        Self {
            source_pack: None,
            source_extracts: Vec::new(),
            question: None,
            token_budget: Some(1_600),
            write_context_receipt: false,
            receipt_root_mode: DxMetasearchContextReceiptRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxMetasearchContextReceiptRootMode {
    #[default]
    Workspace,
    ZedData,
}

impl From<DxMetasearchContextAdapterToolInput>
    for dx_metasearch_context_adapter::DxMetasearchContextAdapterRequest
{
    fn from(input: DxMetasearchContextAdapterToolInput) -> Self {
        Self {
            source_pack: input.source_pack,
            source_extracts: input.source_extracts,
            question: input.question,
            token_budget: input.token_budget,
        }
    }
}

pub struct DxMetasearchContextAdapterTool {
    project: Entity<Project>,
}

impl DxMetasearchContextAdapterTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxMetasearchContextAdapterTool {
    type Input = DxMetasearchContextAdapterToolInput;
    type Output = String;

    const NAME: &'static str = "prepare_dx_metasearch_context";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Edit
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            let source_count =
                input.source_extracts.len() + usize::from(input.source_pack.is_some());
            format!(
                "Prepare DX metasearch context for {} source input(s)",
                MarkdownInlineCode(&source_count.to_string())
            )
            .into()
        } else {
            "Prepare DX metasearch context".into()
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
            let source_input_count =
                input.source_extracts.len() + usize::from(input.source_pack.is_some());
            if source_input_count == 0 {
                return Err(
                    "DX metasearch context adapter needs a source pack or source extract."
                        .to_string(),
                );
            }

            let receipt_target = input.write_context_receipt.then(|| {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxMetasearchContextReceiptTarget::new(project_root, input.receipt_root_mode)
            });

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![format!("{source_input_count} source input(s)")];
                if let Some(question) = &input.question {
                    permission_values.push(question.clone());
                }
                if let Some(receipt_target) = &receipt_target {
                    permission_values.push(path_string(&receipt_target.latest_path));
                    permission_values.push(path_string(&receipt_target.archive_path));
                }
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;
            let mut response =
                dx_metasearch_context_adapter::build_metasearch_context_bundle(input.into())?;

            if let Some(receipt_target) = receipt_target {
                response.context_receipt =
                    Some(receipt_target.write_receipt(&response).map_err(|error| {
                        format!("Failed to write DX metasearch context receipt: {error}")
                    })?);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Prepared DX context: {} source(s), about {} token(s)",
                response.summary.included_source_count, response.summary.estimated_tokens
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX metasearch context response: {error}")
            })
        })
    }
}

struct DxMetasearchContextReceiptTarget {
    root_mode: DxMetasearchContextReceiptRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxMetasearchContextReceiptTarget {
    fn new(project_root: Option<PathBuf>, root_mode: DxMetasearchContextReceiptRootMode) -> Self {
        let use_workspace = matches!(root_mode, DxMetasearchContextReceiptRootMode::Workspace)
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
            allowed_root.join("dx-metasearch").join("context")
        } else {
            allowed_root.join("context")
        };
        let latest_path = receipt_dir.join(DX_METASEARCH_CONTEXT_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-metasearch-context-{}.json",
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
        response: &dx_metasearch_context_adapter::DxMetasearchContextBundle,
    ) -> Result<dx_metasearch_context_adapter::DxMetasearchContextReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_metasearch_context_adapter::DX_METASEARCH_CONTEXT_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxMetasearchContextAdapterTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "context_bundle": response,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "runs_external_serializer": false,
                "runs_external_rlm": false,
                "fetches_network": false,
                "starts_metasearch_server": false,
                "dispatches_browser_input": false,
            },
            "next_action": "Use this compact context receipt for the next Agent turn, then call plan_dx_serializer_rlm_execution after reviewing the adapter contract."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("Failed to serialize context receipt: {error}"))?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX metasearch context receipt directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX metasearch latest context receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX metasearch context receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(dx_metasearch_context_adapter::DxMetasearchContextReceipt {
            schema: dx_metasearch_context_adapter::DX_METASEARCH_CONTEXT_RECEIPT_SCHEMA,
            status: "written",
            root_mode: self.root_mode_label().to_string(),
            receipt_dir: path_string(&self.receipt_dir),
            latest_path: path_string(&self.latest_path),
            archive_path: path_string(&self.archive_path),
            written_bytes: receipt_json.len(),
            context_bundle_schema: response.schema,
            item_count: response.summary.included_source_count,
            estimated_tokens: response.summary.estimated_tokens,
            next_action: "Use the latest context receipt for Agent context; call plan_dx_serializer_rlm_execution when external reduction needs an approved handoff.".to_string(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX metasearch context receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxMetasearchContextReceiptRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxMetasearchContextReceiptRootMode::Workspace => "zed_data_fallback",
            DxMetasearchContextReceiptRootMode::ZedData => "zed_data",
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

fn current_epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
