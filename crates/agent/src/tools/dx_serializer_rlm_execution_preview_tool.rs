use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_serializer_rlm_execution_preview};
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

const DX_SERIALIZER_RLM_EXECUTION_PREVIEW_LATEST_FILE_NAME: &str =
    "latest-dx-serializer-rlm-execution-preview-receipt.json";

/// Preview an external serializer/RLM reducer run without executing it.
///
/// This tool consumes runner-gate and reduced-context receipts and writes managed dry-run preview
/// receipts only. It never runs external reducers, Cargo, model calls, network, shell commands, or
/// unmanaged file writes.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxSerializerRlmExecutionPreviewToolInput {
    /// `zed.dx.serializer_rlm.runner_gate.v1` object, runner-gate receipt, or stringified JSON.
    pub runner_gate: Value,
    /// `zed.dx.serializer_rlm.reduced_context.v1` object, reduced-context receipt, or stringified JSON.
    pub reduced_context: Value,
    /// Explicit approval to write the dry-run preview receipt.
    pub approve_execution_preview: bool,
    /// Explicitly allow future reducer steps that would make RLM model calls. This tool still makes none.
    pub allow_model_calls: bool,
    /// Require a runner-ready gate before the preview can become ready.
    pub require_runner_ready: bool,
    /// Require a reduced-context-ready receipt before the preview can become ready.
    pub require_reduced_context_ready: bool,
    /// Persist the execution preview to a managed receipt file after authorization.
    pub write_execution_preview_receipt: bool,
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data.
    pub receipt_root_mode: DxSerializerRlmExecutionPreviewReceiptRootMode,
}

impl Default for DxSerializerRlmExecutionPreviewToolInput {
    fn default() -> Self {
        Self {
            runner_gate: Value::Null,
            reduced_context: Value::Null,
            approve_execution_preview: false,
            allow_model_calls: false,
            require_runner_ready: true,
            require_reduced_context_ready: true,
            write_execution_preview_receipt: true,
            receipt_root_mode: DxSerializerRlmExecutionPreviewReceiptRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxSerializerRlmExecutionPreviewReceiptRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxSerializerRlmExecutionPreviewTool {
    project: Entity<Project>,
}

impl DxSerializerRlmExecutionPreviewTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxSerializerRlmExecutionPreviewTool {
    type Input = DxSerializerRlmExecutionPreviewToolInput;
    type Output = String;

    const NAME: &'static str = "preview_dx_serializer_rlm_reducer_execution";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Edit
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            if input.approve_execution_preview {
                return "Preview approved DX serializer/RLM reducer run".into();
            }
        }

        "Preview DX serializer/RLM reducer run".into()
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
            if input.runner_gate.is_null() {
                return Err(
                    "DX serializer/RLM execution preview needs a runner gate or receipt."
                        .to_string(),
                );
            }
            if input.reduced_context.is_null() {
                return Err(
                    "DX serializer/RLM execution preview needs a reduced-context receipt."
                        .to_string(),
                );
            }

            let receipt_target = input.write_execution_preview_receipt.then(|| {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxSerializerRlmExecutionPreviewReceiptTarget::new(
                    project_root,
                    input.receipt_root_mode,
                )
            });

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![
                    format!(
                        "approve_execution_preview={}",
                        input.approve_execution_preview
                    ),
                    format!("allow_model_calls={}", input.allow_model_calls),
                    format!("require_runner_ready={}", input.require_runner_ready),
                    format!(
                        "require_reduced_context_ready={}",
                        input.require_reduced_context_ready
                    ),
                ];
                if let Some(receipt_target) = &receipt_target {
                    permission_values.push(path_string(&receipt_target.latest_path));
                    permission_values.push(path_string(&receipt_target.archive_path));
                }
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;
            let root_mode = receipt_target
                .as_ref()
                .map(DxSerializerRlmExecutionPreviewReceiptTarget::root_mode_label)
                .unwrap_or_else(|| input.receipt_root_mode.label())
                .to_string();
            let mut response =
                dx_serializer_rlm_execution_preview::build_serializer_rlm_execution_preview(
                    dx_serializer_rlm_execution_preview::DxSerializerRlmExecutionPreviewRequest {
                        runner_gate: input.runner_gate,
                        reduced_context: input.reduced_context,
                        approve_execution_preview: input.approve_execution_preview,
                        allow_model_calls: input.allow_model_calls,
                        require_runner_ready: input.require_runner_ready,
                        require_reduced_context_ready: input.require_reduced_context_ready,
                        root_mode,
                    },
                )?;

            if let Some(receipt_target) = receipt_target {
                response.execution_preview_receipt =
                    Some(receipt_target.write_receipt(&response).map_err(|error| {
                        format!(
                            "Failed to write DX serializer/RLM execution preview receipt: {error}"
                        )
                    })?);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Previewed DX serializer/RLM reducer run: {}",
                response.preview.status
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX serializer/RLM execution preview: {error}")
            })
        })
    }
}

