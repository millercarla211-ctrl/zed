use super::agent_browser_payload_tool::{
    AGENT_BROWSER_PAYLOAD_QUEUE_FILE_NAME, AGENT_BROWSER_PAYLOAD_QUEUE_ITEM_SCHEMA,
    AgentBrowserPayloadQueueRootMode,
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

pub const AGENT_BROWSER_PAYLOAD_QUEUE_INSPECT_TOOL_NAME: &str =
    "inspect_agent_browser_payload_queue";
pub const AGENT_BROWSER_PAYLOAD_QUEUE_INSPECTION_SCHEMA: &str =
    "zed.agent_plugins.browser_action_payload_queue_inspection.v1";

const WEB_PREVIEW_EXECUTOR_PAYLOAD_SCHEMA: &str =
    "zed.web_preview.agent_browser_executor_payload.v1";

/// Inspects the managed WebPreview Browser payload queue without importing or executing input.
///
/// This read-only tool validates the latest queued in-app Browser payload handoff, managed queue
/// location, action metadata, and WebPreview import requirements. It never writes files, imports
/// payloads into WebPreview, takes screenshots, dispatches mouse/keyboard/wheel input, launches
/// browsers, or touches real browser profiles.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentBrowserPayloadQueueInspectToolInput {
    /// Prefer workspace-local queue under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentBrowserPayloadQueueRootMode,
    /// Include the full queued payload packet in the returned JSON.
    pub include_payload_packet: bool,
}

impl Default for AgentBrowserPayloadQueueInspectToolInput {
    fn default() -> Self {
        Self {
            root_mode: AgentBrowserPayloadQueueRootMode::Workspace,
            include_payload_packet: false,
        }
    }
}

pub struct AgentBrowserPayloadQueueInspectTool {
    project: Entity<Project>,
}

impl AgentBrowserPayloadQueueInspectTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentBrowserPayloadQueueInspectTool {
    type Input = AgentBrowserPayloadQueueInspectToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_BROWSER_PAYLOAD_QUEUE_INSPECT_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Inspect browser payload queue".into()
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
            let inspector = AgentBrowserPayloadQueueInspector::new(project_root, input.root_mode);
            let result = inspector.inspect(input.include_payload_packet);
            let status = result
                .pointer("/result/status")
                .and_then(Value::as_str)
                .unwrap_or("inspected");
            let output = serde_json::to_string_pretty(&result).map_err(|error| {
                format!("Failed to serialize browser queue inspection: {error}")
            })?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(match status {
                "ready_for_webpreview_import" => "Browser payload queue ready",
                "missing_queue" => "Browser payload queue is empty",
                "invalid_queue" => "Browser payload queue needs fixes",
                _ => "Inspected browser payload queue",
            }));

            Ok(output)
        })
    }
}

struct AgentBrowserPayloadQueueInspector {
    root_mode: AgentBrowserPayloadQueueRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    queue_dir: PathBuf,
    latest_path: PathBuf,
}

