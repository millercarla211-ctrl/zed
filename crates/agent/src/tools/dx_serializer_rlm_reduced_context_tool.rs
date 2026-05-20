use crate::{AgentTool, ToolCallEventStream, ToolInput, dx_serializer_rlm_reduced_context};
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
use util::markdown::MarkdownInlineCode;

const DX_SERIALIZER_RLM_REDUCED_CONTEXT_LATEST_FILE_NAME: &str =
    "latest-dx-serializer-rlm-reduced-context-receipt.json";

/// Write a deterministic reduced-context receipt from an approved serializer/RLM runner gate.
///
/// This tool consumes a `zed.dx.serializer_rlm.runner_gate.v1` object or receipt plus a
/// `zed.dx.serializer_rlm.context_bundle.v1` object or receipt. It writes managed receipts only
/// after authorization and never runs external serializer/RLM crates, model calls, cargo, network,
/// or browser input.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxSerializerRlmReducedContextToolInput {
    /// `zed.dx.serializer_rlm.runner_gate.v1` object, runner-gate receipt, or stringified JSON.
    pub runner_gate: Value,
    /// `zed.dx.serializer_rlm.context_bundle.v1` object, context receipt, or stringified JSON.
    pub context_bundle: Value,
    /// Approximate maximum token budget for the reduced context. Defaults to 900 and caps at 4000.
    pub max_output_tokens: Option<usize>,
    /// Require a runner-ready gate before emitting reduced context text.
    pub require_runner_ready: bool,
    /// Persist the reduced-context contract to a managed receipt file after authorization.
    pub write_reduced_context_receipt: bool,
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data.
    pub receipt_root_mode: DxSerializerRlmReducedContextReceiptRootMode,
}

impl Default for DxSerializerRlmReducedContextToolInput {
    fn default() -> Self {
        Self {
            runner_gate: Value::Null,
            context_bundle: Value::Null,
            max_output_tokens: Some(900),
            require_runner_ready: true,
            write_reduced_context_receipt: false,
            receipt_root_mode: DxSerializerRlmReducedContextReceiptRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxSerializerRlmReducedContextReceiptRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxSerializerRlmReducedContextTool {
    project: Entity<Project>,
}

impl DxSerializerRlmReducedContextTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxSerializerRlmReducedContextTool {
    type Input = DxSerializerRlmReducedContextToolInput;
    type Output = String;

    const NAME: &'static str = "write_dx_serializer_rlm_reduced_context";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Edit
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            let tokens = input.max_output_tokens.unwrap_or(900);
            format!(
                "Write DX serializer/RLM reduced context {}",
                MarkdownInlineCode(&format!("{tokens} tokens"))
            )
            .into()
        } else {
            "Write DX serializer/RLM reduced context".into()
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
                    "DX serializer/RLM reduced context needs a runner gate or receipt.".to_string(),
                );
            }
            if input.context_bundle.is_null() {
                return Err(
                    "DX serializer/RLM reduced context needs a context bundle or receipt."
                        .to_string(),
                );
            }

            let receipt_target = input.write_reduced_context_receipt.then(|| {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxSerializerRlmReducedContextReceiptTarget::new(
                    project_root,
                    input.receipt_root_mode,
                )
            });

            let authorize = cx.update(|cx| {
                let mut permission_values = vec![
                    format!(
                        "max_output_tokens={}",
                        input.max_output_tokens.unwrap_or(900)
                    ),
                    format!("require_runner_ready={}", input.require_runner_ready),
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
                .map(DxSerializerRlmReducedContextReceiptTarget::root_mode_label)
                .unwrap_or_else(|| input.receipt_root_mode.label())
                .to_string();
            let mut response =
                dx_serializer_rlm_reduced_context::build_serializer_rlm_reduced_context(
                    dx_serializer_rlm_reduced_context::DxSerializerRlmReducedContextRequest {
                        runner_gate: input.runner_gate,
                        context_bundle: input.context_bundle,
                        max_output_tokens: input.max_output_tokens,
                        require_runner_ready: input.require_runner_ready,
                        root_mode,
                    },
                )?;

            if let Some(receipt_target) = receipt_target {
                response.reduced_context_receipt =
                    Some(receipt_target.write_receipt(&response).map_err(|error| {
                        format!(
                            "Failed to write DX serializer/RLM reduced context receipt: {error}"
                        )
                    })?);
            }

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Wrote DX reduced context: {}",
                response.reduction.status
            )));

            serde_json::to_string_pretty(&response).map_err(|error| {
                format!("Failed to serialize DX serializer/RLM reduced context: {error}")
            })
        })
    }
}

