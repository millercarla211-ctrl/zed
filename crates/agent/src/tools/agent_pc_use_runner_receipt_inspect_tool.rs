use super::{
    agent_pc_use_payload_tool::AgentPcUsePayloadQueueRootMode,
    agent_pc_use_runner_gate_tool::{
        AGENT_PC_USE_RUNNER_RECEIPT_FILE_NAME, AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA,
    },
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

pub const AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME: &str =
    "inspect_zed_pc_use_runner_receipts";
pub const AGENT_PC_USE_RUNNER_RECEIPT_INSPECTION_SCHEMA: &str =
    "zed.agent_plugins.pc_use.runner_receipt_inspection.v1";

const DEFAULT_MAX_ENTRIES: usize = 8;
const MAX_ENTRIES: usize = 50;

/// Reads recent Zed-window PC-use runner-gate receipts without controlling the desktop.
///
/// This read-only tool inspects managed receipt files written by
/// `request_zed_pc_use_payload_run`. It never takes screenshots, focuses Zed, dispatches input,
/// launches processes, or controls OS-wide desktop state.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPcUseRunnerReceiptInspectToolInput {
    /// Prefer workspace-local receipts under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentPcUsePayloadQueueRootMode,
    /// Maximum recent receipt files to summarize.
    pub max_entries: usize,
    /// Include full parsed receipt JSON for recent receipts.
    pub include_receipts: bool,
}

impl Default for AgentPcUseRunnerReceiptInspectToolInput {
    fn default() -> Self {
        Self {
            root_mode: AgentPcUsePayloadQueueRootMode::Workspace,
            max_entries: DEFAULT_MAX_ENTRIES,
            include_receipts: false,
        }
    }
}

pub struct AgentPcUseRunnerReceiptInspectTool {
    project: Entity<Project>,
}

impl AgentPcUseRunnerReceiptInspectTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentPcUseRunnerReceiptInspectTool {
    type Input = AgentPcUseRunnerReceiptInspectToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Inspect Zed PC-use runner receipts".into()
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
            let inspector = AgentPcUseRunnerReceiptInspector::new(project_root, input.root_mode);
            inspector.validate_managed_paths()?;
            let result = inspector.inspect(&input);
            let status = result
                .pointer("/result/status")
                .and_then(Value::as_str)
                .unwrap_or("inspected");
            let output = serde_json::to_string_pretty(&result).map_err(|error| {
                format!("Failed to serialize PC-use receipt inspection: {error}")
            })?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(match status {
                "has_ready_runner_receipt" => "Zed PC-use runner receipt is ready",
                "has_blocked_runner_receipt" => "Zed PC-use runner receipt is blocked",
                "empty" => "No Zed PC-use runner receipts yet",
                _ => "Inspected Zed PC-use runner receipts",
            }));

            Ok(output)
        })
    }
}

struct AgentPcUseRunnerReceiptInspector {
    root_mode: AgentPcUsePayloadQueueRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    pc_use_root: PathBuf,
    receipt_dir: PathBuf,
    latest_receipt_path: PathBuf,
}

impl AgentPcUseRunnerReceiptInspector {
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
        let receipt_dir = pc_use_root.join("receipts");
        let latest_receipt_path = receipt_dir.join(AGENT_PC_USE_RUNNER_RECEIPT_FILE_NAME);

