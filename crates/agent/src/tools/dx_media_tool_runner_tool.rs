use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_media_tool_runner};
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

const DX_MEDIA_EXECUTION_LATEST_FILE_NAME: &str = "latest-dx-media-tool-execution-receipt.json";

/// Execute an approved DX media runner gate with ffmpeg/ffprobe and write produced-file receipts.
///
/// This tool consumes a runner-ready `zed.dx.media_tool.runner_gate.v1` object or receipt, asks for
/// explicit execution approval, runs the argument vector without shell interpolation, refuses
/// overwrite-risk outputs, and writes an execution receipt under the managed DX media directory.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxMediaToolRunnerToolInput {
    /// `zed.dx.media_tool.runner_gate.v1` object, gate receipt, or stringified JSON.
    pub runner_gate: Value,
    /// Explicit approval flag for running ffmpeg/ffprobe.
    pub approve_execution: bool,
    /// Require the managed execution receipt. Must stay true for approved execution.
    pub require_execution_receipt: bool,
    /// Prefer workspace-local execution receipts under `<workspace>/tools`; falls back to Zed data.
    pub artifact_root_mode: DxMediaToolRunnerArtifactRootMode,
}

impl Default for DxMediaToolRunnerToolInput {
    fn default() -> Self {
        Self {
            runner_gate: Value::Null,
            approve_execution: false,
            require_execution_receipt: true,
            artifact_root_mode: DxMediaToolRunnerArtifactRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxMediaToolRunnerArtifactRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxMediaToolRunnerTool {
    project: Entity<Project>,
}

impl DxMediaToolRunnerTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxMediaToolRunnerTool {
    type Input = DxMediaToolRunnerToolInput;
    type Output = String;

    const NAME: &'static str = "execute_dx_media_tool";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            if input.approve_execution {
                "Execute approved DX media tool".into()
            } else {
                "Prepare DX media execution".into()
            }
        } else {
            "Execute DX media tool".into()
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
            if input.runner_gate.is_null() {
                return Err("DX media execution needs a runner gate or gate receipt.".to_string());
            }
            if input.approve_execution && !input.require_execution_receipt {
                return Err(
                    "Approved DX media execution requires an execution receipt.".to_string()
                );
            }

            let artifact_target = {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxMediaToolRunnerArtifactTarget::new(project_root, input.artifact_root_mode)
            };

            let authorize = cx.update(|cx| {
                let permission_values = vec![
                    format!("approve_execution={}", input.approve_execution),
                    format!(
                        "require_execution_receipt={}",
                        input.require_execution_receipt
                    ),
                    path_string(&artifact_target.receipt_dir),
                    path_string(&artifact_target.latest_path),
                    path_string(&artifact_target.archive_path),
                ];
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;
            let mut response =
                dx_media_tool_runner::execute_dx_media_tool(artifact_target.request(input))?;

            response.execution_receipt =
                Some(artifact_target.write_receipt(&response).map_err(|error| {
                    format!("Failed to write DX media execution receipt: {error}")
                })?);

            event_stream.update_fields(
                acp::ToolCallUpdateFields::new()
                    .title(format!("DX media tool: {}", response.execution.status)),
            );

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX media execution response: {error}")
            })
        })
    }
}

struct DxMediaToolRunnerArtifactTarget {
    root_mode: DxMediaToolRunnerArtifactRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxMediaToolRunnerArtifactTarget {
    fn new(project_root: Option<PathBuf>, root_mode: DxMediaToolRunnerArtifactRootMode) -> Self {
        let allowed_root = match (root_mode, project_root.as_ref()) {
            (DxMediaToolRunnerArtifactRootMode::Workspace, Some(root)) => {
                root.join("tools").join("dx-media")
            }
            _ => data_dir().join("dx-media"),
        };
        let receipt_dir = allowed_root.join("executions");
        let latest_path = receipt_dir.join(DX_MEDIA_EXECUTION_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-media-tool-execution-{}.json",
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

    fn request(
        &self,
        input: DxMediaToolRunnerToolInput,
    ) -> dx_media_tool_runner::DxMediaToolExecutionRequest {
        dx_media_tool_runner::DxMediaToolExecutionRequest {
            runner_gate: input.runner_gate,
            approve_execution: input.approve_execution,
            require_execution_receipt: input.require_execution_receipt,
            root_mode: self.root_mode_label().to_string(),
        }
    }

    fn write_receipt(
        &self,
        response: &dx_media_tool_runner::DxMediaToolExecution,
    ) -> Result<dx_media_tool_runner::DxMediaToolExecutionReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_media_tool_runner::DX_MEDIA_TOOL_EXECUTION_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxMediaToolRunnerTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "media_execution": response,
            "produced_files": &response.produced_files,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "runs_shell": false,
                "runs_external_process": response.execution.ran_external_process,
                "runs_ffmpeg_or_ffprobe": response.execution.ran_external_process,
                "deletes_files": false,
                "overwrites_outputs": false,
                "writes_media_outputs": response.execution.wrote_media_outputs,
                "dispatches_browser_input": false,
            },
            "next_action": "Attach produced file paths as sources or render them in the DX media panel; keep binary data out of model context."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("Failed to serialize media execution receipt: {error}"))?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX media execution receipt directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX media latest execution receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX media execution receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(dx_media_tool_runner::DxMediaToolExecutionReceipt {
            schema: dx_media_tool_runner::DX_MEDIA_TOOL_EXECUTION_RECEIPT_SCHEMA,
            status: "written",
            root_mode: self.root_mode_label().to_string(),
            receipt_dir: path_string(&self.receipt_dir),
            latest_path: path_string(&self.latest_path),
            archive_path: path_string(&self.archive_path),
            written_bytes: receipt_json.len(),
            execution_schema: response.schema,
            action: response.gate.action.clone(),
            produced_file_count: response.produced_files.len(),
            next_action: "Use this media execution receipt for source rails, media panel history, or launch demos.".to_string(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX media execution receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }
        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxMediaToolRunnerArtifactRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxMediaToolRunnerArtifactRootMode::Workspace => "zed_data_fallback",
            DxMediaToolRunnerArtifactRootMode::ZedData => "zed_data",
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