struct DxSerializerRlmReducedContextReceiptTarget {
    root_mode: DxSerializerRlmReducedContextReceiptRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    receipt_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl DxSerializerRlmReducedContextReceiptTarget {
    fn new(
        project_root: Option<PathBuf>,
        root_mode: DxSerializerRlmReducedContextReceiptRootMode,
    ) -> Self {
        let use_workspace = matches!(
            root_mode,
            DxSerializerRlmReducedContextReceiptRootMode::Workspace
        ) && project_root.is_some();
        let allowed_root = match (root_mode, project_root.as_ref()) {
            (DxSerializerRlmReducedContextReceiptRootMode::Workspace, Some(root)) => {
                root.join("tools")
            }
            _ => data_dir().join("dx-serializer-rlm"),
        };
        let receipt_dir = if use_workspace {
            allowed_root
                .join("dx-serializer-rlm")
                .join("reduced-context")
        } else {
            allowed_root.join("reduced-context")
        };
        let latest_path = receipt_dir.join(DX_SERIALIZER_RLM_REDUCED_CONTEXT_LATEST_FILE_NAME);
        let archive_path = receipt_dir.join(format!(
            "dx-serializer-rlm-reduced-context-{}.json",
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
        response: &dx_serializer_rlm_reduced_context::DxSerializerRlmReducedContext,
    ) -> Result<dx_serializer_rlm_reduced_context::DxSerializerRlmReducedContextReceipt, String>
    {
        self.validate()?;
        let receipt = serde_json::json!({
            "schema": dx_serializer_rlm_reduced_context::DX_SERIALIZER_RLM_REDUCED_CONTEXT_RECEIPT_SCHEMA,
            "written_at_ms": current_epoch_millis(),
            "source_tool": DxSerializerRlmReducedContextTool::NAME,
            "root_mode": self.root_mode_label(),
            "project_root": self.project_root.as_ref().map(path_string),
            "receipt_dir": path_string(&self.receipt_dir),
            "latest_path": path_string(&self.latest_path),
            "archive_path": path_string(&self.archive_path),
            "reduced_context": response,
            "safety": {
                "written_after_authorization": true,
                "writes_under_managed_root": true,
                "runs_external_serializer": false,
                "runs_external_rlm": false,
                "runs_model_calls": false,
                "runs_cargo": false,
                "fetches_network": false,
                "dispatches_browser_input": false,
            },
            "next_action": "Use this deterministic reduced-context receipt as the future serializer/RLM runner input contract."
        });
        let receipt_json = serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("Failed to serialize reduced context receipt: {error}"))?;

        fs::create_dir_all(&self.receipt_dir).map_err(|error| {
            format!(
                "Failed to prepare DX serializer/RLM reduced context receipt directory {}: {error}",
                self.receipt_dir.display()
            )
        })?;
        fs::write(&self.latest_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to write DX serializer/RLM latest reduced context receipt {}: {error}",
                self.latest_path.display()
            )
        })?;
        fs::write(&self.archive_path, &receipt_json).map_err(|error| {
            format!(
                "Failed to archive DX serializer/RLM reduced context receipt {}: {error}",
                self.archive_path.display()
            )
        })?;

        Ok(
            dx_serializer_rlm_reduced_context::DxSerializerRlmReducedContextReceipt {
                schema: dx_serializer_rlm_reduced_context::DX_SERIALIZER_RLM_REDUCED_CONTEXT_RECEIPT_SCHEMA,
                status: "written",
                root_mode: self.root_mode_label().to_string(),
                receipt_dir: path_string(&self.receipt_dir),
                latest_path: path_string(&self.latest_path),
                archive_path: path_string(&self.archive_path),
                written_bytes: receipt_json.len(),
                reduced_context_schema: response.schema,
                runner_gate_status: response.gate.status.clone(),
                source_count: response.reduction.source_count,
                selected_estimated_tokens: response.reduction.selected_estimated_tokens,
                next_action: "Use the latest reduced context receipt as the deterministic input contract for future serializer/RLM execution.".to_string(),
            },
        )
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.receipt_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to write DX serializer/RLM reduced context receipt at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxSerializerRlmReducedContextReceiptRootMode::Workspace
                if self.project_root.is_some() =>
            {
                "workspace"
            }
            DxSerializerRlmReducedContextReceiptRootMode::Workspace => "zed_data_fallback",
            DxSerializerRlmReducedContextReceiptRootMode::ZedData => "zed_data",
        }
    }
}

impl DxSerializerRlmReducedContextReceiptRootMode {
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
