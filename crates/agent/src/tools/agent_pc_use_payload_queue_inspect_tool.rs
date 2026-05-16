use super::agent_pc_use_payload_tool::{
    AGENT_PC_USE_PAYLOAD_QUEUE_FILE_NAME, AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA,
    AGENT_PC_USE_PAYLOAD_SCHEMA, AgentPcUsePayloadQueueRootMode,
};
use crate::{AgentTool, ToolCallEventStream, ToolInput};
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

pub const AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME: &str = "inspect_zed_pc_use_payload_queue";
pub const AGENT_PC_USE_PAYLOAD_QUEUE_INSPECTION_SCHEMA: &str =
    "zed.agent_plugins.pc_use.action_payload_queue_inspection.v1";

/// Inspects the managed Zed-window PC-use payload queue without executing desktop automation.
///
/// This read-only tool validates the latest queued payload handoff, managed queue location, and
/// safety metadata. It never writes files, takes screenshots, focuses Zed, dispatches input,
/// launches processes, or controls the OS desktop.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPcUsePayloadQueueInspectToolInput {
    /// Prefer workspace-local queue under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentPcUsePayloadQueueRootMode,
    /// Include the full queued payload packet in the returned JSON.
    pub include_payload_packet: bool,
}

impl Default for AgentPcUsePayloadQueueInspectToolInput {
    fn default() -> Self {
        Self {
            root_mode: AgentPcUsePayloadQueueRootMode::Workspace,
            include_payload_packet: false,
        }
    }
}

pub struct AgentPcUsePayloadQueueInspectTool {
    project: Entity<Project>,
}

impl AgentPcUsePayloadQueueInspectTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentPcUsePayloadQueueInspectTool {
    type Input = AgentPcUsePayloadQueueInspectToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Inspect Zed PC-use payload queue".into()
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
            let inspector = AgentPcUsePayloadQueueInspector::new(project_root, input.root_mode);
            let result = inspector.inspect(input.include_payload_packet);
            let status = result
                .pointer("/result/status")
                .and_then(Value::as_str)
                .unwrap_or("inspected");
            let output = serde_json::to_string_pretty(&result)
                .map_err(|error| format!("Failed to serialize PC-use queue inspection: {error}"))?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(match status {
                "ready_for_future_pc_use_import" => "Zed PC-use queue ready",
                "missing_queue" => "Zed PC-use payload queue is empty",
                "invalid_queue" => "Zed PC-use payload queue needs fixes",
                _ => "Inspected Zed PC-use payload queue",
            }));

            Ok(output)
        })
    }
}

struct AgentPcUsePayloadQueueInspector {
    root_mode: AgentPcUsePayloadQueueRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    pc_use_root: PathBuf,
    queue_dir: PathBuf,
    latest_path: PathBuf,
}

impl AgentPcUsePayloadQueueInspector {
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
        let queue_dir = pc_use_root.join("payloads");
        let latest_path = queue_dir.join(AGENT_PC_USE_PAYLOAD_QUEUE_FILE_NAME);

