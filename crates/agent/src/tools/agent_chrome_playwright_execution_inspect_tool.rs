use super::agent_chrome_payload_tool::AgentChromePayloadQueueRootMode;
use super::agent_chrome_playwright_adapter_tool::AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA;
use super::agent_chrome_playwright_invoke_tool::AGENT_CHROME_PLAYWRIGHT_RUN_REQUEST_SCHEMA;
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

pub const AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME: &str =
    "inspect_managed_chrome_playwright_executions";
pub const AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_RESULT_SCHEMA: &str =
    "zed.agent_plugins.managed_chrome_playwright_execution_inspection.v1";

const DEFAULT_MAX_ENTRIES: usize = 8;
const MAX_ENTRIES: usize = 50;

/// Reads recent managed Chrome Playwright request and receipt files without running Chrome.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentChromePlaywrightExecutionInspectToolInput {
    /// Prefer workspace-local executions under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentChromePayloadQueueRootMode,
    /// Maximum recent request and receipt files to summarize.
    pub max_entries: usize,
    /// Include full parsed request JSON for recent run requests.
    pub include_requests: bool,
    /// Include full parsed receipt JSON for recent execution receipts.
    pub include_receipts: bool,
}

impl Default for AgentChromePlaywrightExecutionInspectToolInput {
    fn default() -> Self {
        Self {
            root_mode: AgentChromePayloadQueueRootMode::Workspace,
            max_entries: DEFAULT_MAX_ENTRIES,
            include_requests: false,
            include_receipts: false,
        }
    }
}

pub struct AgentChromePlaywrightExecutionInspectTool {
    project: Entity<Project>,
}

impl AgentChromePlaywrightExecutionInspectTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentChromePlaywrightExecutionInspectTool {
    type Input = AgentChromePlaywrightExecutionInspectToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Inspect Chrome adapter executions".into()
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
            let inspector =
                ManagedChromePlaywrightExecutionInspector::new(project_root, input.root_mode);
            inspector.validate_managed_paths()?;
            let result = inspector.inspect(&input);
            let status = result
                .pointer("/result/status")
                .and_then(Value::as_str)
                .unwrap_or("inspected");
            let output = serde_json::to_string_pretty(&result).map_err(|error| {
                format!("Failed to serialize Chrome adapter execution inspection: {error}")
            })?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(match status {
                "has_recent_execution_receipts" => "Chrome adapter execution receipts found",
                "has_requests_without_receipts" => "Chrome adapter requests need receipts",
                "empty" => "No Chrome adapter executions yet",
                _ => "Inspected Chrome adapter executions",
            }));

            Ok(output)
        })
    }
}

struct ManagedChromePlaywrightExecutionInspector {
    root_mode: AgentChromePayloadQueueRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    plugin_root: PathBuf,
    execution_dir: PathBuf,
}

impl ManagedChromePlaywrightExecutionInspector {
    fn new(project_root: Option<PathBuf>, root_mode: AgentChromePayloadQueueRootMode) -> Self {
        let use_workspace = matches!(root_mode, AgentChromePayloadQueueRootMode::Workspace)
            && project_root.is_some();
        let zed_plugin_root = data_dir().join("agent-plugins");
        let (allowed_root, plugin_root) = if use_workspace {
            let workspace_root = project_root.as_ref().expect("workspace root checked above");
            let tools_root = workspace_root.join("tools");
            (tools_root.clone(), tools_root.join("agent-plugins"))
        } else {
            (zed_plugin_root.clone(), zed_plugin_root.clone())
        };
        let execution_dir = plugin_root.join("chrome-executions");

        Self {
            root_mode,
            project_root,
            allowed_root,
            plugin_root,
            execution_dir,
        }
    }

