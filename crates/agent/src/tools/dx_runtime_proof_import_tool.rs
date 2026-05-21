use crate::{
    AgentTool, ToolCallEventStream, ToolInput, dx_runtime_proof_import,
    dx_runtime_proof_import::DxRuntimeProofOperatorStatus,
};
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

const DX_RUNTIME_PROOF_IMPORT_LATEST_FILE_NAME: &str =
    "latest-dx-runtime-proof-import-receipt.json";
const DX_RUNTIME_PROOF_STATUS_LATEST_FILE_NAME: &str = "latest-dx-runtime-proof-status-copy.json";

/// Import operator-supplied runtime proof evidence into managed DX receipts.
///
/// This tool records proof produced outside the Agent tool runtime, such as a manually governed
/// final validation window. It writes managed receipts only after authorization and never runs
/// `just run`, Cargo, local servers, browser automation, deploys, external reducer code, model
/// calls, or restore-to-target actions.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxRuntimeProofImportToolInput {
    /// Operator-reported runtime proof status from the governed validation window.
    pub operator_status: DxRuntimeProofOperatorStatus,
    /// Short human-readable summary of the proof outcome.
    pub proof_summary: String,
    /// Evidence lines, such as command output summary, receipt path, window title, or artifact path.
    pub evidence: Vec<String>,
    /// Known blockers that should prevent runtime-green claims.
    pub blockers: Vec<String>,
    /// The manual command or validation action that produced the proof.
    pub final_command: Option<String>,
    /// Optional source label for the imported proof.
    pub source: Option<String>,
    /// Persist import and operator-status receipts to a managed receipt root after authorization.
    pub write_runtime_proof_receipt: bool,
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data.
    pub receipt_root_mode: DxRuntimeProofImportReceiptRootMode,
}

impl Default for DxRuntimeProofImportToolInput {
    fn default() -> Self {
        Self {
            operator_status: DxRuntimeProofOperatorStatus::Unknown,
            proof_summary: String::new(),
            evidence: Vec::new(),
            blockers: Vec::new(),
            final_command: None,
            source: None,
            write_runtime_proof_receipt: true,
            receipt_root_mode: DxRuntimeProofImportReceiptRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxRuntimeProofImportReceiptRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxRuntimeProofImportTool {
    project: Entity<Project>,
}

impl DxRuntimeProofImportTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxRuntimeProofImportTool {
    type Input = DxRuntimeProofImportToolInput;
    type Output = String;

    const NAME: &'static str = "import_dx_runtime_proof";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Edit
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            return format!(
                "Import DX runtime proof {}",
                MarkdownInlineCode(input.operator_status.as_str())
            )
            .into();
        }

        "Import DX runtime proof".into()
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
            let receipt_target = input.write_runtime_proof_receipt.then(|| {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxRuntimeProofImportReceiptTarget::new(project_root, input.receipt_root_mode)
            });

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![
                    format!("operator_status={}", input.operator_status.as_str()),
                    format!("evidence_count={}", input.evidence.len()),
                    format!("blocker_count={}", input.blockers.len()),
                ];
                if let Some(command) = input.final_command.as_deref() {
                    permission_values.push(format!("final_command={}", command.trim()));
                }
                if let Some(receipt_target) = &receipt_target {
                    permission_values.push(path_string(&receipt_target.import_latest_path));
                    permission_values.push(path_string(&receipt_target.status_latest_path));
                }
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;
            let root_mode = receipt_target
                .as_ref()
                .map(DxRuntimeProofImportReceiptTarget::root_mode_label)
                .unwrap_or_else(|| input.receipt_root_mode.label())
                .to_string();
            let mut response = dx_runtime_proof_import::build_runtime_proof_import(
                dx_runtime_proof_import::DxRuntimeProofImportRequest {
                    operator_status: input.operator_status,
                    proof_summary: input.proof_summary,
                    evidence: input.evidence,
                    blockers: input.blockers,
                    final_command: input.final_command,
                    source: input.source,
                    root_mode,
                    generated_at_ms: current_epoch_millis(),
                },
            );

            if let Some(receipt_target) = receipt_target {
                let (import_receipt, status_receipt) =
                    receipt_target.write_receipts(&response).map_err(|error| {
                        format!("Failed to write DX runtime proof receipts: {error}")
                    })?;
                response.import_receipt = Some(import_receipt);
                response.status_receipt = Some(status_receipt);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Imported DX runtime proof: {}",
                response.validation.status
            )));

            serde_json::to_string_pretty(&response)
                .map_err(|error| format!("Failed to serialize DX runtime proof import: {error}"))
        })
    }
}

struct DxRuntimeProofImportReceiptTarget {
    root_mode: DxRuntimeProofImportReceiptRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    import_dir: PathBuf,
    status_dir: PathBuf,
    import_latest_path: PathBuf,
    import_archive_path: PathBuf,
    status_latest_path: PathBuf,
    status_archive_path: PathBuf,
}

