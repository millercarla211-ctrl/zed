use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_media_tool_plan};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, Entity, SharedString, Task};
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

const DX_MEDIA_TOOL_PLAN_LATEST_FILE_NAME: &str = "latest-dx-media-tool-plan-receipt.json";

/// Create a safe ffmpeg/ffprobe media action plan for future Agent execution.
///
/// This tool validates a local media source, prepares no-overwrite argument vectors, and can write
/// a managed receipt. It does not run ffmpeg, ffprobe, downloads, deletes, or shell commands.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxMediaToolPlanToolInput {
    /// Local media path or URL to plan against. Relative paths resolve from the active workspace.
    pub media_source: String,
    /// Action to plan: inspect, extract_audio, or extract_frame.
    pub action: Option<String>,
    /// Optional output format. Audio: wav/mp3/m4a/flac/ogg. Frame: png/jpg/jpeg/webp.
    pub output_format: Option<String>,
    /// Optional start time for extraction, for example 00:00:05.
    pub start_time: Option<String>,
    /// Optional duration for audio extraction, for example 00:00:30.
    pub duration: Option<String>,
    /// Optional timestamp for frame extraction. Defaults to 00:00:01.
    pub frame_timestamp: Option<String>,
    /// Explicit approval flag for the future ffmpeg/ffprobe runner.
    pub approve_media_execution: bool,
    /// Persist the plan to a managed receipt file after authorization.
    pub write_plan_receipt: bool,
    /// Prefer workspace-local receipts/outputs under `<workspace>/tools`; falls back to Zed data.
    pub artifact_root_mode: DxMediaToolPlanArtifactRootMode,
}

impl Default for DxMediaToolPlanToolInput {
    fn default() -> Self {
        Self {
            media_source: String::new(),
            action: Some("inspect".to_string()),
            output_format: None,
            start_time: None,
            duration: None,
            frame_timestamp: None,
            approve_media_execution: false,
            write_plan_receipt: false,
            artifact_root_mode: DxMediaToolPlanArtifactRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxMediaToolPlanArtifactRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxMediaToolPlanTool {
    project: Entity<Project>,
}

impl DxMediaToolPlanTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxMediaToolPlanTool {
    type Input = DxMediaToolPlanToolInput;
    type Output = String;

    const NAME: &'static str = "plan_dx_media_tool";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            let action = input.action.unwrap_or_else(|| "inspect".to_string());
            format!("Plan DX media tool {}", MarkdownInlineCode(&action)).into()
        } else {
            "Plan DX media tool".into()
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
            if input.media_source.trim().is_empty() {
                return Err("DX media tool plan needs a media source path or URL.".to_string());
            }

            let artifact_target = {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxMediaToolPlanArtifactTarget::new(project_root, input.artifact_root_mode)
            };

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![
                    input.media_source.clone(),
                    format!(
                        "action={}",
                        input
                            .action
                            .clone()
                            .unwrap_or_else(|| "inspect".to_string())
                    ),
                    format!("approve_media_execution={}", input.approve_media_execution),
                    path_string(&artifact_target.output_dir),
                ];
                if input.write_plan_receipt {
                    permission_values.push(path_string(&artifact_target.latest_path));
                    permission_values.push(path_string(&artifact_target.archive_path));
                }
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;
            let write_plan_receipt = input.write_plan_receipt;
            let mut response =
                dx_media_tool_plan::build_dx_media_tool_plan(artifact_target.request(input))?;

            if write_plan_receipt {
                response.plan_receipt =
                    Some(artifact_target.write_receipt(&response).map_err(|error| {
                        format!("Failed to write DX media tool plan receipt: {error}")
                    })?);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Planned DX media tool: {}",
                response.action_plan.status
            )));

            serde_json::to_string_pretty(&response)
                .map_err(|error| format!("Failed to serialize DX media tool plan: {error}"))
        })
    }
}

struct DxMediaToolPlanArtifactTarget {
    root_mode: DxMediaToolPlanArtifactRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    plan_dir: PathBuf,
    output_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxMediaToolPlanArtifactTarget {
    fn new(project_root: Option<PathBuf>, root_mode: DxMediaToolPlanArtifactRootMode) -> Self {
        let use_workspace = matches!(root_mode, DxMediaToolPlanArtifactRootMode::Workspace)
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
        let plan_dir = allowed_root.join("plans");
        let output_dir = allowed_root.join("outputs");
        let latest_path = plan_dir.join(DX_MEDIA_TOOL_PLAN_LATEST_FILE_NAME);
        let archive_path = plan_dir.join(format!(
            "dx-media-tool-plan-{}.json",
            current_epoch_millis()
        ));

        Self {
            root_mode,
            project_root,
            allowed_root,
            plan_dir,
            output_dir,
            latest_path,
            archive_path,
        }
    }

    fn request(
        &self,
        input: DxMediaToolPlanToolInput,
    ) -> dx_media_tool_plan::DxMediaToolPlanRequest {
        dx_media_tool_plan::DxMediaToolPlanRequest {
            media_source: input.media_source,
            action: input.action,
            output_format: input.output_format,
            start_time: input.start_time,
            duration: input.duration,
            frame_timestamp: input.frame_timestamp,
            approve_media_execution: input.approve_media_execution,
            workspace_root: self.project_root.clone(),
            managed_output_root: self.allowed_root.clone(),
            root_mode: self.root_mode_label().to_string(),
        }
    }

    fn write_receipt(
        &self,
        response: &dx_media_tool_plan::DxMediaToolPlan,
    ) -> Result<dx_media_tool_plan::DxMediaToolPlanReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_media_tool_plan::DX_MEDIA_TOOL_PLAN_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxMediaToolPlanTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "plan_dir": path_string(&self.plan_dir),
            "output_dir": path_string(&self.output_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "media_plan": response,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "runs_ffmpeg": false,
                "runs_ffprobe": false,
                "runs_shell": false,
                "downloads_remote_media": false,
                "deletes_files": false,
                "overwrites_outputs": false,
                "dispatches_browser_input": false,
            },
            "next_action": "Use this plan receipt with gate_dx_media_tool_runner before wiring any no-shell media runner."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("Failed to serialize media plan receipt: {error}"))?;

        fs::create_dir_all(&self.plan_dir).map_err(|error| {
            format!(
                "Failed to prepare DX media plan receipt directory {}: {error}",
                self.plan_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX media latest plan receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX media plan receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(dx_media_tool_plan::DxMediaToolPlanReceipt {
            schema: dx_media_tool_plan::DX_MEDIA_TOOL_PLAN_RECEIPT_SCHEMA,
            status: "written",
            root_mode: self.root_mode_label().to_string(),
            receipt_dir: path_string(&self.plan_dir),
            latest_path: path_string(&self.latest_path),
            archive_path: path_string(&self.archive_path),
            written_bytes: receipt_json.len(),
            plan_schema: response.schema,
            action: response.action_plan.action.clone(),
            planned_output_count: response.action_plan.planned_outputs.len(),
            next_action: "Use the latest media plan receipt with gate_dx_media_tool_runner before implementing the future no-shell ffmpeg runner.".to_string(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        for path in [
            &self.plan_dir,
            &self.output_dir,
            &self.latest_path,
            &self.archive_path,
        ] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX media plan receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxMediaToolPlanArtifactRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxMediaToolPlanArtifactRootMode::Workspace => "zed_data_fallback",
            DxMediaToolPlanArtifactRootMode::ZedData => "zed_data",
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