struct DxSerializerRlmExecutionPreviewReceiptTarget {
    root_mode: DxSerializerRlmExecutionPreviewReceiptRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxSerializerRlmExecutionPreviewReceiptTarget {
    fn new(
        project_root: Option<PathBuf>,
        root_mode: DxSerializerRlmExecutionPreviewReceiptRootMode,
    ) -> Self {
        let allowed_root = match (root_mode, project_root.as_ref()) {
            (DxSerializerRlmExecutionPreviewReceiptRootMode::Workspace, Some(root)) => {
                root.join("tools").join("dx-serializer-rlm")
            }
            _ => data_dir().join("dx-serializer-rlm"),
        };
        let receipt_dir = allowed_root.join("execution-previews");
        let latest_path = receipt_dir.join(DX_SERIALIZER_RLM_EXECUTION_PREVIEW_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-serializer-rlm-execution-preview-{}.json",
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
        response: &dx_serializer_rlm_execution_preview::DxSerializerRlmExecutionPreview,
    ) -> Result<dx_serializer_rlm_execution_preview::DxSerializerRlmExecutionPreviewReceipt, String>
    {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_serializer_rlm_execution_preview::DX_SERIALIZER_RLM_EXECUTION_PREVIEW_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxSerializerRlmExecutionPreviewTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "execution_preview": response,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "writes_execution_preview_receipt_only": true,
                "runs_external_serializer": false,
                "runs_external_rlm": false,
                "runs_model_calls": false,
                "runs_cargo": false,
                "fetches_network": false,
                "dispatches_browser_input": false,
                "writes_execution_output": false,
            },
            "next_action": "Use this dry-run preview receipt only as a review input for a future separately approved external reducer executor."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt).map_err(|error| {
            format!("Failed to serialize serializer/RLM execution preview receipt: {error}")
        })?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX serializer/RLM execution preview directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX serializer/RLM latest execution preview receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX serializer/RLM execution preview receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(
            dx_serializer_rlm_execution_preview::DxSerializerRlmExecutionPreviewReceipt {
                schema: dx_serializer_rlm_execution_preview::DX_SERIALIZER_RLM_EXECUTION_PREVIEW_RECEIPT_SCHEMA,
                status: "written",
                root_mode: self.root_mode_label().to_string(),
                receipt_dir: path_string(&self.receipt_dir),
                latest_path: path_string(&self.latest_path),
                archive_path: path_string(&self.archive_path),
                written_bytes: receipt_json.len(),
                execution_preview_ready: response.preview.execution_preview_ready,
                reducer: response.gate.reducer.clone(),
                step_count: response.planned_steps.len(),
                would_run_model_calls: response.preview.would_run_model_calls,
                next_action: response.next_action.clone(),
            },
        )
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX serializer/RLM execution preview receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxSerializerRlmExecutionPreviewReceiptRootMode::Workspace
                if self.project_root.is_some() =>
            {
                "workspace"
            }
            DxSerializerRlmExecutionPreviewReceiptRootMode::Workspace => "zed_data_fallback",
            DxSerializerRlmExecutionPreviewReceiptRootMode::ZedData => "zed_data",
        }
    }
}

impl DxSerializerRlmExecutionPreviewReceiptRootMode {
    fn label(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::ZedData => "zed_data",
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

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

fn current_epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
