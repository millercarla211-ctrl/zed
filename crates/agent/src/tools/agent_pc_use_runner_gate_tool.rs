use super::agent_pc_use_payload_tool::{
    AGENT_PC_USE_PAYLOAD_QUEUE_FILE_NAME, AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA,
    AGENT_PC_USE_PAYLOAD_SCHEMA, AgentPcUsePayloadQueueRootMode,
};
use crate::{AgentTool, ToolCallEventStream, ToolInput, ToolPermissionContext};
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

pub const AGENT_PC_USE_RUNNER_GATE_TOOL_NAME: &str = "request_zed_pc_use_payload_run";
pub const AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA: &str = "zed.agent_plugins.pc_use.runner_receipt.v1";
pub const AGENT_PC_USE_RUNNER_RECEIPT_FILE_NAME: &str = "latest-zed-pc-use-runner-receipt.json";

/// Opens the Zed-window PC-use runner gate and writes an auditable receipt.
///
/// This is intentionally not a desktop automation dispatcher. It validates the managed queue item,
/// asks for permission, writes a blocked or ready receipt, and preserves the invariant that no
/// screenshot, focus, click, type, process launch, or OS-wide desktop control happens here.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPcUseRunnerGateToolInput {
    /// Prefer workspace-local queue under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentPcUsePayloadQueueRootMode,
    /// Include the full queued payload packet in the receipt.
    pub include_payload_packet: bool,
}

impl Default for AgentPcUseRunnerGateToolInput {
    fn default() -> Self {
        Self {
            root_mode: AgentPcUsePayloadQueueRootMode::Workspace,
            include_payload_packet: false,
        }
    }
}

pub struct AgentPcUseRunnerGateTool {
    project: Entity<Project>,
}

impl AgentPcUseRunnerGateTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentPcUseRunnerGateTool {
    type Input = AgentPcUseRunnerGateToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_PC_USE_RUNNER_GATE_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Request Zed PC-use run".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let project_root = cx.update(|cx| workspace_root_for_project(&self.project, cx));
            let gate = AgentPcUseRunnerGate::new(project_root, input.root_mode);
            gate.validate_managed_paths()?;
            let receipt = gate.receipt(input.include_payload_packet);

            let outcome = receipt
                .pointer("/result/outcome")
                .and_then(Value::as_str)
                .unwrap_or("blocked")
                .to_string();
            let action = receipt
                .pointer("/queue/action")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let context = ToolPermissionContext::new(
                Self::NAME,
                vec![
                    action.clone(),
                    outcome.clone(),
                    path_string(&gate.latest_queue_path),
                    path_string(&gate.latest_receipt_path),
                ],
            );
            let authorize = cx
                .update(|cx| {
                    event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
                })
                .map_err(|error| error.to_string())?;
            authorize.await.map_err(|error| error.to_string())?;

            let receipt_json = serde_json::to_vec_pretty(&receipt)
                .map_err(|error| format!("Failed to serialize PC-use runner receipt: {error}"))?;
            fs::create_dir_all(&gate.receipt_dir).map_err(|error| {
                format!(
                    "Failed to prepare PC-use runner receipt directory {}: {error}",
                    gate.receipt_dir.display()
                )
            })?;
            fs::write(&gate.latest_receipt_path, &receipt_json).map_err(|error| {
                format!(
                    "Failed to write PC-use runner receipt {}: {error}",
                    gate.latest_receipt_path.display()
                )
            })?;
            fs::write(&gate.archive_receipt_path, &receipt_json).map_err(|error| {
                format!(
                    "Failed to archive PC-use runner receipt {}: {error}",
                    gate.archive_receipt_path.display()
                )
            })?;

            let output = serde_json::json!({
                "schema": "zed.agent_plugins.pc_use.runner_gate_result.v1",
                "result": {
                    "generated_at_ms": current_epoch_millis(),
                    "status": "receipt_written",
                    "outcome": outcome.clone(),
                    "root_mode": gate.root_mode_label(),
                    "latest_receipt_path": path_string(&gate.latest_receipt_path),
                    "archive_receipt_path": path_string(&gate.archive_receipt_path),
                    "next_step": if outcome == "ready_future_executor_pending" {
                        "The managed PC-use queue is valid and permission was granted for this receipt; future work can add a Zed-window importer/executor that consumes this receipt without changing its safety contract."
                    } else {
                        "Resolve the receipt blockers, queue a valid PC-use payload, and request the runner gate again before any future executor can consume it."
                    }
                },
                "receipt": receipt,
                "safety": {
                    "permission_prompted_before_receipt_write": true,
                    "receipt_only": true,
                    "takes_screenshot": false,
                    "focuses_zed": false,
                    "dispatches_mouse": false,
                    "dispatches_keyboard": false,
                    "launches_process": false,
                    "os_wide_desktop_control": false
                }
            });
            let output = serde_json::to_string_pretty(&output)
                .map_err(|error| format!("Failed to serialize PC-use run request: {error}"))?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(match outcome.as_str() {
                "ready_future_executor_pending" => "Zed PC-use run gate ready",
                "blocked_missing_queue" => "Zed PC-use run blocked: missing queue",
                "blocked_invalid_queue" => "Zed PC-use run blocked: invalid queue",
                _ => "Zed PC-use run gate receipt written",
            }));

            Ok(output)
        })
    }
}