impl AgentBrowserPayloadQueueInspector {
    fn new(project_root: Option<PathBuf>, root_mode: AgentBrowserPayloadQueueRootMode) -> Self {
        let use_workspace = matches!(root_mode, AgentBrowserPayloadQueueRootMode::Workspace)
            && project_root.is_some();
        let allowed_root = if use_workspace {
            project_root
                .as_ref()
                .expect("workspace root checked above")
                .join("tools")
        } else {
            data_dir().join("agent-plugins")
        };
        let queue_dir = if use_workspace {
            allowed_root.join("agent-plugins").join("browser-payloads")
        } else {
            allowed_root.join("browser-payloads")
        };
        let latest_path = queue_dir.join(AGENT_BROWSER_PAYLOAD_QUEUE_FILE_NAME);

        Self {
            root_mode,
            project_root,
            allowed_root,
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
                    "details": "Latest browser queue path is outside the managed root."
                }),
            }
        };
        let status = match queue.state {
            QueueState::Ready => "ready_for_webpreview_import",
            QueueState::Missing => "missing_queue",
            QueueState::Invalid | QueueState::InvalidPath => "invalid_queue",
        };

        serde_json::json!({
            "schema": AGENT_BROWSER_PAYLOAD_QUEUE_INSPECTION_SCHEMA,
            "result": {
                "generated_at_ms": current_epoch_millis(),
                "status": status,
                "root_mode": self.root_mode_label(),
                "queue_path_valid": queue_path_valid,
                "ready_for_webpreview_import": status == "ready_for_webpreview_import",
                "next_actions": next_actions(status),
            },
            "roots": {
                "project_root": self.project_root.as_ref().map(path_string),
                "allowed_root": path_string(&self.allowed_root),
                "queue_dir": path_string(&self.queue_dir),
                "latest_queue_path": path_string(&self.latest_path),
            },
            "queue": queue.value,
            "required_before_execution": [
                "WebPreview imports this managed queue item explicitly",
                "user-visible WebPreview interactive permission is unlocked",
                "fresh action-specific preflight and native trace receipt exist",
                "dispatch QA checklist passes",
                "WebPreview emits an executor receipt after any dispatch"
            ],
            "safety": {
                "read_only": true,
                "writes_files": false,
                "imports_into_webpreview": false,
                "takes_screenshot": false,
                "dispatches_mouse": false,
                "dispatches_keyboard": false,
                "dispatches_wheel": false,
                "launches_browser": false,
                "touches_real_browser_profiles": false,
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
                        "details": "No WebPreview Browser payload has been queued yet."
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
        let packet_schema = parsed
            .pointer("/payload_packet/schema")
            .and_then(Value::as_str);
        let action = parsed
            .pointer("/payload_packet/payload/action")
            .and_then(Value::as_str)
            .or_else(|| parsed.pointer("/metadata/action").and_then(Value::as_str));
        let schema_valid = schema == Some(AGENT_BROWSER_PAYLOAD_QUEUE_ITEM_SCHEMA)
            && packet_schema == Some(WEB_PREVIEW_EXECUTOR_PAYLOAD_SCHEMA)
            && action.is_some();
        let state = if schema_valid {
            QueueState::Ready
        } else {
            QueueState::Invalid
        };
        let mut value = serde_json::json!({
            "state": if schema_valid { "ready" } else { "schema_mismatch" },
            "path": path_string(&self.latest_path),
            "byte_len": bytes.len(),
            "schema": schema,
            "expected_schema": AGENT_BROWSER_PAYLOAD_QUEUE_ITEM_SCHEMA,
            "executor_payload_schema": packet_schema,
            "expected_executor_payload_schema": WEB_PREVIEW_EXECUTOR_PAYLOAD_SCHEMA,
            "action": action,
            "queued_at_ms": parsed.get("queued_at_ms").cloned(),
            "source_tool": parsed.get("source_tool").and_then(Value::as_str),
            "metadata": parsed.get("metadata").cloned(),
        });
        if include_payload_packet {
            value["payload_packet"] = parsed.get("payload_packet").cloned().unwrap_or(Value::Null);
        }

        QueueSummary { state, value }
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            AgentBrowserPayloadQueueRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            AgentBrowserPayloadQueueRootMode::Workspace => "zed_data_fallback",
            AgentBrowserPayloadQueueRootMode::ZedData => "zed_data",
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
        "ready_for_webpreview_import" => vec![
            "Import the managed queue item from WebPreview only when browser control is intended.",
            "Run the matching WebPreview preflight and executor only after the permission gate is unlocked.",
        ],
        "missing_queue" => vec![
            "Queue a payload with queue_agent_browser_action_payload before importing from WebPreview.",
        ],
        _ => vec![
            "Regenerate the payload with compose_agent_browser_action_payload, then queue it again.",
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
