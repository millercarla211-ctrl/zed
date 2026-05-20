use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_media_tool_runner_gate};
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

const DX_MEDIA_RUNNER_GATE_LATEST_FILE_NAME: &str = "latest-dx-media-runner-gate-receipt.json";

/// Validate a DX media plan before any future no-shell ffmpeg/ffprobe execution.
///
/// This gate accepts `zed.dx.media_tool.plan.v1` plans or plan receipts and can write a managed
/// receipt proving runner readiness. It does not run ffmpeg, ffprobe, shell commands, deletes, or
/// media writes.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxMediaToolRunnerGateToolInput {
    /// `zed.dx.media_tool.plan.v1` object, plan receipt, or stringified JSON.
    pub media_plan: Value,
    /// Explicit approval flag for the future no-shell media runner.
    pub approve_runner: bool,
    /// Require the local source file to exist before the gate can become runner-ready.
    pub require_existing_source: bool,
    /// Persist the runner gate to a managed receipt file after authorization.
    pub write_runner_gate_receipt: bool,
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data.
    pub artifact_root_mode: DxMediaToolRunnerGateArtifactRootMode,
}

impl Default for DxMediaToolRunnerGateToolInput {
    fn default() -> Self {
        Self {
            media_plan: Value::Null,
            approve_runner: false,
            require_existing_source: true,
            write_runner_gate_receipt: false,
            artifact_root_mode: DxMediaToolRunnerGateArtifactRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxMediaToolRunnerGateArtifactRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxMediaToolRunnerGateTool {
    project: Entity<Project>,
}

impl DxMediaToolRunnerGateTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxMediaToolRunnerGateTool {
    type Input = DxMediaToolRunnerGateToolInput;
    type Output = String;

    const NAME: &'static str = "gate_dx_media_tool_runner";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            if input.approve_runner {
                "Gate approved DX media runner".into()
            } else {
                "Gate DX media runner".into()
            }
        } else {
            "Gate DX media runner".into()
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
            if input.media_plan.is_null() {
                return Err("DX media runner gate needs a media plan or plan receipt.".to_string());
            }

            let artifact_target = {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxMediaToolRunnerGateArtifactTarget::new(project_root, input.artifact_root_mode)
            };

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![
                    format!("approve_runner={}", input.approve_runner),
                    format!("require_existing_source={}", input.require_existing_source),
                ];
                if input.write_runner_gate_receipt {
                    permission_values.push(path_string(&artifact_target.latest_path));
                    permission_values.push(path_string(&artifact_target.archive_path));
                }
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;
            let write_runner_gate_receipt = input.write_runner_gate_receipt;
            let mut response = dx_media_tool_runner_gate::build_dx_media_tool_runner_gate(
                artifact_target.request(input),
            )?;

            if write_runner_gate_receipt {
                response.runner_receipt =
                    Some(artifact_target.write_receipt(&response).map_err(|error| {
                        format!("Failed to write DX media runner gate receipt: {error}")
                    })?);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Gated DX media runner: {}",
                response.validation.status
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX media runner gate response: {error}")
            })
        })
    }
}

struct DxMediaToolRunnerGateArtifactTarget {
    root_mode: DxMediaToolRunnerGateArtifactRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxMediaToolRunnerGateArtifactTarget {
    fn new(
        project_root: Option<PathBuf>,
        root_mode: DxMediaToolRunnerGateArtifactRootMode,
    ) -> Self {
        let use_workspace = matches!(root_mode, DxMediaToolRunnerGateArtifactRootMode::Workspace)
            && project_root.is_some();
        let allowed_root = if use_workspace {
            project_root
                .as_ref()
                .expect("workspace root checked above")
                .join("tools")
                .join("dx-media")
        } else {
            data_dir().join("dx-media")
        };
        let receipt_dir = allowed_root.join("runner-gates");
        let latest_path = receipt_dir.join(DX_MEDIA_RUNNER_GATE_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-media-runner-gate-{}.json",
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
        input: DxMediaToolRunnerGateToolInput,
    ) -> dx_media_tool_runner_gate::DxMediaToolRunnerGateRequest {
        dx_media_tool_runner_gate::DxMediaToolRunnerGateRequest {
            media_plan: input.media_plan,
            approve_runner: input.approve_runner,
            require_existing_source: input.require_existing_source,
            root_mode: self.root_mode_label().to_string(),
        }
    }

    fn write_receipt(
        &self,
        response: &dx_media_tool_runner_gate::DxMediaToolRunnerGate,
    ) -> Result<dx_media_tool_runner_gate::DxMediaToolRunnerGateReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_media_tool_runner_gate::DX_MEDIA_TOOL_RUNNER_GATE_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxMediaToolRunnerGateTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "runner_gate": response,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "runs_ffmpeg": false,
                "runs_ffprobe": false,
                "runs_shell": false,
                "downloads_remote_media": false,
                "deletes_files": false,
                "overwrites_outputs": false,
                "writes_media_outputs": false,
                "dispatches_browser_input": false,
            },
            "next_action": "Use this runner gate receipt to implement the future no-shell media runner and produced-file receipt writer."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("Failed to serialize media runner gate receipt: {error}"))?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX media runner gate receipt directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX media latest runner gate receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX media runner gate receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(dx_media_tool_runner_gate::DxMediaToolRunnerGateReceipt {
            schema: dx_media_tool_runner_gate::DX_MEDIA_TOOL_RUNNER_GATE_RECEIPT_SCHEMA,
            status: "written",
            root_mode: self.root_mode_label().to_string(),
            receipt_dir: path_string(&self.receipt_dir),
            latest_path: path_string(&self.latest_path),
            archive_path: path_string(&self.archive_path),
            written_bytes: receipt_json.len(),
            runner_gate_schema: response.schema,
            action: response.plan.action.clone(),
            runner_ready: response.validation.runner_ready,
            planned_output_count: response.validation.planned_outputs.len(),
            next_action: "Use the latest runner gate receipt to add the future no-shell ffmpeg runner after review.".to_string(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX media runner gate receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxMediaToolRunnerGateArtifactRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxMediaToolRunnerGateArtifactRootMode::Workspace => "zed_data_fallback",
            DxMediaToolRunnerGateArtifactRootMode::ZedData => "zed_data",
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
