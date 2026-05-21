use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_serializer_rlm_external_execution};
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

const DX_SERIALIZER_RLM_EXTERNAL_EXECUTION_LATEST_FILE_NAME: &str =
    "latest-dx-serializer-rlm-external-execution-receipt.json";

/// Execute an approved DX serializer/RLM reducer command and write a managed receipt.
///
/// The executor accepts only an explicit absolute no-shell command vector under approved DX
/// serializer/RLM roots. It consumes a ready execution-preview receipt plus a deterministic
/// reduced-context receipt, writes stdout/stderr previews and hashes to a managed receipt, and
/// never starts Cargo, package managers, local servers, or shell interpreters.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxSerializerRlmExternalExecutionToolInput {
    /// `zed.dx.serializer_rlm.execution_preview.v1` object, execution-preview receipt, or stringified JSON.
    pub execution_preview: Value,
    /// `zed.dx.serializer_rlm.reduced_context.v1` object, reduced-context receipt, or stringified JSON.
    pub reduced_context: Value,
    /// Explicit absolute executable path plus arguments. Do not include shell syntax or secrets.
    pub command_vector: Vec<String>,
    /// Explicit approval flag for running the external serializer/RLM reducer.
    pub approve_external_execution: bool,
    /// Explicitly allow reducer steps that may make model calls outside Zed.
    pub allow_model_calls: bool,
    /// Require a managed execution receipt. Must stay true for approved execution.
    pub require_execution_receipt: bool,
    /// What the executor should send to stdin.
    pub stdin_mode: DxSerializerRlmExternalExecutionStdinModeInput,
    /// Maximum reduced-context text characters to send to stdin.
    pub max_stdin_chars: Option<usize>,
    /// Maximum stdout/stderr characters to include in the response and receipt previews.
    pub max_output_preview_chars: Option<usize>,
    /// Prefer workspace-local execution receipts under `<workspace>/tools`; falls back to Zed data.
    pub receipt_root_mode: DxSerializerRlmExternalExecutionReceiptRootMode,
}

impl Default for DxSerializerRlmExternalExecutionToolInput {
    fn default() -> Self {
        Self {
            execution_preview: Value::Null,
            reduced_context: Value::Null,
            command_vector: Vec::new(),
            approve_external_execution: false,
            allow_model_calls: false,
            require_execution_receipt: true,
            stdin_mode: DxSerializerRlmExternalExecutionStdinModeInput::ReducedContextText,
            max_stdin_chars: None,
            max_output_preview_chars: None,
            receipt_root_mode: DxSerializerRlmExternalExecutionReceiptRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxSerializerRlmExternalExecutionStdinModeInput {
    None,
    #[default]
    ReducedContextText,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxSerializerRlmExternalExecutionReceiptRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxSerializerRlmExternalExecutionTool {
    project: Entity<Project>,
}

impl DxSerializerRlmExternalExecutionTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxSerializerRlmExternalExecutionTool {
    type Input = DxSerializerRlmExternalExecutionToolInput;
    type Output = String;

    const NAME: &'static str = "execute_dx_serializer_rlm_reducer";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            if input.approve_external_execution {
                return "Execute approved DX serializer/RLM reducer".into();
            }
        }

        "Prepare DX serializer/RLM reducer execution".into()
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
            if input.execution_preview.is_null() {
                return Err(
                    "DX serializer/RLM external execution needs an execution-preview receipt."
                        .to_string(),
                );
            }
            if input.reduced_context.is_null() {
                return Err(
                    "DX serializer/RLM external execution needs a reduced-context receipt."
                        .to_string(),
                );
            }
            if input.approve_external_execution && !input.require_execution_receipt {
                return Err(
                    "Approved DX serializer/RLM external execution requires an execution receipt."
                        .to_string(),
                );
            }

            let receipt_target = {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxSerializerRlmExternalExecutionReceiptTarget::new(
                    project_root,
                    input.receipt_root_mode,
                )
            };

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![
                    format!(
                        "approve_external_execution={}",
                        input.approve_external_execution
                    ),
                    format!("allow_model_calls={}", input.allow_model_calls),
                    format!(
                        "require_execution_receipt={}",
                        input.require_execution_receipt
                    ),
                    format!("stdin_mode={}", input.stdin_mode.label()),
                    path_string(&receipt_target.receipt_dir),
                    path_string(&receipt_target.latest_path),
                    path_string(&receipt_target.archive_path),
                ];
                if let Some(executable) = input.command_vector.first() {
                    permission_values.push(format!("executable={executable}"));
                }
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;
            let mut response =
                dx_serializer_rlm_external_execution::execute_serializer_rlm_external_reducer(
                    receipt_target.request(input),
                )?;

            response.execution.wrote_managed_receipt = true;
            response.external_execution_receipt =
                Some(receipt_target.write_receipt(&response).map_err(|error| {
                    format!("Failed to write DX serializer/RLM external execution receipt: {error}")
                })?);

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "DX serializer/RLM reducer: {}",
                response.execution.status
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX serializer/RLM external execution: {error}")
            })
        })
    }
}