struct AgentPcUseRunnerGate {
    root_mode: AgentPcUsePayloadQueueRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    pc_use_root: PathBuf,
    latest_queue_path: PathBuf,
    receipt_dir: PathBuf,
    latest_receipt_path: PathBuf,
    archive_receipt_path: PathBuf,
}

impl AgentPcUseRunnerGate {
    fn new(project_root: Option<PathBuf>, root_mode: AgentPcUsePayloadQueueRootMode) -> Self {
        let use_workspace = matches!(root_mode, AgentPcUsePayloadQueueRootMode::Workspace)
            && project_root.is_some();
        let zed_plugin_root = data_dir().join("agent-plugins");
        let (allowed_root, pc_use_root) = if use_workspace {
            let workspace_root = project_root.as_ref().expect("workspace root checked above");
            let tools_root = workspace_root.join("tools");
            (
                tools_root.clone(),
                tools_root.join("agent-plugins").join("pc-use"),
            )
        } else {
            (zed_plugin_root.clone(), zed_plugin_root.join("pc-use"))
        };
        let latest_queue_path = pc_use_root
            .join("payloads")
            .join(AGENT_PC_USE_PAYLOAD_QUEUE_FILE_NAME);
        let receipt_dir = pc_use_root.join("receipts");
        let latest_receipt_path = receipt_dir.join(AGENT_PC_USE_RUNNER_RECEIPT_FILE_NAME);
        let archive_receipt_path = receipt_dir.join(format!(
            "zed-pc-use-runner-receipt-{}.json",
            current_epoch_millis()
        ));

        Self {
            root_mode,
            project_root,
            allowed_root,
            pc_use_root,
            latest_queue_path,
            receipt_dir,
            latest_receipt_path,
            archive_receipt_path,
        }
    }

