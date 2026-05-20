use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_forge_backup_runner_gate};
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

const DX_FORGE_RUNNER_GATE_LATEST_FILE_NAME: &str =
    "latest-dx-forge-backup-runner-gate-receipt.json";

/// Validate a Forge safety policy before any future backup/quarantine runner executes.
///
/// This gate accepts `zed.dx.forge.safety_policy.v1` policies or policy receipts and can write a
/// managed readiness receipt. It does not run Forge, zstd, shell commands, deletes, moves,
/// overwrites, archives, or backup writes.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxForgeBackupRunnerGateToolInput {
    /// `zed.dx.forge.safety_policy.v1` object, policy receipt, or stringified JSON.
    pub forge_safety_policy: Value,
    /// Explicit approval flag for the future Forge/zstd backup runner.
    pub approve_runner: bool,
    /// Require the protected target path to exist before the runner can become ready.
    pub require_existing_target: bool,
    /// Persist the runner gate to a managed receipt file after authorization.
    pub write_runner_gate_receipt: bool,
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data.
    pub artifact_root_mode: DxForgeBackupRunnerGateArtifactRootMode,
}

impl Default for DxForgeBackupRunnerGateToolInput {
    fn default() -> Self {
        Self {
            forge_safety_policy: Value::Null,
            approve_runner: false,
            require_existing_target: true,
            write_runner_gate_receipt: false,
            artifact_root_mode: DxForgeBackupRunnerGateArtifactRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxForgeBackupRunnerGateArtifactRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxForgeBackupRunnerGateTool {
    project: Entity<Project>,
}

impl DxForgeBackupRunnerGateTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxForgeBackupRunnerGateTool {
    type Input = DxForgeBackupRunnerGateToolInput;
    type Output = String;

    const NAME: &'static str = "gate_dx_forge_backup_runner";

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
                "Gate approved DX Forge backup runner".into()
            } else {
                "Gate DX Forge backup runner".into()
            }
        } else {
            "Gate DX Forge backup runner".into()
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
            if input.forge_safety_policy.is_null() {
                return Err(
                    "DX Forge backup runner gate needs a safety policy or policy receipt."
                        .to_string(),
                );
            }

            let artifact_target = {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxForgeBackupRunnerGateArtifactTarget::new(project_root, input.artifact_root_mode)
            };

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![
                    format!("approve_runner={}", input.approve_runner),
                    format!("require_existing_target={}", input.require_existing_target),
                    path_string(&artifact_target.receipt_dir),
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
            let mut response = dx_forge_backup_runner_gate::build_dx_forge_backup_runner_gate(
                artifact_target.request(input),
            )?;

            if write_runner_gate_receipt {
                response.runner_receipt =
                    Some(artifact_target.write_receipt(&response).map_err(|error| {
                        format!("Failed to write DX Forge backup runner gate receipt: {error}")
                    })?);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Gated DX Forge backup runner: {}",
                response.validation.status
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX Forge backup runner gate response: {error}")
            })
        })
    }
}

struct DxForgeBackupRunnerGateArtifactTarget {
    root_mode: DxForgeBackupRunnerGateArtifactRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxForgeBackupRunnerGateArtifactTarget {
    fn new(
        project_root: Option<PathBuf>,
        root_mode: DxForgeBackupRunnerGateArtifactRootMode,
    ) -> Self {
        let use_workspace = matches!(
            root_mode,
            DxForgeBackupRunnerGateArtifactRootMode::Workspace
        ) && project_root.is_some();
        let allowed_root = if use_workspace {
            project_root
                .as_ref()
                .expect("workspace root checked above")
                .join("tools")
                .join("dx-forge")
        } else {
            data_dir().join("dx-forge")
        };
        let receipt_dir = allowed_root.join("runner-gates");
        let latest_path = receipt_dir.join(DX_FORGE_RUNNER_GATE_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-forge-backup-runner-gate-{}.json",
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
        input: DxForgeBackupRunnerGateToolInput,
    ) -> dx_forge_backup_runner_gate::DxForgeBackupRunnerGateRequest {
        dx_forge_backup_runner_gate::DxForgeBackupRunnerGateRequest {
            forge_safety_policy: input.forge_safety_policy,
            approve_runner: input.approve_runner,
            require_existing_target: input.require_existing_target,
            root_mode: self.root_mode_label().to_string(),
        }
    }

    fn write_receipt(
        &self,
        response: &dx_forge_backup_runner_gate::DxForgeBackupRunnerGate,
    ) -> Result<dx_forge_backup_runner_gate::DxForgeBackupRunnerGateReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_forge_backup_runner_gate::DX_FORGE_BACKUP_RUNNER_GATE_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxForgeBackupRunnerGateTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "runner_gate": response,
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
            "next_action": "Use this runner gate receipt to implement the future Forge/zstd backup runner and restore receipt writer."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt).map_err(|error| {
            format!("Failed to serialize Forge backup runner gate receipt: {error}")
        })?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX Forge backup runner receipt directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX Forge latest backup runner gate receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX Forge backup runner gate receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(dx_forge_backup_runner_gate::DxForgeBackupRunnerGateReceipt {
            schema: dx_forge_backup_runner_gate::DX_FORGE_BACKUP_RUNNER_GATE_RECEIPT_SCHEMA,
            status: "written",
            root_mode: self.root_mode_label().to_string(),
            receipt_dir: path_string(&self.receipt_dir),
            latest_path: path_string(&self.latest_path),
            archive_path: path_string(&self.archive_path),
            written_bytes: receipt_json.len(),
            runner_gate_schema: response.schema,
            operation: response.policy.operation.clone(),
            runner_ready: response.validation.runner_ready,
            planned_archive_path: response.policy.planned_archive_path.clone(),
            planned_manifest_path: response.policy.planned_manifest_path.clone(),
            planned_quarantine_path: response.policy.planned_quarantine_path.clone(),
            next_action: "Use the latest Forge backup runner gate receipt to add the future archive/quarantine executor after review.".to_string(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX Forge backup runner gate receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxForgeBackupRunnerGateArtifactRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxForgeBackupRunnerGateArtifactRootMode::Workspace => "zed_data_fallback",
            DxForgeBackupRunnerGateArtifactRootMode::ZedData => "zed_data",
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
