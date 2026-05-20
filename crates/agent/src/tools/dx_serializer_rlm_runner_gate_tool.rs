use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_serializer_rlm_runner_gate};
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

const DX_SERIALIZER_RLM_RUNNER_GATE_LATEST_FILE_NAME: &str =
    "latest-dx-serializer-rlm-runner-gate-receipt.json";

/// Validate an approved DX serializer/RLM execution plan before reducer runner wiring.
///
/// This gate accepts `zed.dx.serializer_rlm.execution_plan.v1` plans or execution receipts and
/// can write a managed runner-gate receipt. It does not run external processes, cargo, serializer,
/// RLM, model calls, network requests, or writes reduced context.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxSerializerRlmRunnerGateToolInput {
    /// `zed.dx.serializer_rlm.execution_plan.v1` object, execution receipt, or stringified JSON.
    pub execution_plan: Value,
    /// Explicit approval flag for the future serializer/RLM reducer runner.
    pub approve_runner: bool,
    /// Explicitly allow RLM steps that would perform model calls in a future runner.
    pub allow_model_calls: bool,
    /// Require the plan to be provided through a managed execution receipt before runner readiness.
    pub require_execution_receipt: bool,
    /// Persist the runner gate to a managed receipt file after authorization.
    pub write_runner_gate_receipt: bool,
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data.
    pub receipt_root_mode: DxSerializerRlmRunnerGateReceiptRootMode,
}

impl Default for DxSerializerRlmRunnerGateToolInput {
    fn default() -> Self {
        Self {
            execution_plan: Value::Null,
            approve_runner: false,
            allow_model_calls: false,
            require_execution_receipt: true,
            write_runner_gate_receipt: false,
            receipt_root_mode: DxSerializerRlmRunnerGateReceiptRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxSerializerRlmRunnerGateReceiptRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxSerializerRlmRunnerGateTool {
    project: Entity<Project>,
}

impl DxSerializerRlmRunnerGateTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxSerializerRlmRunnerGateTool {
    type Input = DxSerializerRlmRunnerGateToolInput;
    type Output = String;

    const NAME: &'static str = "gate_dx_serializer_rlm_runner";

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
                "Gate approved DX serializer/RLM runner".into()
            } else {
                "Gate DX serializer/RLM runner".into()
            }
        } else {
            "Gate DX serializer/RLM runner".into()
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
            if input.execution_plan.is_null() {
                return Err(
                    "DX serializer/RLM runner gate needs an execution plan or receipt.".to_string(),
                );
            }

            let receipt_target = input.write_runner_gate_receipt.then(|| {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxSerializerRlmRunnerGateReceiptTarget::new(project_root, input.receipt_root_mode)
            });

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![
                    format!("approve_runner={}", input.approve_runner),
                    format!("allow_model_calls={}", input.allow_model_calls),
                    format!(
                        "require_execution_receipt={}",
                        input.require_execution_receipt
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
                .map(DxSerializerRlmRunnerGateReceiptTarget::root_mode_label)
                .unwrap_or_else(|| input.receipt_root_mode.label())
                .to_string();
            let mut response = dx_serializer_rlm_runner_gate::build_serializer_rlm_runner_gate(
                dx_serializer_rlm_runner_gate::DxSerializerRlmRunnerGateRequest {
                    execution_plan: input.execution_plan,
                    approve_runner: input.approve_runner,
                    allow_model_calls: input.allow_model_calls,
                    require_execution_receipt: input.require_execution_receipt,
                    root_mode,
                },
            )?;

            if let Some(receipt_target) = receipt_target {
                response.runner_receipt =
                    Some(receipt_target.write_receipt(&response).map_err(|error| {
                        format!("Failed to write DX serializer/RLM runner gate receipt: {error}")
                    })?);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Gated DX serializer/RLM runner: {}",
                response.validation.status
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX serializer/RLM runner gate: {error}")
            })
        })
    }
}

struct DxSerializerRlmRunnerGateReceiptTarget {
    root_mode: DxSerializerRlmRunnerGateReceiptRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxSerializerRlmRunnerGateReceiptTarget {
    fn new(
        project_root: Option<PathBuf>,
        root_mode: DxSerializerRlmRunnerGateReceiptRootMode,
    ) -> Self {
        let allowed_root = match (root_mode, project_root.as_ref()) {
            (DxSerializerRlmRunnerGateReceiptRootMode::Workspace, Some(root)) => {
                root.join("tools").join("dx-serializer-rlm")
            }
            _ => data_dir().join("dx-serializer-rlm"),
        };
        let receipt_dir = allowed_root.join("runner-gates");
        let latest_path = receipt_dir.join(DX_SERIALIZER_RLM_RUNNER_GATE_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-serializer-rlm-runner-gate-{}.json",
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
        response: &dx_serializer_rlm_runner_gate::DxSerializerRlmRunnerGate,
    ) -> Result<dx_serializer_rlm_runner_gate::DxSerializerRlmRunnerGateReceipt, String> {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_serializer_rlm_runner_gate::DX_SERIALIZER_RLM_RUNNER_GATE_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxSerializerRlmRunnerGateTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "runner_gate": response,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "runs_external_serializer": false,
                "runs_external_rlm": false,
                "runs_model_calls": false,
                "runs_cargo": false,
                "fetches_network": false,
                "writes_reduced_context": false,
                "dispatches_browser_input": false,
            },
            "next_action": "Use this runner gate receipt to wire the future serializer/RLM reducer executor under the same approval gates."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("Failed to serialize runner gate receipt: {error}"))?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX serializer/RLM runner gate receipt directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX serializer/RLM latest runner gate receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX serializer/RLM runner gate receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(
            dx_serializer_rlm_runner_gate::DxSerializerRlmRunnerGateReceipt {
                schema: dx_serializer_rlm_runner_gate::DX_SERIALIZER_RLM_RUNNER_GATE_RECEIPT_SCHEMA,
                status: "written",
                root_mode: self.root_mode_label().to_string(),
                receipt_dir: path_string(&self.receipt_dir),
                latest_path: path_string(&self.latest_path),
                archive_path: path_string(&self.archive_path),
                written_bytes: receipt_json.len(),
                runner_gate_schema: response.schema,
                reducer: response.plan.reducer.clone(),
                runner_ready: response.validation.runner_ready,
                step_count: response.plan.step_count,
                next_action: "Use the latest runner gate receipt to add the future serializer/RLM reducer executor after review.".to_string(),
            },
        )
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX serializer/RLM runner gate receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxSerializerRlmRunnerGateReceiptRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            DxSerializerRlmRunnerGateReceiptRootMode::Workspace => "zed_data_fallback",
            DxSerializerRlmRunnerGateReceiptRootMode::ZedData => "zed_data",
        }
    }
}

impl DxSerializerRlmRunnerGateReceiptRootMode {
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