        Self {
            root_mode,
            project_root,
            allowed_root,
            pc_use_root,
            receipt_dir,
            latest_receipt_path,
        }
    }

    fn validate_managed_paths(&self) -> Result<(), String> {
        for path in [
            &self.pc_use_root,
            &self.receipt_dir,
            &self.latest_receipt_path,
        ] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing PC-use receipt inspection path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }
        Ok(())
    }

    fn inspect(&self, input: &AgentPcUseRunnerReceiptInspectToolInput) -> Value {
        let max_entries = input.max_entries.clamp(1, MAX_ENTRIES);
        let entries = match self.read_receipt_entries() {
            Ok(entries) => entries,
            Err(error) => {
                return serde_json::json!({
                    "schema": AGENT_PC_USE_RUNNER_RECEIPT_INSPECTION_SCHEMA,
                    "result": {
                        "generated_at_ms": current_epoch_millis(),
                        "status": "read_error",
                        "root_mode": self.root_mode_label(),
                        "receipt_dir": path_string(&self.receipt_dir),
                        "details": error,
                    },
                    "latest_receipt": null,
                    "receipts": [],
                    "safety": self.safety(),
                });
            }
        };

        let receipts = entries
            .iter()
            .take(max_entries)
            .map(|entry| summarize_receipt_entry(entry, input.include_receipts))
            .collect::<Vec<_>>();
        let latest_receipt = receipts.first().cloned().unwrap_or(Value::Null);
        let latest_outcome = latest_receipt
            .pointer("/read/outcome")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let status = if receipts.is_empty() {
            "empty"
        } else if latest_outcome == "ready_future_executor_pending" {
            "has_ready_runner_receipt"
        } else {
            "has_blocked_runner_receipt"
        };

        serde_json::json!({
            "schema": AGENT_PC_USE_RUNNER_RECEIPT_INSPECTION_SCHEMA,
            "result": {
                "generated_at_ms": current_epoch_millis(),
                "status": status,
                "root_mode": self.root_mode_label(),
                "receipt_dir": path_string(&self.receipt_dir),
                "latest_receipt_path": path_string(&self.latest_receipt_path),
                "max_entries": max_entries,
                "receipt_count": receipts.len(),
                "latest_outcome": latest_outcome,
            },
            "roots": {
                "project_root": self.project_root.as_ref().map(path_string),
                "allowed_root": path_string(&self.allowed_root),
                "pc_use_root": path_string(&self.pc_use_root),
                "receipt_dir": path_string(&self.receipt_dir),
                "latest_receipt_path": path_string(&self.latest_receipt_path),
            },
            "latest_receipt": latest_receipt,
            "receipts": receipts,
            "next_actions": next_actions(status, latest_outcome),
            "safety": self.safety(),
        })
    }

    fn read_receipt_entries(&self) -> Result<Vec<ReceiptEntry>, String> {
        let directory = match fs::read_dir(&self.receipt_dir) {
            Ok(directory) => directory,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(format!(
                    "Failed to read PC-use receipts directory {}: {error}",
                    self.receipt_dir.display()
                ));
            }
        };
        let mut entries = Vec::new();
        for entry in directory {
            let entry = entry.map_err(|error| {
                format!(
                    "Failed to read PC-use receipt directory entry {}: {error}",
                    self.receipt_dir.display()
                )
            })?;
            let path = entry.path();
            if !path.starts_with(&self.receipt_dir)
                || path.extension().and_then(|ext| ext.to_str()) != Some("json")
            {
                continue;
            }
            let Some(file_name) = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
            else {
                continue;
            };
            if file_name != AGENT_PC_USE_RUNNER_RECEIPT_FILE_NAME
                && !file_name.starts_with("zed-pc-use-runner-receipt-")
            {
                continue;
            }
            let metadata = entry.metadata().ok();
            let modified_ms = metadata
                .as_ref()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(system_time_ms)
                .unwrap_or_default();
            let bytes = metadata.map(|metadata| metadata.len()).unwrap_or_default();
            entries.push(ReceiptEntry {
                path,
                file_name: file_name.clone(),
                modified_ms,
                bytes,
                is_latest: file_name == AGENT_PC_USE_RUNNER_RECEIPT_FILE_NAME,
            });
        }

        entries.sort_by(|a, b| {
            b.modified_ms
                .cmp(&a.modified_ms)
                .then_with(|| b.is_latest.cmp(&a.is_latest))
                .then_with(|| b.file_name.cmp(&a.file_name))
        });
        Ok(entries)
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            AgentPcUsePayloadQueueRootMode::Workspace if self.project_root.is_some() => "workspace",
            AgentPcUsePayloadQueueRootMode::Workspace => "zed_data_fallback",
            AgentPcUsePayloadQueueRootMode::ZedData => "zed_data",
        }
    }

    fn safety(&self) -> Value {
        serde_json::json!({
            "read_only": true,
            "writes_files": false,
            "takes_screenshot": false,
            "focuses_zed": false,
            "dispatches_mouse": false,
            "dispatches_keyboard": false,
            "launches_process": false,
            "os_wide_desktop_control": false,
            "managed_root_only": true,
        })
    }
}

struct ReceiptEntry {
    path: PathBuf,
    file_name: String,
    modified_ms: u64,
    bytes: u64,
    is_latest: bool,
}

