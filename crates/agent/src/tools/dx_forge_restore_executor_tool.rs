use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_forge_restore_executor};
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

const DX_FORGE_RESTORE_LATEST_FILE_NAME: &str = "latest-dx-forge-restore-execution-receipt.json";

/// Restore a Forge backup into a managed preview directory and write a restore receipt.
///
/// This tool decodes the native zstd backup bundle, verifies the manifest and entry hashes, and
/// writes restored files only under a managed restore preview root. It never overwrites live target
/// files, shells out, runs Forge/zstd binaries, or performs permanent deletes.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxForgeRestoreExecutorToolInput {
    /// `zed.dx.forge.backup_execution.v1` object, execution receipt, or stringified JSON.
    pub backup_execution: Value,
    /// Explicit approval flag for writing the managed restore preview.
    pub approve_restore: bool,
    /// Require the managed restore receipt. Must stay true for approved execution.
    pub require_restore_receipt: bool,
    /// Prefer workspace-local restore previews/receipts under `<workspace>/tools`; falls back to Zed data.
    pub artifact_root_mode: DxForgeRestoreExecutorArtifactRootMode,
}

impl Default for DxForgeRestoreExecutorToolInput {
    fn default() -> Self {
        Self {
            backup_execution: Value::Null,
            approve_restore: false,
            require_restore_receipt: true,
            artifact_root_mode: DxForgeRestoreExecutorArtifactRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxForgeRestoreExecutorArtifactRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxForgeRestoreExecutorTool {
    project: Entity<Project>,
}

impl DxForgeRestoreExecutorTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxForgeRestoreExecutorTool {
    type Input = DxForgeRestoreExecutorToolInput;
    type Output = String;

    const NAME: &'static str = "execute_dx_forge_restore";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            if input.approve_restore {
                "Execute approved DX Forge restore".into()
            } else {
                "Prepare DX Forge restore".into()
            }
        } else {
            "Execute DX Forge restore".into()
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
            if input.backup_execution.is_null() {
                return Err(
                    "DX Forge restore needs a backup execution or execution receipt.".to_string(),
                );
            }
            if input.approve_restore && !input.require_restore_receipt {
                return Err("Approved DX Forge restore requires a restore receipt.".to_string());
            }

            let artifact_target = {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxForgeRestoreExecutorArtifactTarget::new(project_root, input.artifact_root_mode)
            };

            let authorize = cx.update(|cx| {
                let permission_values = vec![
                    format!("approve_restore={}", input.approve_restore),
                    format!("require_restore_receipt={}", input.require_restore_receipt),
                    path_string(&artifact_target.restore_destination_root),
                    path_string(&artifact_target.receipt_dir),
                    path_string(&artifact_target.latest_path),
                    path_string(&artifact_target.archive_path),
                ];
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;
            let mut response = dx_forge_restore_executor::execute_dx_forge_restore(
                artifact_target.request(input),
            )?;

            response.restore_receipt =
                Some(artifact_target.write_receipt(&response).map_err(|error| {
                    format!("Failed to write DX Forge restore execution receipt: {error}")
                })?);

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Executed DX Forge restore: {}",
                response.restore.status
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX Forge restore execution response: {error}")
            })
        })
    }
}

struct DxForgeRestoreExecutorArtifactTarget {
    root_mode: DxForgeRestoreExecutorArtifactRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    restore_destination_root: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxForgeRestoreExecutorArtifactTarget {
    fn new(
        project_root: Option<PathBuf>,
        root_mode: DxForgeRestoreExecutorArtifactRootMode,
    ) -> Self {
        let use_workspace = matches!(root_mode, DxForgeRestoreExecutorArtifactRootMode::Workspace)
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
        let receipt_dir = allowed_root.join("restores");
        let restore_id = format!("dx-forge-restore-{}", current_epoch_millis());
        let restore_destination_root = receipt_dir.join(&restore_id).join("preview");
        let latest_path = receipt_dir.join(DX_FORGE_RESTORE_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!("{restore_id}.json"));

        Self {
            root_mode,
            project_root,
            allowed_root,
            receipt_dir,
            restore_destination_root,
            latest_path,
            archive_path,
        }
    }

    fn request(
        &self,
        input: DxForgeRestoreExecutorToolInput,
    ) -> dx_forge_restore_executor::DxForgeRestoreExecutionRequest {
        dx_forge_restore_executor::DxForgeRestoreExecutionRequest {
            backup_execution: input.backup_execution,
            approve_restore: input.approve_restore,
            require_restore_receipt: input.require_restore_receipt,
            restore_destination_root: self.restore_destination_root.clone(),
            root_mode: self.root_mode_label().to_string(),
        }
    }

    fn write_receipt(
        &self,
        response: &dx_forge_restore_executor::DxForgeRestoreExecution,
    ) -> Result<dx_forge_restore_executor::DxForgeRestoreExecutionReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_forge_restore_executor::DX_FORGE_RESTORE_EXECUTION_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxForgeRestoreExecutorTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "restore_destination_root": path_string(&self.restore_destination_root),
            "restore_execution": response,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "writes_restore_preview_only": true,
                "runs_shell": false,
                "runs_external_process": false,
                "runs_forge_binary": false,
                "runs_zstd_binary": false,
                "uses_native_zstd_library": true,
                "deletes_files": false,
                "permanent_delete_allowed": false,
                "overwrites_live_target": false,
                "target_mutation_applied": false,
            },
            "next_action": "Use this restore receipt for Forge panel history, audit, or a future explicit restore-to-target flow."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt).map_err(|error| {
            format!("Failed to serialize Forge restore execution receipt: {error}")
        })?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX Forge restore receipt directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX Forge latest restore receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX Forge restore receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(dx_forge_restore_executor::DxForgeRestoreExecutionReceipt {
            schema: dx_forge_restore_executor::DX_FORGE_RESTORE_EXECUTION_RECEIPT_SCHEMA,
            status: "written",
            root_mode: self.root_mode_label().to_string(),
            receipt_dir: path_string(&self.receipt_dir),
            latest_path: path_string(&self.latest_path),
            archive_path: path_string(&self.archive_path),
            written_bytes: receipt_json.len(),
            restore_schema: response.schema,
            backup_archive_path: response.backup.archive_path.clone(),
            backup_manifest_path: response.backup.manifest_path.clone(),
            restore_destination_root: response.restore.restore_destination_root.clone(),
            restored_file_count: response.restore.restored_file_count,
            restored_directory_count: response.restore.restored_directory_count,
            restored_total_file_bytes: response.restore.restored_total_file_bytes,
            next_action:
                "Use the latest Forge restore execution receipt to populate Forge history and audit UI."
                    .to_string(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        for path in [
            &self.receipt_dir,
            &self.restore_destination_root,
            &self.latest_path,
            &self.archive_path,
        ] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX Forge restore artifact at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxForgeRestoreExecutorArtifactRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxForgeRestoreExecutorArtifactRootMode::Workspace => "zed_data_fallback",
            DxForgeRestoreExecutorArtifactRootMode::ZedData => "zed_data",
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