        Self {
            root_mode,
            project_root,
            allowed_root,
            pc_use_root,
            queue_dir,
            latest_path,
        }
    }

    fn inspect(&self, include_payload_packet: bool) -> Value {
        let queue_path_valid = self.latest_path.starts_with(&self.allowed_root);
        let queue = if queue_path_valid {
            self.latest_queue_summary(include_payload_packet)
        } else {
            QueueSummary {
                state: QueueState::InvalidPath,
                value: serde_json::json!({
                    "state": "invalid_path",
                    "path": path_string(&self.latest_path),
                    "details": "Latest queue path is outside the managed PC-use root."
                }),
            }
        };
        let status = match queue.state {
            QueueState::Ready => "ready_for_future_pc_use_import",
            QueueState::Missing => "missing_queue",
            QueueState::Invalid | QueueState::InvalidPath => "invalid_queue",
        };

        serde_json::json!({
            "schema": AGENT_PC_USE_PAYLOAD_QUEUE_INSPECTION_SCHEMA,
            "result": {
                "generated_at_ms": current_epoch_millis(),
                "status": status,
                "root_mode": self.root_mode_label(),
                "queue_path_valid": queue_path_valid,
                "ready_for_future_import": status == "ready_for_future_pc_use_import",
                "next_actions": next_actions(status),
            },
            "roots": {
                "project_root": self.project_root.as_ref().map(path_string),
                "allowed_root": path_string(&self.allowed_root),
                "pc_use_root": path_string(&self.pc_use_root),
                "queue_dir": path_string(&self.queue_dir),
                "latest_queue_path": path_string(&self.latest_path),
            },
            "queue": queue.value,
            "required_before_execution": [
                "future PC-use importer confirms this queue item is intended for the active Zed window",
                "fresh Zed UI inspection receipt provides the target id for focus, click, or type actions",
                "user-visible permission gate is unlocked",
                "future executor emits a schema-versioned receipt after execution or block"
            ],
            "safety": {
                "read_only": true,
                "writes_files": false,
                "takes_screenshot": false,
                "focuses_zed": false,
                "dispatches_mouse": false,
                "dispatches_keyboard": false,
                "launches_process": false,
                "os_wide_desktop_control": false,
                "future_execution_requires_permission": true,
                "future_execution_requires_receipt": true,
            }
        })
    }

    fn latest_queue_summary(&self, include_payload_packet: bool) -> QueueSummary {
        let bytes = match fs::read(&self.latest_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return QueueSummary {
                    state: QueueState::Missing,
                    value: serde_json::json!({
                        "state": "missing",
                        "path": path_string(&self.latest_path),
                        "details": "No Zed PC-use payload has been queued yet."
                    }),
                };
            }
            Err(error) => {
                return QueueSummary {
                    state: QueueState::Invalid,
                    value: serde_json::json!({
                        "state": "read_error",
                        "path": path_string(&self.latest_path),
                        "error": error.to_string(),
                    }),
                };
            }
        };

        let parsed = match serde_json::from_slice::<Value>(&bytes) {
            Ok(parsed) => parsed,
            Err(error) => {
                return QueueSummary {
                    state: QueueState::Invalid,
                    value: serde_json::json!({
                        "state": "parse_error",
                        "path": path_string(&self.latest_path),
                        "byte_len": bytes.len(),
                        "error": error.to_string(),
                    }),
                };
            }
        };

        let schema = parsed.get("schema").and_then(Value::as_str);
        let payload_schema = parsed
            .pointer("/payload_packet/schema")
            .and_then(Value::as_str);
        let action = parsed
            .pointer("/payload_packet/payload/action")
            .and_then(Value::as_str);
        let surface = parsed
            .pointer("/payload_packet/payload/surface")
            .and_then(Value::as_str);
        let target_id_present = parsed
            .pointer("/payload_packet/payload/target_id")
            .and_then(Value::as_str)
            .is_some_and(|target_id| !target_id.trim().is_empty());
        let target_snapshot_id = parsed
            .pointer("/payload_packet/payload/target_snapshot_id")
            .and_then(Value::as_str);
        let target_snapshot_id_present =
            target_snapshot_id.is_some_and(|snapshot_id| !snapshot_id.trim().is_empty());
        let safety_dispatches_input = parsed
            .pointer("/payload_packet/safety/dispatches_input")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let schema_valid = schema == Some(AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA);
        let payload_schema_valid = payload_schema == Some(AGENT_PC_USE_PAYLOAD_SCHEMA);
        let safe_for_handoff = !safety_dispatches_input;
        let state = if schema_valid && payload_schema_valid && safe_for_handoff {
            QueueState::Ready
        } else {
            QueueState::Invalid
        };

        QueueSummary {
            state,
            value: serde_json::json!({
                "state": if matches!(state, QueueState::Ready) { "ready" } else { "invalid" },
                "path": path_string(&self.latest_path),
                "byte_len": bytes.len(),
                "schema": schema,
                "schema_valid": schema_valid,
                "payload_schema": payload_schema,
                "payload_schema_valid": payload_schema_valid,
                "action": action,
                "surface": surface,
                "target_id_present": target_id_present,
                "target_snapshot_id": target_snapshot_id,
                "target_snapshot_id_present": target_snapshot_id_present,
                "target_reference": parsed.pointer("/payload_packet/payload/target_reference").cloned(),
                "safe_for_handoff": safe_for_handoff,
                "payload_packet": include_payload_packet.then(|| {
                    parsed.get("payload_packet").cloned().unwrap_or(Value::Null)
                }),
            }),
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
    InvalidPath,
}

fn next_actions(status: &str) -> Vec<&'static str> {
    match status {
        "ready_for_future_pc_use_import" => vec![
            "Keep the queue item as handoff metadata until a future PC-use importer and executor exist.",
            "Require fresh UI inspection, explicit permission, and a receipt before any future action runs.",
        ],
        "missing_queue" => vec![
            "Queue a payload with queue_zed_pc_use_action_payload before future PC-use import.",
            "Keep OS-wide desktop automation blocked by default.",
        ],
        _ => vec![
            "Regenerate the payload with compose_zed_pc_use_action_payload and queue it again.",
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