fn summarize_receipt_entry(entry: &ReceiptEntry, include_full_value: bool) -> Value {
    let parsed = read_json_summary(&entry.path);
    let mut summary = serde_json::json!({
        "kind": if entry.is_latest { "latest_receipt" } else { "archived_receipt" },
        "path": path_string(&entry.path),
        "file_name": entry.file_name.as_str(),
        "bytes": entry.bytes,
        "modified_at_ms": entry.modified_ms,
        "read": parsed,
    });
    if include_full_value {
        if let Some(value) = summary.pointer("/read/value").cloned() {
            summary["value"] = value;
        }
    }
    if let Some(read_object) = summary.get_mut("read").and_then(Value::as_object_mut) {
        read_object.remove("value");
    }
    summary
}

fn read_json_summary(path: &Path) -> Value {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return serde_json::json!({
                "ok": false,
                "state": "read_error",
                "details": error.to_string(),
            });
        }
    };
    let value = match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) => value,
        Err(error) => {
            return serde_json::json!({
                "ok": false,
                "state": "parse_error",
                "bytes": bytes.len(),
                "details": error.to_string(),
            });
        }
    };
    let schema = value
        .get("schema")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let schema_ok = schema == AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA;

    serde_json::json!({
        "ok": true,
        "state": if schema_ok { "ready" } else { "schema_mismatch" },
        "bytes": bytes.len(),
        "schema": schema,
        "schema_ok": schema_ok,
        "generated_at_ms": value.get("generated_at_ms").cloned(),
        "outcome": value.pointer("/result/outcome").and_then(Value::as_str),
        "action": value.pointer("/queue/action").and_then(Value::as_str),
        "surface": value.pointer("/queue/surface").and_then(Value::as_str),
        "target_id_present": value.pointer("/queue/target_id_present").and_then(Value::as_bool),
        "target_snapshot_id": value.pointer("/queue/target_snapshot_id").and_then(Value::as_str),
        "target_snapshot_id_present": value.pointer("/queue/target_snapshot_id_present").and_then(Value::as_bool),
        "target_reference": value.pointer("/queue/target_reference").cloned(),
        "queue_blocker_count": value.pointer("/queue_blockers").and_then(Value::as_array).map(Vec::len),
        "executor_pending_count": value.pointer("/executor_pending").and_then(Value::as_array).map(Vec::len),
        "future_executor_pending": value.pointer("/result/outcome").and_then(Value::as_str) == Some("ready_future_executor_pending"),
        "future_executor_enabled": value.pointer("/result/future_executor_enabled").and_then(Value::as_bool),
        "takes_screenshot": value.pointer("/safety/takes_screenshot").and_then(Value::as_bool),
        "focuses_zed": value.pointer("/safety/focuses_zed").and_then(Value::as_bool),
        "dispatches_mouse": value.pointer("/safety/dispatches_mouse").and_then(Value::as_bool),
        "dispatches_keyboard": value.pointer("/safety/dispatches_keyboard").and_then(Value::as_bool),
        "launches_process": value.pointer("/safety/launches_process").and_then(Value::as_bool),
        "os_wide_desktop_control": value.pointer("/safety/os_wide_desktop_control").and_then(Value::as_bool),
        "value": value.clone(),
    })
}

fn next_actions(status: &str, latest_outcome: &str) -> Vec<&'static str> {
    match status {
        "has_ready_runner_receipt" => vec![
            "Keep the latest receipt as auditable readiness evidence until a future Zed-window importer/executor exists.",
            "Do not enable screenshots, focus, click, or type until the future executor can consume this receipt and emit its own after-action receipt.",
        ],
        "has_blocked_runner_receipt" if latest_outcome == "blocked_missing_queue" => vec![
            "Queue a PC-use payload with queue_zed_pc_use_action_payload, then request the runner gate again.",
            "Keep OS-wide desktop automation blocked by default.",
        ],
        "has_blocked_runner_receipt" => vec![
            "Inspect the latest receipt blockers and regenerate the PC-use payload if schema or safety checks failed.",
            "Do not run future PC-use execution against a blocked receipt.",
        ],
        _ => vec![
            "Queue a managed PC-use payload, then run request_zed_pc_use_payload_run to create the first receipt.",
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

fn system_time_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
}