    fn validate_managed_paths(&self) -> Result<(), String> {
        for path in [&self.plugin_root, &self.execution_dir] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing Chrome execution inspection path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }
        Ok(())
    }

    fn inspect(&self, input: &AgentChromePlaywrightExecutionInspectToolInput) -> Value {
        let max_entries = input.max_entries.clamp(1, MAX_ENTRIES);
        let entries = match self.read_execution_entries() {
            Ok(entries) => entries,
            Err(error) => {
                return serde_json::json!({
                    "schema": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_RESULT_SCHEMA,
                    "result": {
                        "generated_at_ms": current_epoch_millis(),
                        "status": "read_error",
                        "root_mode": self.root_mode_label(),
                        "execution_dir": path_string(&self.execution_dir),
                        "details": error,
                    },
                    "latest_receipt": null,
                    "latest_request": null,
                    "receipts": [],
                    "requests": [],
                    "safety": self.safety(),
                });
            }
        };
        let receipts = entries
            .iter()
            .filter(|entry| entry.kind == ExecutionEntryKind::Receipt)
            .take(max_entries)
            .map(|entry| summarize_execution_entry(entry, input.include_receipts))
            .collect::<Vec<_>>();
        let requests = entries
            .iter()
            .filter(|entry| entry.kind == ExecutionEntryKind::Request)
            .take(max_entries)
            .map(|entry| summarize_execution_entry(entry, input.include_requests))
            .collect::<Vec<_>>();
        let latest_receipt = receipts.first().cloned().unwrap_or(Value::Null);
        let latest_request = requests.first().cloned().unwrap_or(Value::Null);
        let status = if !receipts.is_empty() {
            "has_recent_execution_receipts"
        } else if !requests.is_empty() {
            "has_requests_without_receipts"
        } else {
            "empty"
        };

        serde_json::json!({
            "schema": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_RESULT_SCHEMA,
            "result": {
                "generated_at_ms": current_epoch_millis(),
                "status": status,
                "root_mode": self.root_mode_label(),
                "execution_dir": path_string(&self.execution_dir),
                "max_entries": max_entries,
                "receipt_count": receipts.len(),
                "request_count": requests.len(),
            },
            "latest_receipt": latest_receipt,
            "latest_request": latest_request,
            "receipts": receipts,
            "requests": requests,
            "next_actions": next_actions(status),
            "safety": self.safety(),
        })
    }

    fn read_execution_entries(&self) -> Result<Vec<ExecutionEntry>, String> {
        let directory = match fs::read_dir(&self.execution_dir) {
            Ok(directory) => directory,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(format!(
                    "Failed to read Chrome executions directory {}: {error}",
                    self.execution_dir.display()
                ));
            }
        };
        let mut entries = Vec::new();
        for entry in directory {
            let entry = entry.map_err(|error| {
                format!(
                    "Failed to read Chrome execution directory entry {}: {error}",
                    self.execution_dir.display()
                )
            })?;
            let path = entry.path();
            if !path.starts_with(&self.execution_dir)
                || path.extension().and_then(|ext| ext.to_str()) != Some("json")
            {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let kind = if file_name.starts_with("managed-chrome-execution-receipt-") {
                ExecutionEntryKind::Receipt
            } else if file_name.starts_with("managed-chrome-run-request-") {
                ExecutionEntryKind::Request
            } else {
                continue;
            };
            let metadata = entry.metadata().ok();
            let modified_ms = metadata
                .as_ref()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(system_time_ms)
                .unwrap_or_default();
            let bytes = metadata.map(|metadata| metadata.len()).unwrap_or_default();
            entries.push(ExecutionEntry {
                kind,
                path,
                file_name: file_name.to_string(),
                modified_ms,
                bytes,
            });
        }

        entries.sort_by(|a, b| {
            b.modified_ms
                .cmp(&a.modified_ms)
                .then_with(|| b.file_name.cmp(&a.file_name))
        });
        Ok(entries)
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            AgentChromePayloadQueueRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            AgentChromePayloadQueueRootMode::Workspace => "zed_data_fallback",
            AgentChromePayloadQueueRootMode::ZedData => "zed_data",
        }
    }

    fn safety(&self) -> Value {
        serde_json::json!({
            "read_only": true,
            "launches_browser": false,
            "runs_node": false,
            "dispatches_input": false,
            "managed_root_only": true,
            "real_browser_profiles_touched": false,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExecutionEntryKind {
    Request,
    Receipt,
}

struct ExecutionEntry {
    kind: ExecutionEntryKind,
    path: PathBuf,
    file_name: String,
    modified_ms: u64,
    bytes: u64,
}

fn summarize_execution_entry(entry: &ExecutionEntry, include_full_value: bool) -> Value {
    let parsed = read_json_summary(&entry.path);
    let mut summary = serde_json::json!({
        "kind": match entry.kind {
            ExecutionEntryKind::Request => "request",
            ExecutionEntryKind::Receipt => "receipt",
        },
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
    let expected_schema = if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("managed-chrome-execution-receipt-"))
    {
        AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA
    } else {
        AGENT_CHROME_PLAYWRIGHT_RUN_REQUEST_SCHEMA
    };
    let schema_ok = schema == expected_schema;

    serde_json::json!({
        "ok": true,
        "state": if schema_ok { "ready" } else { "schema_mismatch" },
        "bytes": bytes.len(),
        "schema": schema,
        "schema_ok": schema_ok,
        "generated_at_ms": value.get("generated_at_ms").or_else(|| value.get("queued_at_ms")).cloned(),
        "source_tool": value.get("source_tool").and_then(Value::as_str),
        "outcome": value.get("outcome").and_then(Value::as_str),
        "action": value.get("action").and_then(Value::as_str).or_else(|| value.pointer("/payload_packet/payload/action").and_then(Value::as_str)),
        "url": value.get("url").and_then(Value::as_str).or_else(|| value.pointer("/payload_packet/payload/url").and_then(Value::as_str)),
        "title": value.get("title").and_then(Value::as_str),
        "screenshot": value.pointer("/artifacts/screenshot").and_then(Value::as_str),
        "screenshot_capture": screenshot_receipt_summary(&value),
        "viewport_change": viewport_receipt_summary(&value),
        "selector_wait": selector_wait_receipt_summary(&value),
        "inspect_element": inspect_element_receipt_summary(&value),
        "dom_snapshot": dom_snapshot_receipt_summary(&value),
        "runtime_events": runtime_events_receipt_summary(&value),
        "page_scripts_executed": value.pointer("/safety/page_scripts_executed").and_then(Value::as_bool),
        "error": value.get("error").and_then(Value::as_str),
        "value": value.clone(),
    })
}

fn screenshot_receipt_summary(value: &Value) -> Option<Value> {
    let screenshot = value.get("screenshot")?;
    Some(serde_json::json!({
        "path": screenshot.get("path").and_then(Value::as_str),
        "capture_target": screenshot.get("capture_target").and_then(Value::as_str),
        "selector": screenshot.get("selector").and_then(Value::as_str),
        "full_page": screenshot.get("full_page").and_then(Value::as_bool),
        "dimensions": screenshot.get("dimensions").cloned(),
    }))
}

fn viewport_receipt_summary(value: &Value) -> Option<Value> {
    let viewport = value.get("viewport")?;
    Some(serde_json::json!({
        "requested": viewport.get("requested").cloned(),
        "applied": viewport.get("applied").cloned(),
    }))
}

fn selector_wait_receipt_summary(value: &Value) -> Option<Value> {
    let wait = value.get("selector_wait")?;
    Some(serde_json::json!({
        "selector": wait.get("selector").and_then(Value::as_str),
        "state": wait.get("state").and_then(Value::as_str),
        "timeout_ms": wait.get("timeout_ms").and_then(Value::as_u64),
        "matched": wait.get("matched").and_then(Value::as_bool),
        "bounds": wait.get("bounds").cloned(),
    }))
}

fn inspect_element_receipt_summary(value: &Value) -> Option<Value> {
    let inspection = value.get("inspection")?;
    Some(serde_json::json!({
        "selector": inspection.get("selector").and_then(Value::as_str),
        "tag_name": inspection.pointer("/element/tag_name").and_then(Value::as_str),
        "visible": inspection.pointer("/element/visible").and_then(Value::as_bool),
        "text_truncated": inspection.pointer("/element/text_truncated").and_then(Value::as_bool),
        "bounds": inspection.pointer("/element/bounding_client_rect").cloned(),
    }))
}

fn dom_snapshot_receipt_summary(value: &Value) -> Option<Value> {
    let snapshot = value.get("dom_snapshot")?;
    Some(serde_json::json!({
        "selector": snapshot.get("selector").and_then(Value::as_str),
        "scoped": snapshot.get("scoped").and_then(Value::as_bool),
        "ready_state": snapshot.get("ready_state").and_then(Value::as_str),
        "url": snapshot.get("url").and_then(Value::as_str),
        "title": snapshot.get("title").and_then(Value::as_str),
        "counts": snapshot.get("counts").cloned(),
        "heading_count": snapshot.get("headings").and_then(Value::as_array).map(|items| items.len()),
        "link_count": snapshot.get("links").and_then(Value::as_array).map(|items| items.len()),
        "form_count": snapshot.get("forms").and_then(Value::as_array).map(|items| items.len()),
    }))
}

fn runtime_events_receipt_summary(value: &Value) -> Option<Value> {
    let events = value.get("runtime_events")?;
    Some(serde_json::json!({
        "observation_ms": events.get("observation_ms").and_then(Value::as_u64),
        "url": events.pointer("/page/url").and_then(Value::as_str),
        "ready_state": events.pointer("/page/ready_state").and_then(Value::as_str),
        "console_message_count": events.get("console_messages").and_then(Value::as_array).map(|items| items.len()),
        "page_error_count": events.get("page_errors").and_then(Value::as_array).map(|items| items.len()),
        "request_failure_count": events.get("request_failures").and_then(Value::as_array).map(|items| items.len()),
        "response_error_count": events.get("response_errors").and_then(Value::as_array).map(|items| items.len()),
        "resource_summary": events.pointer("/page/resource_summary").cloned(),
        "truncated": events.get("truncated").cloned(),
        "dropped_counts": events.get("dropped_counts").cloned(),
    }))
}

fn next_actions(status: &str) -> Vec<&'static str> {
    match status {
        "has_recent_execution_receipts" => vec![
            "Send the latest receipt summary to the Agent Panel or browser status surface.",
            "If an action failed, inspect the receipt error, screenshot artifact, runtime events, DOM snapshot, or element inspection summary before queueing another payload.",
        ],
        "has_requests_without_receipts" => vec![
            "Run invoke_managed_chrome_playwright_adapter after checking runner-gate readiness.",
            "Confirm the managed adapter script can write receipts under chrome-executions.",
        ],
        _ => vec![
            "Queue a managed Chrome payload, request the runner gate, prepare the adapter, then invoke it.",
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
