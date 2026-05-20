use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_serializer_rlm_execution_plan};
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

const DX_SERIALIZER_RLM_EXECUTION_LATEST_FILE_NAME: &str =
    "latest-dx-serializer-rlm-execution-receipt.json";

/// Create an approved execution plan for the DX serializer/RLM context pipeline.
///
/// This tool validates a prepared metasearch context bundle, checks the G-drive serializer/RLM
/// roots, and writes an optional receipt for the future external runner. It does not run external
/// processes or model calls.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxSerializerRlmExecutionPlanToolInput {
    /// `zed.dx.serializer_rlm.context_bundle.v1` object, context receipt, or stringified JSON.
    pub context_bundle: Value,
    /// Optional task the reducer should optimize for.
    pub task: Option<String>,
    /// Reducer mode: hybrid, serializer_only, or rlm_only.
    pub reducer: Option<String>,
    /// Explicit approval flag for the future external serializer/RLM runner.
    pub approve_external_execution: bool,
    /// Persist the execution plan to a managed receipt file after authorization.
    pub write_execution_receipt: bool,
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub receipt_root_mode: DxSerializerRlmExecutionReceiptRootMode,
}

impl Default for DxSerializerRlmExecutionPlanToolInput {
    fn default() -> Self {
        Self {
            context_bundle: Value::Null,
            task: None,
            reducer: Some("hybrid".to_string()),
            approve_external_execution: false,
            write_execution_receipt: false,
            receipt_root_mode: DxSerializerRlmExecutionReceiptRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxSerializerRlmExecutionReceiptRootMode {
    #[default]
    Workspace,
    ZedData,
}

impl From<DxSerializerRlmExecutionPlanToolInput>
    for dx_serializer_rlm_execution_plan::DxSerializerRlmExecutionPlanRequest
{
    fn from(input: DxSerializerRlmExecutionPlanToolInput) -> Self {
        Self {
            context_bundle: input.context_bundle,
            task: input.task,
            reducer: input.reducer,
            approve_external_execution: input.approve_external_execution,
        }
    }
}

pub struct DxSerializerRlmExecutionPlanTool {
    project: Entity<Project>,
}

impl DxSerializerRlmExecutionPlanTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxSerializerRlmExecutionPlanTool {
    type Input = DxSerializerRlmExecutionPlanToolInput;
    type Output = String;

    const NAME: &'static str = "plan_dx_serializer_rlm_execution";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            let reducer = input.reducer.unwrap_or_else(|| "hybrid".to_string());
            format!(
                "Plan DX serializer/RLM execution {}",
                MarkdownInlineCode(&reducer)
            )
            .into()
        } else {
            "Plan DX serializer/RLM execution".into()
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
            if input.context_bundle.is_null() {
                return Err(
                    "DX serializer/RLM execution plan needs a context bundle or receipt."
                        .to_string(),
                );
            }

            let receipt_target = input.write_execution_receipt.then(|| {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxSerializerRlmExecutionReceiptTarget::new(project_root, input.receipt_root_mode)
            });

            let authorize = cx.update(|cx| {
                let reducer = input
                    .reducer
                    .clone()
                    .unwrap_or_else(|| "hybrid".to_string());
                let mut permission_values = vec![
                    format!("reducer={reducer}"),
                    format!(
                        "approve_external_execution={}",
                        input.approve_external_execution
                    ),
                ];
                if let Some(task) = &input.task {
                    permission_values.push(task.clone());
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
                dx_serializer_rlm_execution_plan::build_serializer_rlm_execution_plan(
                    input.into(),
                )?;

            if let Some(receipt_target) = receipt_target {
                response.execution_receipt =
                    Some(receipt_target.write_receipt(&response).map_err(|error| {
                        format!("Failed to write DX serializer/RLM execution receipt: {error}")
                    })?);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Planned DX serializer/RLM execution: {}",
                response.approval.status
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX serializer/RLM execution plan: {error}")
            })
        })
    }
}

struct DxSerializerRlmExecutionReceiptTarget {
    root_mode: DxSerializerRlmExecutionReceiptRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxSerializerRlmExecutionReceiptTarget {
    fn new(
        project_root: Option<PathBuf>,
        root_mode: DxSerializerRlmExecutionReceiptRootMode,
    ) -> Self {
        let use_workspace = matches!(
            root_mode,
            DxSerializerRlmExecutionReceiptRootMode::Workspace
        ) && project_root.is_some();
        let allowed_root = if use_workspace {
            project_root
                .as_ref()
                .expect("workspace root checked above")
                .join("tools")
        } else {
            data_dir().join("dx-serializer-rlm")
        };
        let receipt_dir = if use_workspace {
            allowed_root.join("dx-serializer-rlm").join("execution")
        } else {
            allowed_root.join("execution")
        };
        let latest_path = receipt_dir.join(DX_SERIALIZER_RLM_EXECUTION_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-serializer-rlm-execution-{}.json",
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
        response: &dx_serializer_rlm_execution_plan::DxSerializerRlmExecutionPlan,
    ) -> Result<dx_serializer_rlm_execution_plan::DxSerializerRlmExecutionReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_serializer_rlm_execution_plan::DX_SERIALIZER_RLM_EXECUTION_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxSerializerRlmExecutionPlanTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "execution_plan": response,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "runs_external_serializer": false,
                "runs_external_rlm": false,
                "runs_model_calls": false,
                "runs_cargo": false,
                "fetches_network": false,
                "dispatches_browser_input": false,
            },
            "next_action": "Use this approved plan receipt to wire the future serializer/RLM runner without changing the safety gates."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("Failed to serialize execution receipt: {error}"))?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX serializer/RLM execution receipt directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX serializer/RLM latest execution receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX serializer/RLM execution receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(
            dx_serializer_rlm_execution_plan::DxSerializerRlmExecutionReceipt {
                schema: dx_serializer_rlm_execution_plan::DX_SERIALIZER_RLM_EXECUTION_RECEIPT_SCHEMA,
                status: "written",
                root_mode: self.root_mode_label().to_string(),
                receipt_dir: path_string(&self.receipt_dir),
                latest_path: path_string(&self.latest_path),
                archive_path: path_string(&self.archive_path),
                written_bytes: receipt_json.len(),
                execution_plan_schema: response.schema,
                approval_status: response.approval.status.clone(),
                step_count: response.steps.len(),
                next_action: "Use the latest execution receipt to wire the future external serializer/RLM runner after review.".to_string(),
            },
        )
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX serializer/RLM execution receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxSerializerRlmExecutionReceiptRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxSerializerRlmExecutionReceiptRootMode::Workspace => "zed_data_fallback",
            DxSerializerRlmExecutionReceiptRootMode::ZedData => "zed_data",
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
