use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_forge_backup_executor};
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

const DX_FORGE_EXECUTION_LATEST_FILE_NAME: &str = "latest-dx-forge-backup-execution-receipt.json";

/// Execute an approved Forge backup runner gate with native zstd backup and optional quarantine.
///
/// This tool writes a compressed backup bundle, a manifest, and an execution receipt. For delete
/// policies it can move the target into the planned quarantine path after the backup. It never
/// permanently deletes, shells out, overwrites planned artifacts, or runs external Forge/zstd
/// commands.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxForgeBackupExecutorToolInput {
    /// `zed.dx.forge.backup_runner_gate.v1` object, gate receipt, or stringified JSON.
    pub runner_gate: Value,
    /// Explicit approval flag for writing the backup bundle and manifest.
    pub approve_execution: bool,
    /// For delete operations, move the backed-up target into the planned quarantine path.
    pub apply_quarantine_after_backup: bool,
    /// Require the managed execution receipt. Must stay true for approved execution.
    pub require_execution_receipt: bool,
    /// Prefer workspace-local execution receipts under `<workspace>/tools`; falls back to Zed data.
    pub artifact_root_mode: DxForgeBackupExecutorArtifactRootMode,
}

impl Default for DxForgeBackupExecutorToolInput {
    fn default() -> Self {
        Self {
            runner_gate: Value::Null,
            approve_execution: false,
            apply_quarantine_after_backup: false,
            require_execution_receipt: true,
            artifact_root_mode: DxForgeBackupExecutorArtifactRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxForgeBackupExecutorArtifactRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxForgeBackupExecutorTool {
    project: Entity<Project>,
}

impl DxForgeBackupExecutorTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxForgeBackupExecutorTool {
    type Input = DxForgeBackupExecutorToolInput;
    type Output = String;

    const NAME: &'static str = "execute_dx_forge_backup";

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
                "Execute approved DX Forge backup".into()
            } else {
                "Prepare DX Forge backup execution".into()
            }
        } else {
            "Execute DX Forge backup".into()
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
                return Err(
                    "DX Forge backup execution needs a runner gate or gate receipt.".to_string(),
                );
            }
            if input.approve_execution && !input.require_execution_receipt {
                return Err(
                    "Approved DX Forge backup execution requires an execution receipt.".to_string(),
                );
            }

            let artifact_target = {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxForgeBackupExecutorArtifactTarget::new(project_root, input.artifact_root_mode)
            };

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![
                    format!("approve_execution={}", input.approve_execution),
                    format!(
                        "apply_quarantine_after_backup={}",
                        input.apply_quarantine_after_backup
                    ),
                    format!(
                        "require_execution_receipt={}",
                        input.require_execution_receipt
                    ),
                    path_string(&artifact_target.receipt_dir),
                    path_string(&artifact_target.latest_path),
                    path_string(&artifact_target.archive_path),
                ];
                if input.apply_quarantine_after_backup {
                    permission_values.push("quarantine_after_backup=true".to_string());
                }
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;
            let mut response =
                dx_forge_backup_executor::execute_dx_forge_backup(artifact_target.request(input))?;

            response.execution_receipt =
                Some(artifact_target.write_receipt(&response).map_err(|error| {
                    format!("Failed to write DX Forge backup execution receipt: {error}")
                })?);

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Executed DX Forge backup: {}",
                response.execution.status
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX Forge backup execution response: {error}")
            })
        })
    }
}

struct DxForgeBackupExecutorArtifactTarget {
    root_mode: DxForgeBackupExecutorArtifactRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxForgeBackupExecutorArtifactTarget {
    fn new(
        project_root: Option<PathBuf>,
        root_mode: DxForgeBackupExecutorArtifactRootMode,
    ) -> Self {
        let use_workspace = matches!(root_mode, DxForgeBackupExecutorArtifactRootMode::Workspace)
            && project_root.is_some();
        let allowed_root = if use_workspace {
            project_root
                .as_ref()
                .expect("workspace root checked above")
                .join("tools")
                .join("dx-forge")
        } else {
            data_dir().join("dx-forge")
        };
        let receipt_dir = allowed_root.join("executions");
        let latest_path = receipt_dir.join(DX_FORGE_EXECUTION_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-forge-backup-execution-{}.json",
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
        input: DxForgeBackupExecutorToolInput,
    ) -> dx_forge_backup_executor::DxForgeBackupExecutionRequest {
        dx_forge_backup_executor::DxForgeBackupExecutionRequest {
            runner_gate: input.runner_gate,
            approve_execution: input.approve_execution,
            apply_quarantine_after_backup: input.apply_quarantine_after_backup,
            require_execution_receipt: input.require_execution_receipt,
            root_mode: self.root_mode_label().to_string(),
        }
    }

    fn write_receipt(
        &self,
        response: &dx_forge_backup_executor::DxForgeBackupExecution,
    ) -> Result<dx_forge_backup_executor::DxForgeBackupExecutionReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_forge_backup_executor::DX_FORGE_BACKUP_EXECUTION_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxForgeBackupExecutorTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "backup_execution": response,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "runs_shell": false,
                "runs_external_process": false,
                "runs_forge_binary": false,
                "runs_zstd_binary": false,
                "uses_native_zstd_library": true,
                "deletes_files": false,
                "permanent_delete_allowed": false,
                "overwrites_outputs": false,
            },
            "next_action": "Use this execution receipt with execute_dx_forge_restore for a managed restore preview, audit, or future Forge panel history."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt).map_err(|error| {
            format!("Failed to serialize Forge backup execution receipt: {error}")
        })?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX Forge backup execution receipt directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX Forge latest backup execution receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX Forge backup execution receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(dx_forge_backup_executor::DxForgeBackupExecutionReceipt {
            schema: dx_forge_backup_executor::DX_FORGE_BACKUP_EXECUTION_RECEIPT_SCHEMA,
            status: "written",
            root_mode: self.root_mode_label().to_string(),
            receipt_dir: path_string(&self.receipt_dir),
            latest_path: path_string(&self.latest_path),
            archive_path: path_string(&self.archive_path),
            written_bytes: receipt_json.len(),
            execution_schema: response.schema,
            operation: response.gate.operation.clone(),
            archive_path_written: response.execution.archive_path.clone(),
            manifest_path_written: response.execution.manifest_path.clone(),
            quarantine_path: response.execution.quarantine_path.clone(),
            target_mutation_applied: response.execution.target_mutation_applied,
            next_action:
                "Use the latest Forge backup execution receipt with execute_dx_forge_restore before adding restore UI and Forge panel history."
                    .to_string(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX Forge backup execution receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxForgeBackupExecutorArtifactRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxForgeBackupExecutorArtifactRootMode::Workspace => "zed_data_fallback",
            DxForgeBackupExecutorArtifactRootMode::ZedData => "zed_data",
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
