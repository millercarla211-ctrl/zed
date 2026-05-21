use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_forge_restore_target_plan};
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

const DX_FORGE_RESTORE_TARGET_PLAN_LATEST_FILE_NAME: &str =
    "latest-dx-forge-restore-target-plan-receipt.json";

/// Plan a future DX Forge restore-to-target operation without mutating the target.
///
/// This tool reads restore approval evidence and writes only managed dry-run plan receipts. It
/// never restores into live paths, overwrites, deletes, shells out, runs Forge/zstd, or starts
/// validation.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxForgeRestoreTargetPlanToolInput {
    /// `zed.dx.forge.restore_approval.v1` object, approval receipt, or stringified JSON.
    pub restore_approval: Value,
    /// Optional live target path override for the future restore-to-target request.
    pub target_path: Option<String>,
    /// Require the approval receipt to be approval-ready.
    pub require_approval_ready: bool,
    /// Require rollback evidence to be verified in the approval receipt.
    pub require_rollback_verified: bool,
    /// Require the managed restore preview directory to still exist on disk.
    pub require_preview_exists: bool,
    /// Persist the dry-run plan to a managed receipt root after authorization.
    pub write_plan_receipt: bool,
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data.
    pub receipt_root_mode: DxForgeRestoreTargetPlanReceiptRootMode,
}

impl Default for DxForgeRestoreTargetPlanToolInput {
    fn default() -> Self {
        Self {
            restore_approval: Value::Null,
            target_path: None,
            require_approval_ready: true,
            require_rollback_verified: true,
            require_preview_exists: true,
            write_plan_receipt: true,
            receipt_root_mode: DxForgeRestoreTargetPlanReceiptRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxForgeRestoreTargetPlanReceiptRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxForgeRestoreTargetPlanTool {
    project: Entity<Project>,
}

impl DxForgeRestoreTargetPlanTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxForgeRestoreTargetPlanTool {
    type Input = DxForgeRestoreTargetPlanToolInput;
    type Output = String;

    const NAME: &'static str = "plan_dx_forge_restore_target";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Edit
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Plan DX Forge restore target".into()
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
            let receipt_target = input.write_plan_receipt.then(|| {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxForgeRestoreTargetPlanReceiptTarget::new(project_root, input.receipt_root_mode)
            });

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![
                    format!("require_approval_ready={}", input.require_approval_ready),
                    format!(
                        "require_rollback_verified={}",
                        input.require_rollback_verified
                    ),
                    format!("require_preview_exists={}", input.require_preview_exists),
                ];
                if let Some(target_path) = input.target_path.as_deref() {
                    permission_values.push(format!("target_path={}", target_path.trim()));
                }
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
                .map(DxForgeRestoreTargetPlanReceiptTarget::root_mode_label)
                .unwrap_or_else(|| input.receipt_root_mode.label())
                .to_string();
            let mut response = dx_forge_restore_target_plan::build_dx_forge_restore_target_plan(
                dx_forge_restore_target_plan::DxForgeRestoreTargetPlanRequest {
                    restore_approval: input.restore_approval,
                    target_path: input.target_path,
                    require_approval_ready: input.require_approval_ready,
                    require_rollback_verified: input.require_rollback_verified,
                    require_preview_exists: input.require_preview_exists,
                    root_mode,
                    generated_at_ms: current_epoch_millis(),
                },
            );

            if let Some(receipt_target) = receipt_target {
                response.plan_receipt =
                    Some(receipt_target.write_receipt(&response).map_err(|error| {
                        format!("Failed to write DX Forge restore target plan receipt: {error}")
                    })?);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Planned DX Forge restore target: {}",
                response.validation.status
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX Forge restore target plan response: {error}")
            })
        })
    }
}

struct DxForgeRestoreTargetPlanReceiptTarget {
    root_mode: DxForgeRestoreTargetPlanReceiptRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxForgeRestoreTargetPlanReceiptTarget {
    fn new(
        project_root: Option<PathBuf>,
        root_mode: DxForgeRestoreTargetPlanReceiptRootMode,
    ) -> Self {
        let allowed_root = match (root_mode, project_root.as_ref()) {
            (DxForgeRestoreTargetPlanReceiptRootMode::Workspace, Some(root)) => {
                root.join("tools").join("dx-forge")
            }
            _ => data_dir().join("dx-forge"),
        };
        let receipt_dir = allowed_root.join("restore-target-plans");
        let latest_path = receipt_dir.join(DX_FORGE_RESTORE_TARGET_PLAN_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-forge-restore-target-plan-{}.json",
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
        response: &dx_forge_restore_target_plan::DxForgeRestoreTargetPlan,
    ) -> Result<dx_forge_restore_target_plan::DxForgeRestoreTargetPlanReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_forge_restore_target_plan::DX_FORGE_RESTORE_TARGET_PLAN_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxForgeRestoreTargetPlanTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "restore_target_plan": response,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "writes_managed_receipts_only": true,
                "mutates_target_path": false,
                "overwrites_target_files": false,
                "deletes_files": false,
                "runs_shell": false,
                "runs_external_processes": false,
                "runs_forge_binary": false,
                "runs_zstd_binary": false,
                "starts_local_servers": false,
                "dispatches_browser_input": false,
            },
            "next_action": "Use this dry-run plan only as evidence for a future governed restore-to-target window."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt).map_err(|error| {
            format!("Failed to serialize Forge restore target plan receipt: {error}")
        })?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX Forge restore target plan directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX Forge latest restore target plan receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX Forge restore target plan receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(
            dx_forge_restore_target_plan::DxForgeRestoreTargetPlanReceipt {
                schema: dx_forge_restore_target_plan::DX_FORGE_RESTORE_TARGET_PLAN_RECEIPT_SCHEMA,
                status: "written",
                root_mode: self.root_mode_label().to_string(),
                receipt_dir: path_string(&self.receipt_dir),
                latest_path: path_string(&self.latest_path),
                archive_path: path_string(&self.archive_path),
                written_bytes: receipt_json.len(),
                plan_ready: response.validation.plan_ready,
                target_path: response.request.target_path.clone(),
                restore_destination_root: response.approval.restore_destination_root.clone(),
                blocker_count: response.validation.blocker_count,
                next_action: response.next_action.clone(),
            },
        )
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX Forge restore target plan receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxForgeRestoreTargetPlanReceiptRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxForgeRestoreTargetPlanReceiptRootMode::Workspace => "zed_data_fallback",
            DxForgeRestoreTargetPlanReceiptRootMode::ZedData => "zed_data",
        }
    }
}

impl DxForgeRestoreTargetPlanReceiptRootMode {
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