impl DxRuntimeProofImportReceiptTarget {
    fn new(project_root: Option<PathBuf>, root_mode: DxRuntimeProofImportReceiptRootMode) -> Self {
        let allowed_root = match (root_mode, project_root.as_ref()) {
            (DxRuntimeProofImportReceiptRootMode::Workspace, Some(root)) => {
                root.join("tools").join("dx-runtime-proof")
            }
            _ => data_dir().join("dx-runtime-proof"),
        };
        let import_dir = allowed_root.join("imports");
        let status_dir = allowed_root.join("status");
        let import_latest_path = import_dir.join(DX_RUNTIME_PROOF_IMPORT_LATEST_FILE_NAME);
        let import_archive_path = import_dir.join(format!(
            "dx-runtime-proof-import-{}.json",
            current_epoch_millis()
        ));
        let status_latest_path = status_dir.join(DX_RUNTIME_PROOF_STATUS_LATEST_FILE_NAME);
        let status_archive_path = status_dir.join(format!(
            "dx-runtime-proof-status-{}.json",
            current_epoch_millis()
        ));

        Self {
            root_mode,
            project_root,
            allowed_root,
            import_dir,
            status_dir,
            import_latest_path,
            import_archive_path,
            status_latest_path,
            status_archive_path,
        }
    }

    fn write_receipts(
        &self,
        response: &dx_runtime_proof_import::DxRuntimeProofImport,
    ) -> Result<
        (
            dx_runtime_proof_import::DxRuntimeProofImportReceipt,
            dx_runtime_proof_import::DxRuntimeProofStatusReceipt,
        ),
        String,
    > {
        self.validate()?;

        let import_receipt = serde_json::json!({
            "schema": dx_runtime_proof_import::DX_RUNTIME_PROOF_IMPORT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxRuntimeProofImportTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.import_dir),
            "latest_path": path_string(&self.import_latest_path),
            "archive_path": path_string(&self.import_archive_path),
            "runtime_proof": response,
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
        });
        let status_copy = serde_json::json!({
            "schema": dx_runtime_proof_import::DX_RUNTIME_PROOF_STATUS_COPY_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxRuntimeProofImportTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "status_dir": path_string(&self.status_dir),
            "latest_path": path_string(&self.status_latest_path),
            "archive_path": path_string(&self.status_archive_path),
            "operator_status_copy": &response.operator_status_copy,
            "validation": &response.validation,
        });
        let import_json = serde_json::to_vec_pretty(&import_receipt).map_err(|error| {
            format!("Failed to serialize runtime proof import receipt: {error}")
        })?;
        let status_json = serde_json::to_vec_pretty(&status_copy).map_err(|error| {
            format!("Failed to serialize runtime proof status receipt: {error}")
        })?;

        fs::create_dir_all(&self.import_dir).map_err(|error| {
            format!(
                "Failed to prepare DX runtime proof import directory {}: {error}",
                self.import_dir.display()
            )
        })?;
        fs::create_dir_all(&self.status_dir).map_err(|error| {
            format!(
                "Failed to prepare DX runtime proof status directory {}: {error}",
                self.status_dir.display()
            )
        })?;
        fs::write(&self.import_latest_path, &import_json).map_err(|error| {
            format!(
                "Failed to write DX runtime proof latest import receipt {}: {error}",
                self.import_latest_path.display()
            )
        })?;
        fs::write(&self.import_archive_path, &import_json).map_err(|error| {
            format!(
                "Failed to archive DX runtime proof import receipt {}: {error}",
                self.import_archive_path.display()
            )
        })?;
        fs::write(&self.status_latest_path, &status_json).map_err(|error| {
            format!(
                "Failed to write DX runtime proof latest status copy {}: {error}",
                self.status_latest_path.display()
            )
        })?;
        fs::write(&self.status_archive_path, &status_json).map_err(|error| {
            format!(
                "Failed to archive DX runtime proof status copy {}: {error}",
                self.status_archive_path.display()
            )
        })?;

        Ok((
            dx_runtime_proof_import::DxRuntimeProofImportReceipt {
                schema: dx_runtime_proof_import::DX_RUNTIME_PROOF_IMPORT_SCHEMA,
                status: "written",
                root_mode: self.root_mode_label().to_string(),
                receipt_dir: path_string(&self.import_dir),
                latest_path: path_string(&self.import_latest_path),
                archive_path: path_string(&self.import_archive_path),
                written_bytes: import_json.len(),
                operator_status: response.request.operator_status,
                runtime_green_candidate: response.validation.runtime_green_candidate,
                next_action: response.next_action.clone(),
            },
            dx_runtime_proof_import::DxRuntimeProofStatusReceipt {
                schema: dx_runtime_proof_import::DX_RUNTIME_PROOF_STATUS_COPY_SCHEMA,
                status: "written",
                root_mode: self.root_mode_label().to_string(),
                status_dir: path_string(&self.status_dir),
                latest_path: path_string(&self.status_latest_path),
                archive_path: path_string(&self.status_archive_path),
                written_bytes: status_json.len(),
                headline: response.operator_status_copy.headline.clone(),
                next_action: response.operator_status_copy.next_action.clone(),
            },
        ))
    }

    fn validate(&self) -> Result<(), String> {
        for path in [
            &self.import_dir,
            &self.status_dir,
            &self.import_latest_path,
            &self.import_archive_path,
            &self.status_latest_path,
            &self.status_archive_path,
        ] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX runtime proof receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxRuntimeProofImportReceiptRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxRuntimeProofImportReceiptRootMode::Workspace => "zed_data_fallback",
            DxRuntimeProofImportReceiptRootMode::ZedData => "zed_data",
        }
    }
}

impl DxRuntimeProofImportReceiptRootMode {
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
