use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_forge_restore_approval};
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

const DX_FORGE_RESTORE_APPROVAL_LATEST_FILE_NAME: &str =
    "latest-dx-forge-restore-approval-receipt.json";

/// Capture operator approval evidence for a future Forge restore-to-target flow.
///
/// This tool writes only managed approval receipts. It does not restore into target paths,
/// overwrite files, delete files, shell out, run Forge, run zstd, or start local validation.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxForgeRestoreApprovalToolInput {
    /// `zed.dx.forge.restore_execution.v1` object, restore receipt, or stringified JSON.
    pub restore_execution: Value,
    /// Explicit live target path for the future restore-to-target request.
    pub target_path: Option<String>,
    /// Operator confirmation that the restore-to-target request is approved for later execution.
    pub operator_approval: bool,
    /// Confirmation that rollback evidence has been reviewed.
    pub rollback_verified: bool,
    /// Explicit approval marker for future overwrite risk. This tool still never overwrites.
    pub overwrite_approved: bool,
    /// Evidence lines such as preview path, hash, diff review, backup id, or operator note.
    pub evidence: Vec<String>,
    /// Known blockers that should prevent a restore-to-target-ready approval receipt.
    pub blockers: Vec<String>,
    /// Optional human-readable approval note.
    pub approval_note: Option<String>,
    /// Persist approval receipts to a managed receipt root after authorization.
    pub write_approval_receipt: bool,
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data.
    pub receipt_root_mode: DxForgeRestoreApprovalReceiptRootMode,
}

impl Default for DxForgeRestoreApprovalToolInput {
    fn default() -> Self {
        Self {
            restore_execution: Value::Null,
            target_path: None,
            operator_approval: false,
            rollback_verified: false,
            overwrite_approved: false,
            evidence: Vec::new(),
            blockers: Vec::new(),
            approval_note: None,
            write_approval_receipt: true,
            receipt_root_mode: DxForgeRestoreApprovalReceiptRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxForgeRestoreApprovalReceiptRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxForgeRestoreApprovalTool {
    project: Entity<Project>,
}

impl DxForgeRestoreApprovalTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxForgeRestoreApprovalTool {
    type Input = DxForgeRestoreApprovalToolInput;
    type Output = String;

    const NAME: &'static str = "capture_dx_forge_restore_approval";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Edit
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            if input.operator_approval {
                return "Capture approved DX Forge restore evidence".into();
            }
        }

        "Capture DX Forge restore approval".into()
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
            let receipt_target = input.write_approval_receipt.then(|| {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxForgeRestoreApprovalReceiptTarget::new(project_root, input.receipt_root_mode)
            });

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![
                    format!("operator_approval={}", input.operator_approval),
                    format!("rollback_verified={}", input.rollback_verified),
                    format!("overwrite_approved={}", input.overwrite_approved),
                    format!("evidence_count={}", input.evidence.len()),
                    format!("blocker_count={}", input.blockers.len()),
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
                .map(DxForgeRestoreApprovalReceiptTarget::root_mode_label)
                .unwrap_or_else(|| input.receipt_root_mode.label())
                .to_string();
            let mut response = dx_forge_restore_approval::build_dx_forge_restore_approval(
                dx_forge_restore_approval::DxForgeRestoreApprovalRequest {
                    restore_execution: input.restore_execution,
                    target_path: input.target_path,
                    operator_approval: input.operator_approval,
                    rollback_verified: input.rollback_verified,
                    overwrite_approved: input.overwrite_approved,
                    evidence: input.evidence,
                    blockers: input.blockers,
                    approval_note: input.approval_note,
                    root_mode,
                    generated_at_ms: current_epoch_millis(),
                },
            );

            if let Some(receipt_target) = receipt_target {
                response.approval_receipt =
                    Some(receipt_target.write_receipt(&response).map_err(|error| {
                        format!("Failed to write DX Forge restore approval receipt: {error}")
                    })?);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Captured DX Forge restore approval: {}",
                response.validation.status
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX Forge restore approval response: {error}")
            })
        })
    }
}

struct DxForgeRestoreApprovalReceiptTarget {
    root_mode: DxForgeRestoreApprovalReceiptRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxForgeRestoreApprovalReceiptTarget {
    fn new(
        project_root: Option<PathBuf>,
        root_mode: DxForgeRestoreApprovalReceiptRootMode,
    ) -> Self {
        let allowed_root = match (root_mode, project_root.as_ref()) {
            (DxForgeRestoreApprovalReceiptRootMode::Workspace, Some(root)) => {
                root.join("tools").join("dx-forge")
            }
            _ => data_dir().join("dx-forge"),
        };
        let receipt_dir = allowed_root.join("restore-approvals");
        let latest_path = receipt_dir.join(DX_FORGE_RESTORE_APPROVAL_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-forge-restore-approval-{}.json",
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
        response: &dx_forge_restore_approval::DxForgeRestoreApproval,
    ) -> Result<dx_forge_restore_approval::DxForgeRestoreApprovalReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_forge_restore_approval::DX_FORGE_RESTORE_APPROVAL_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxForgeRestoreApprovalTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "restore_approval": response,
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
            "next_action": "Use this approval receipt only as evidence for a future governed restore-to-target flow."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt).map_err(|error| {
            format!("Failed to serialize Forge restore approval receipt: {error}")
        })?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX Forge restore approval directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX Forge latest restore approval receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX Forge restore approval receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(dx_forge_restore_approval::DxForgeRestoreApprovalReceipt {
            schema: dx_forge_restore_approval::DX_FORGE_RESTORE_APPROVAL_RECEIPT_SCHEMA,
            status: "written",
            root_mode: self.root_mode_label().to_string(),
            receipt_dir: path_string(&self.receipt_dir),
            latest_path: path_string(&self.latest_path),
            archive_path: path_string(&self.archive_path),
            written_bytes: receipt_json.len(),
            approval_ready: response.validation.approval_ready,
            target_path: response.request.target_path.clone(),
            restore_destination_root: response.restore.restore_destination_root.clone(),
            blocker_count: response.validation.blocker_count,
            next_action: response.next_action.clone(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX Forge restore approval receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxForgeRestoreApprovalReceiptRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxForgeRestoreApprovalReceiptRootMode::Workspace => "zed_data_fallback",
            DxForgeRestoreApprovalReceiptRootMode::ZedData => "zed_data",
        }
    }
}

impl DxForgeRestoreApprovalReceiptRootMode {
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
