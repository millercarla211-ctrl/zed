use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_runtime_proof_plan};
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

const DX_RUNTIME_PROOF_PLAN_LATEST_FILE_NAME: &str = "latest-dx-runtime-proof-plan-receipt.json";

/// Prepare the governed manual runtime proof checklist and optional managed plan receipt.
///
/// This tool does not run runtime validation. It records the exact manual proof contract that must
/// be satisfied before `import_dx_runtime_proof` can mark a runtime-green candidate.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxRuntimeProofPlanToolInput {
    /// Manual final runtime command expected inside the governed validation window.
    pub expected_final_command: Option<String>,
    /// Require git status evidence before runtime proof can be imported as passed.
    pub require_clean_git: bool,
    /// Require `git diff --check` hygiene evidence before runtime proof can be imported as passed.
    pub require_diff_check: bool,
    /// Require visual/window/panel evidence from the runtime session.
    pub require_runtime_visual_evidence: bool,
    /// Require a follow-up managed import receipt via `import_dx_runtime_proof`.
    pub require_runtime_proof_import: bool,
    /// Optional operator notes to carry into the plan receipt.
    pub operator_notes: Vec<String>,
    /// Persist the plan to a managed runtime-proof plan receipt after authorization.
    pub write_runtime_proof_plan_receipt: bool,
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data.
    pub receipt_root_mode: DxRuntimeProofPlanReceiptRootMode,
}

impl Default for DxRuntimeProofPlanToolInput {
    fn default() -> Self {
        Self {
            expected_final_command: Some("just run".to_string()),
            require_clean_git: true,
            require_diff_check: true,
            require_runtime_visual_evidence: true,
            require_runtime_proof_import: true,
            operator_notes: Vec::new(),
            write_runtime_proof_plan_receipt: true,
            receipt_root_mode: DxRuntimeProofPlanReceiptRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxRuntimeProofPlanReceiptRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxRuntimeProofPlanTool {
    project: Entity<Project>,
}

impl DxRuntimeProofPlanTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxRuntimeProofPlanTool {
    type Input = DxRuntimeProofPlanToolInput;
    type Output = String;

    const NAME: &'static str = "plan_dx_runtime_proof";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Edit
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Plan DX runtime proof".into()
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
            let receipt_target = input.write_runtime_proof_plan_receipt.then(|| {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxRuntimeProofPlanReceiptTarget::new(project_root, input.receipt_root_mode)
            });

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![
                    format!(
                        "expected_final_command={}",
                        input
                            .expected_final_command
                            .as_deref()
                            .unwrap_or("just run")
                            .trim()
                    ),
                    format!("require_clean_git={}", input.require_clean_git),
                    format!("require_diff_check={}", input.require_diff_check),
                    format!(
                        "require_runtime_visual_evidence={}",
                        input.require_runtime_visual_evidence
                    ),
                    format!(
                        "require_runtime_proof_import={}",
                        input.require_runtime_proof_import
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
                .map(DxRuntimeProofPlanReceiptTarget::root_mode_label)
                .unwrap_or_else(|| input.receipt_root_mode.label())
                .to_string();
            let mut response = dx_runtime_proof_plan::build_runtime_proof_plan(
                dx_runtime_proof_plan::DxRuntimeProofPlanRequest {
                    expected_final_command: input.expected_final_command,
                    require_clean_git: input.require_clean_git,
                    require_diff_check: input.require_diff_check,
                    require_runtime_visual_evidence: input.require_runtime_visual_evidence,
                    require_runtime_proof_import: input.require_runtime_proof_import,
                    operator_notes: input.operator_notes,
                    root_mode,
                },
            );

            if let Some(receipt_target) = receipt_target {
                response.runtime_proof_plan_receipt =
                    Some(receipt_target.write_receipt(&response).map_err(|error| {
                        format!("Failed to write DX runtime proof plan receipt: {error}")
                    })?);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Planned DX runtime proof: {}",
                response.status.status
            )));

            serde_json::to_string_pretty(&response)
                .map_err(|error| format!("Failed to serialize DX runtime proof plan: {error}"))
        })
    }
}

struct DxRuntimeProofPlanReceiptTarget {
    root_mode: DxRuntimeProofPlanReceiptRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxRuntimeProofPlanReceiptTarget {
    fn new(project_root: Option<PathBuf>, root_mode: DxRuntimeProofPlanReceiptRootMode) -> Self {
        let allowed_root = match (root_mode, project_root.as_ref()) {
            (DxRuntimeProofPlanReceiptRootMode::Workspace, Some(root)) => {
                root.join("tools").join("dx-runtime-proof")
            }
            _ => data_dir().join("dx-runtime-proof"),
        };
        let receipt_dir = allowed_root.join("plans");
        let latest_path = receipt_dir.join(DX_RUNTIME_PROOF_PLAN_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-runtime-proof-plan-{}.json",
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
        response: &dx_runtime_proof_plan::DxRuntimeProofPlan,
    ) -> Result<dx_runtime_proof_plan::DxRuntimeProofPlanReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_runtime_proof_plan::DX_RUNTIME_PROOF_PLAN_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxRuntimeProofPlanTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "runtime_proof_plan": response,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "runs_just_run": false,
                "runs_cargo": false,
                "starts_local_servers": false,
                "dispatches_browser_input": false,
                "runs_external_processes": false,
                "deploys": false,
                "restores_to_target": false,
            },
            "next_action": "Run the governed manual validation window, then import evidence with import_dx_runtime_proof."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("Failed to serialize runtime proof plan receipt: {error}"))?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX runtime proof plan directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX runtime proof latest plan receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX runtime proof plan receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(dx_runtime_proof_plan::DxRuntimeProofPlanReceipt {
            schema: dx_runtime_proof_plan::DX_RUNTIME_PROOF_PLAN_RECEIPT_SCHEMA,
            status: "written",
            root_mode: self.root_mode_label().to_string(),
            receipt_dir: path_string(&self.receipt_dir),
            latest_path: path_string(&self.latest_path),
            archive_path: path_string(&self.archive_path),
            written_bytes: receipt_json.len(),
            plan_schema: response.schema,
            checklist_step_count: response.checklist.len(),
            next_action: response.next_action.clone(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX runtime proof plan receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxRuntimeProofPlanReceiptRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxRuntimeProofPlanReceiptRootMode::Workspace => "zed_data_fallback",
            DxRuntimeProofPlanReceiptRootMode::ZedData => "zed_data",
        }
    }
}

impl DxRuntimeProofPlanReceiptRootMode {
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
