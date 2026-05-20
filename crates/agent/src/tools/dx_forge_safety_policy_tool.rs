use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_forge_safety_policy};
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

const DX_FORGE_POLICY_LATEST_FILE_NAME: &str = "latest-dx-forge-safety-policy-receipt.json";

/// Plan a Forge/zstd backup-first safety policy before risky file operations.
///
/// This tool writes optional policy receipts only. It does not delete, move, overwrite, archive,
/// compress, or run external Forge/zstd commands.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxForgeSafetyPolicyToolInput {
    /// Target path for a risky operation. Relative paths resolve from the active workspace.
    pub target_path: String,
    /// Risky operation to protect: delete, move, overwrite, or cleanup.
    pub operation: Option<String>,
    /// Destination path for move/overwrite operations.
    pub destination_path: Option<String>,
    /// Short reason for the risky operation.
    pub reason: Option<String>,
    /// Explicit approval flag for the future backup/quarantine runner.
    pub approve_policy: bool,
    /// Allow planning outside the active workspace after explicit review.
    pub allow_outside_workspace: bool,
    /// Persist the policy plan to a managed receipt file after authorization.
    pub write_policy_receipt: bool,
    /// Prefer workspace-local receipts/artifacts under `<workspace>/tools`; falls back to Zed data.
    pub artifact_root_mode: DxForgeSafetyPolicyArtifactRootMode,
}

impl Default for DxForgeSafetyPolicyToolInput {
    fn default() -> Self {
        Self {
            target_path: String::new(),
            operation: Some("delete".to_string()),
            destination_path: None,
            reason: None,
            approve_policy: false,
            allow_outside_workspace: false,
            write_policy_receipt: false,
            artifact_root_mode: DxForgeSafetyPolicyArtifactRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxForgeSafetyPolicyArtifactRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxForgeSafetyPolicyTool {
    project: Entity<Project>,
}

impl DxForgeSafetyPolicyTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxForgeSafetyPolicyTool {
    type Input = DxForgeSafetyPolicyToolInput;
    type Output = String;

    const NAME: &'static str = "plan_dx_forge_safety_policy";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            let operation = input.operation.unwrap_or_else(|| "delete".to_string());
            format!("Plan DX Forge safety {}", MarkdownInlineCode(&operation)).into()
        } else {
            "Plan DX Forge safety".into()
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
            if input.target_path.trim().is_empty() {
                return Err("DX Forge safety policy needs a target path.".to_string());
            }

            let artifact_target = {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxForgeSafetyPolicyArtifactTarget::new(project_root, input.artifact_root_mode)
            };

            let authorize = cx.update(|cx| {
                let operation = input
                    .operation
                    .clone()
                    .unwrap_or_else(|| "delete".to_string());
                let mut permission_values = vec![
                    input.target_path.clone(),
                    format!("operation={operation}"),
                    format!("approve_policy={}", input.approve_policy),
                    format!("allow_outside_workspace={}", input.allow_outside_workspace),
                    path_string(&artifact_target.artifact_root),
                ];
                if let Some(destination_path) = &input.destination_path {
                    permission_values.push(destination_path.clone());
                }
                if input.write_policy_receipt {
                    permission_values.push(path_string(&artifact_target.latest_path));
                    permission_values.push(path_string(&artifact_target.archive_path));
                }
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;
            let write_policy_receipt = input.write_policy_receipt;
            let mut response = dx_forge_safety_policy::build_dx_forge_safety_policy(
                artifact_target.request(input),
            )?;

            if write_policy_receipt {
                response.policy_receipt =
                    Some(artifact_target.write_receipt(&response).map_err(|error| {
                        format!("Failed to write DX Forge safety policy receipt: {error}")
                    })?);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Planned DX Forge safety: {}",
                response.policy.status
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX Forge safety policy response: {error}")
            })
        })
    }
}

struct DxForgeSafetyPolicyArtifactTarget {
    root_mode: DxForgeSafetyPolicyArtifactRootMode,
    project_root: Option<PathBuf>,
    artifact_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxForgeSafetyPolicyArtifactTarget {
    fn new(project_root: Option<PathBuf>, root_mode: DxForgeSafetyPolicyArtifactRootMode) -> Self {
        let use_workspace = matches!(root_mode, DxForgeSafetyPolicyArtifactRootMode::Workspace)
            && project_root.is_some();
        let artifact_root = if use_workspace {
            project_root
                .as_ref()
                .expect("workspace root checked above")
                .join("tools")
                .join("dx-forge")
        } else {
            data_dir().join("dx-forge")
        };
        let receipt_dir = artifact_root.join("safety-policies");
        let latest_path = receipt_dir.join(DX_FORGE_POLICY_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-forge-safety-policy-{}.json",
            current_epoch_millis()
        ));

        Self {
            root_mode,
            project_root,
            artifact_root,
            receipt_dir,
            latest_path,
            archive_path,
        }
    }

    fn request(
        &self,
        input: DxForgeSafetyPolicyToolInput,
    ) -> dx_forge_safety_policy::DxForgeSafetyPolicyRequest {
        dx_forge_safety_policy::DxForgeSafetyPolicyRequest {
            target_path: input.target_path,
            operation: input.operation,
            destination_path: input.destination_path,
            reason: input.reason,
            approve_policy: input.approve_policy,
            allow_outside_workspace: input.allow_outside_workspace,
            workspace_root: self.project_root.clone(),
            managed_artifact_root: self.artifact_root.clone(),
            root_mode: self.root_mode_label().to_string(),
        }
    }

    fn write_receipt(
        &self,
        response: &dx_forge_safety_policy::DxForgeSafetyPolicy,
    ) -> Result<dx_forge_safety_policy::DxForgeSafetyPolicyReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_forge_safety_policy::DX_FORGE_SAFETY_POLICY_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxForgeSafetyPolicyTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "artifact_root": path_string(&self.artifact_root),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "forge_safety_policy": response,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "runs_forge": false,
                "runs_zstd": false,
                "runs_shell": false,
                "writes_backup_archive": false,
                "deletes_files": false,
                "moves_files": false,
                "overwrites_files": false,
                "permanent_delete_allowed": false,
            },
            "next_action": "Use this policy receipt to implement the future backup runner before any risky mutation."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("Failed to serialize Forge safety policy receipt: {error}"))?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX Forge safety receipt directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX Forge latest safety receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX Forge safety receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(dx_forge_safety_policy::DxForgeSafetyPolicyReceipt {
            schema: dx_forge_safety_policy::DX_FORGE_SAFETY_POLICY_RECEIPT_SCHEMA,
            status: "written",
            root_mode: self.root_mode_label().to_string(),
            receipt_dir: path_string(&self.receipt_dir),
            latest_path: path_string(&self.latest_path),
            archive_path: path_string(&self.archive_path),
            written_bytes: receipt_json.len(),
            policy_schema: response.schema,
            operation: response.request.operation.clone(),
            policy_status: response.policy.status.clone(),
            next_action: "Use the latest Forge safety policy receipt to add the future zstd backup runner after review.".to_string(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.artifact_root) {
                return Err(format!(
                    "Refusing to write DX Forge safety receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.artifact_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxForgeSafetyPolicyArtifactRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxForgeSafetyPolicyArtifactRootMode::Workspace => "zed_data_fallback",
            DxForgeSafetyPolicyArtifactRootMode::ZedData => "zed_data",
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