struct DxSerializerRlmExternalExecutionReceiptTarget {
    root_mode: DxSerializerRlmExternalExecutionReceiptRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxSerializerRlmExternalExecutionReceiptTarget {
    fn new(
        project_root: Option<PathBuf>,
        root_mode: DxSerializerRlmExternalExecutionReceiptRootMode,
    ) -> Self {
        let allowed_root = match (root_mode, project_root.as_ref()) {
            (DxSerializerRlmExternalExecutionReceiptRootMode::Workspace, Some(root)) => {
                root.join("tools").join("dx-serializer-rlm")
            }
            _ => data_dir().join("dx-serializer-rlm"),
        };
        let receipt_dir = allowed_root.join("external-executions");
        let latest_path = receipt_dir.join(DX_SERIALIZER_RLM_EXTERNAL_EXECUTION_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-serializer-rlm-external-execution-{}.json",
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
        input: DxSerializerRlmExternalExecutionToolInput,
    ) -> dx_serializer_rlm_external_execution::DxSerializerRlmExternalExecutionRequest {
        dx_serializer_rlm_external_execution::DxSerializerRlmExternalExecutionRequest {
            execution_preview: input.execution_preview,
            reduced_context: input.reduced_context,
            command_vector: input.command_vector,
            approve_external_execution: input.approve_external_execution,
            allow_model_calls: input.allow_model_calls,
            require_execution_receipt: input.require_execution_receipt,
            stdin_mode: input.stdin_mode.into(),
            max_stdin_chars: input.max_stdin_chars,
            max_output_preview_chars: input.max_output_preview_chars,
            root_mode: self.root_mode_label().to_string(),
        }
    }

    fn write_receipt(
        &self,
        response: &dx_serializer_rlm_external_execution::DxSerializerRlmExternalExecution,
    ) -> Result<dx_serializer_rlm_external_execution::DxSerializerRlmExternalExecutionReceipt, String>
    {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_serializer_rlm_external_execution::DX_SERIALIZER_RLM_EXTERNAL_EXECUTION_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxSerializerRlmExternalExecutionTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "external_execution": response,
            "stdout_sha256": &response.execution.stdout_sha256,
            "stderr_sha256": &response.execution.stderr_sha256,
            "stdout_preview": &response.execution.stdout_preview,
            "stderr_preview": &response.execution.stderr_preview,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "runs_shell": false,
                "runs_cargo": false,
                "runs_build_or_package_tools": false,
                "runs_external_process": response.execution.ran_external_process,
                "runs_external_serializer_or_rlm": response.execution.ran_external_process,
                "runs_model_calls_in_zed_process": false,
                "external_reducer_model_calls_approved": response.execution.model_calls_approved,
                "zed_writes_unmanaged_files": false,
                "external_process_filesystem_writes_unverified": response.execution.ran_external_process,
                "dispatches_browser_input": false,
            },
            "next_action": "Review this managed reducer execution receipt before copying stdout into Agent context or pinning it in the Sources rail."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt).map_err(|error| {
            format!("Failed to serialize serializer/RLM external execution receipt: {error}")
        })?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX serializer/RLM external execution directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX serializer/RLM latest external execution receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX serializer/RLM external execution receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(
            dx_serializer_rlm_external_execution::DxSerializerRlmExternalExecutionReceipt {
                schema: dx_serializer_rlm_external_execution::DX_SERIALIZER_RLM_EXTERNAL_EXECUTION_RECEIPT_SCHEMA,
                status: "written",
                root_mode: self.root_mode_label().to_string(),
                receipt_dir: path_string(&self.receipt_dir),
                latest_path: path_string(&self.latest_path),
                archive_path: path_string(&self.archive_path),
                written_bytes: receipt_json.len(),
                execution_schema: response.schema,
                reducer: response.preview.reducer.clone(),
                exit_code: response.execution.exit_code,
                ran_external_process: response.execution.ran_external_process,
                next_action: response.next_action.clone(),
            },
        )
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX serializer/RLM external execution receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxSerializerRlmExternalExecutionReceiptRootMode::Workspace
                if self.project_root.is_some() =>
            {
                "workspace"
            }
            DxSerializerRlmExternalExecutionReceiptRootMode::Workspace => "zed_data_fallback",
            DxSerializerRlmExternalExecutionReceiptRootMode::ZedData => "zed_data",
        }
    }
}

impl DxSerializerRlmExternalExecutionStdinModeInput {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ReducedContextText => "reduced_context_text",
        }
    }
}

impl From<DxSerializerRlmExternalExecutionStdinModeInput>
    for dx_serializer_rlm_external_execution::DxSerializerRlmExternalExecutionStdinMode
{
    fn from(value: DxSerializerRlmExternalExecutionStdinModeInput) -> Self {
        match value {
            DxSerializerRlmExternalExecutionStdinModeInput::None => Self::None,
            DxSerializerRlmExternalExecutionStdinModeInput::ReducedContextText => {
                Self::ReducedContextText
            }
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