    fn validate_managed_paths(&self) -> Result<(), String> {
        for path in [
            &self.latest_queue_path,
            &self.receipt_dir,
            &self.latest_receipt_path,
            &self.archive_receipt_path,
        ] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing PC-use runner path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }
        Ok(())
    }

    fn receipt(&self, include_payload_packet: bool) -> Value {
        let queue = self.latest_queue_summary(include_payload_packet);
        let checks = vec![
            gate_check(
                "queue.latest_payload",
                "Latest managed PC-use payload",
                matches!(queue.state, QueueState::Ready),
                Some(self.latest_queue_path.clone()),
                if matches!(queue.state, QueueState::Missing) {
                    "queue_missing"
                } else {
                    "queue_invalid"
                },
                "Queue a valid Zed PC-use payload before requesting a run.",
            ),
            gate_check(
                "execution.future_importer",
                "Future Zed-window PC-use importer",
                false,
                None,
                "executor_pending",
                "A future Zed-window importer/executor must consume this receipt before any screenshot, focus, click, or type action can run.",
            ),
        ];
        let queue_missing = gate_issues(&checks, "queue_missing");
        let queue_invalid = gate_issues(&checks, "queue_invalid");
        let executor_pending = gate_issues(&checks, "executor_pending");
        let queue_blockers = [queue_missing.clone(), queue_invalid.clone()].concat();
        let outcome = if !queue_missing.is_empty() {
            "blocked_missing_queue"
        } else if !queue_invalid.is_empty() {
            "blocked_invalid_queue"
        } else {
            "ready_future_executor_pending"
        };

        serde_json::json!({
            "schema": AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA,
            "result": {
                "generated_at_ms": current_epoch_millis(),
                "outcome": outcome,
                "root_mode": self.root_mode_label(),
                "future_executor_enabled": false,
                "screenshot_taken": false,
                "zed_focus_changed": false,
                "mouse_dispatched": false,
                "keyboard_dispatched": false,
                "process_launched": false,
                "next_actions": runner_next_actions(outcome),
            },
            "roots": {
                "project_root": self.project_root.as_ref().map(path_string),
                "allowed_root": path_string(&self.allowed_root),
                "pc_use_root": path_string(&self.pc_use_root),
                "latest_queue_path": path_string(&self.latest_queue_path),
                "receipt_dir": path_string(&self.receipt_dir),
                "latest_receipt_path": path_string(&self.latest_receipt_path),
                "archive_receipt_path": path_string(&self.archive_receipt_path),
            },
            "queue": queue.value,
            "checks": checks,
            "queue_blockers": queue_blockers,
            "executor_pending": executor_pending,
            "safety": {
                "permission_prompt_required": true,
                "receipt_only": true,
                "write_scope": "managed Zed data roots or workspace tools roots only",
                "takes_screenshot": false,
                "focuses_zed": false,
                "dispatches_mouse": false,
                "dispatches_keyboard": false,
                "launches_process": false,
                "os_wide_desktop_control": false,
            }
        })
    }

    fn latest_queue_summary(&self, include_payload_packet: bool) -> QueueSummary {
        let bytes = match fs::read(&self.latest_queue_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return QueueSummary {
                    state: QueueState::Missing,
                    value: serde_json::json!({
                        "state": "missing",
                        "path": path_string(&self.latest_queue_path),
                    }),
                };
            }
            Err(error) => {
                return QueueSummary {
                    state: QueueState::Invalid,
                    value: serde_json::json!({
                        "state": "read_error",
                        "path": path_string(&self.latest_queue_path),
                        "details": error.to_string(),
                    }),
                };
            }
        };

        let value = match serde_json::from_slice::<Value>(&bytes) {
            Ok(value) => value,
            Err(error) => {
                return QueueSummary {
                    state: QueueState::Invalid,
                    value: serde_json::json!({
                        "state": "parse_error",
                        "path": path_string(&self.latest_queue_path),
                        "bytes": bytes.len(),
                        "details": error.to_string(),
                    }),
                };
            }
        };

        let queue_schema_ok = value
            .get("schema")
            .and_then(Value::as_str)
            .is_some_and(|schema| schema == AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA);
        let payload_schema_ok = value
            .pointer("/payload_packet/schema")
            .and_then(Value::as_str)
            .is_some_and(|schema| schema == AGENT_PC_USE_PAYLOAD_SCHEMA);
        let action = value
            .pointer("/payload_packet/payload/action")
            .and_then(Value::as_str);
        let action_ok = action.is_some_and(is_supported_action);
        let dispatches_input = value
            .pointer("/payload_packet/safety/dispatches_input")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let target_snapshot_id = value
            .pointer("/payload_packet/payload/target_snapshot_id")
            .and_then(Value::as_str);
        let target_snapshot_id_present =
            target_snapshot_id.is_some_and(|snapshot_id| !snapshot_id.trim().is_empty());
        let state = if queue_schema_ok && payload_schema_ok && action_ok && !dispatches_input {
            QueueState::Ready
        } else {
            QueueState::Invalid
        };

        let mut summary = serde_json::json!({
            "state": if matches!(state, QueueState::Ready) { "ready" } else { "invalid_schema" },
            "path": path_string(&self.latest_queue_path),
            "bytes": bytes.len(),
            "queue_schema_ok": queue_schema_ok,
            "payload_schema_ok": payload_schema_ok,
            "action_ok": action_ok,
            "dispatches_input": dispatches_input,
            "action": action,
            "surface": value.pointer("/payload_packet/payload/surface").cloned(),
            "target_id_present": value
                .pointer("/payload_packet/payload/target_id")
                .and_then(Value::as_str)
                .is_some_and(|target_id| !target_id.trim().is_empty()),
            "target_snapshot_id": target_snapshot_id,
            "target_snapshot_id_present": target_snapshot_id_present,
            "target_reference": value.pointer("/payload_packet/payload/target_reference").cloned(),
            "queued_at_ms": value.get("queued_at_ms").cloned(),
            "source_tool": value.get("source_tool").cloned(),
        });

        if include_payload_packet {
            summary["payload_packet"] = value.get("payload_packet").cloned().unwrap_or(Value::Null);
        }

        QueueSummary {
            state,
            value: summary,
        }
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            AgentPcUsePayloadQueueRootMode::Workspace if self.project_root.is_some() => "workspace",
            AgentPcUsePayloadQueueRootMode::Workspace => "zed_data_fallback",
            AgentPcUsePayloadQueueRootMode::ZedData => "zed_data",
        }
    }
}

struct QueueSummary {
    state: QueueState,
    value: Value,
}

#[derive(Clone, Copy)]
enum QueueState {
    Ready,
    Missing,
    Invalid,
}

fn gate_check(
    id: &str,
    label: &str,
    ok: bool,
    path: Option<PathBuf>,
    category: &str,
    required_action: &str,
) -> Value {
    serde_json::json!({
        "id": id,
        "label": label,
        "ok": ok,
        "path": path.map(path_string),
        "category": category,
        "required_action": required_action,
    })
}

fn gate_issues(checks: &[Value], category: &str) -> Vec<Value> {
    checks
        .iter()
        .filter(|check| {
            !check.get("ok").and_then(Value::as_bool).unwrap_or(false)
                && check.get("category").and_then(Value::as_str) == Some(category)
        })
        .cloned()
        .collect()
}

fn is_supported_action(action: &str) -> bool {
    matches!(
        action,
        "screenshot" | "focus" | "click" | "type_text" | "inspect_ui"
    )
}

fn runner_next_actions(outcome: &str) -> Vec<&'static str> {
    match outcome {
        "ready_future_executor_pending" => vec![
            "Keep this receipt as the permissioned handoff for the future Zed-window PC-use executor.",
            "Do not dispatch screenshot, focus, click, or type actions until a future executor consumes this receipt and emits its own receipt.",
        ],
        "blocked_missing_queue" => vec![
            "Queue a valid Zed PC-use payload before requesting the runner gate.",
            "Keep OS-wide desktop automation blocked by default.",
        ],
        _ => vec![
            "Regenerate and queue the PC-use payload before requesting the runner gate again.",
            "Do not run future PC-use execution against invalid or unmanaged queue files.",
        ],
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

fn current_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}
