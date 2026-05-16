use super::agent_chrome_payload_tool::{
    AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA, AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME,
    AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA, AgentChromePayloadQueueRootMode,
};
use super::agent_chrome_playwright_adapter_tool::{
    AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA, AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME,
    AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA, AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME,
};
use super::agent_chrome_runner_gate_tool::{
    AGENT_CHROME_RUNNER_RECEIPT_FILE_NAME, AGENT_CHROME_RUNNER_RECEIPT_SCHEMA,
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
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME: &str =
    "invoke_managed_chrome_playwright_adapter";
pub const AGENT_CHROME_PLAYWRIGHT_RUN_REQUEST_SCHEMA: &str =
    "zed.agent_plugins.managed_chrome_playwright_run_request.v1";
pub const AGENT_CHROME_PLAYWRIGHT_INVOCATION_RESULT_SCHEMA: &str =
    "zed.agent_plugins.managed_chrome_playwright_invocation_result.v1";

const DEFAULT_TIMEOUT_MS: u64 = 60_000;
const MAX_TIMEOUT_MS: u64 = 180_000;
const MAX_OUTPUT_CHARS: usize = 8_000;

/// Invokes the managed Playwright adapter for safe non-input Chrome actions.
///
/// The default is a read-only plan. Set `execute_adapter` to run the prepared Node adapter after
/// explicit authorization. This tool refuses click, type, key, and scroll payloads until those
/// action families have their own permission, focus, QA, and receipt gates.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentChromePlaywrightInvokeToolInput {
    /// Prefer workspace-local queue and adapter roots under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentChromePayloadQueueRootMode,
    /// Actually run the prepared Node/Playwright adapter. When false, only returns readiness and planned request paths.
    pub execute_adapter: bool,
    /// Timeout for the adapter process.
    pub timeout_ms: u64,
    /// Include bounded stdout/stderr from the adapter process in the result.
    pub include_process_output: bool,
    /// Include the queued payload packet in the returned JSON.
    pub include_payload_packet: bool,
}

impl Default for AgentChromePlaywrightInvokeToolInput {
    fn default() -> Self {
        Self {
            root_mode: AgentChromePayloadQueueRootMode::Workspace,
            execute_adapter: false,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            include_process_output: false,
            include_payload_packet: false,
        }
    }
}

pub struct AgentChromePlaywrightInvokeTool {
    project: Entity<Project>,
}

impl AgentChromePlaywrightInvokeTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentChromePlaywrightInvokeTool {
    type Input = AgentChromePlaywrightInvokeToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input {
            Ok(input) if input.execute_adapter => "Invoke Chrome Playwright adapter".into(),
            _ => "Plan Chrome Playwright invocation".into(),
        }
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let timeout_ms = input.timeout_ms.clamp(1_000, MAX_TIMEOUT_MS);
            let project_root = cx.update(|cx| workspace_root_for_project(&self.project, cx));
            let invocation = ManagedChromePlaywrightInvocation::new(project_root, input.root_mode);
            invocation.validate_managed_paths()?;

            let readiness =
                invocation.readiness(input.include_payload_packet, input.execute_adapter);
            let status = readiness
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("blocked")
                .to_string();
            let action = readiness
                .get("action")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();

            if !input.execute_adapter || status != "ready_to_execute" {
                let output = invocation.output(&input, timeout_ms, readiness, None, None, false)?;
                event_stream.update_fields(acp::ToolCallUpdateFields::new().title(
                    if status == "ready_to_execute" {
                        "Chrome adapter invocation ready"
                    } else {
                        "Chrome adapter invocation blocked"
                    },
                ));
                return if status == "ready_to_execute" || !input.execute_adapter {
                    Ok(output)
                } else {
                    Err(output)
                };
            }

            let context = ToolPermissionContext::new(
                Self::NAME,
                vec![
                    action.clone(),
                    path_string(&invocation.runner_script_path),
                    path_string(&invocation.managed_profile_root),
                    path_string(&invocation.request_path),
                    path_string(&invocation.execution_receipt_path),
                    format!("{timeout_ms} ms timeout"),
                ],
            );
            let authorize = cx
                .update(|cx| {
                    event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
                })
                .map_err(|error| error.to_string())?;
            authorize.await.map_err(|error| error.to_string())?;

            let request = invocation.request_value(&readiness, timeout_ms)?;
            let request_json = serde_json::to_vec_pretty(&request)
                .map_err(|error| format!("Failed to serialize Chrome adapter request: {error}"))?;
            fs::create_dir_all(&invocation.execution_dir).map_err(|error| {
                format!(
                    "Failed to prepare Chrome adapter execution directory {}: {error}",
                    invocation.execution_dir.display()
                )
            })?;
            fs::create_dir_all(&invocation.artifacts_dir).map_err(|error| {
                format!(
                    "Failed to prepare Chrome adapter artifacts directory {}: {error}",
                    invocation.artifacts_dir.display()
                )
            })?;
            fs::write(&invocation.request_path, &request_json).map_err(|error| {
                format!(
                    "Failed to write Chrome adapter request {}: {error}",
                    invocation.request_path.display()
                )
            })?;

            let process_result =
                invocation.run_adapter(timeout_ms, input.include_process_output)?;
            let execution_receipt = invocation.read_execution_receipt();
            let output = invocation.output(
                &input,
                timeout_ms,
                readiness,
                Some(process_result),
                execution_receipt,
                true,
            )?;

            event_stream.update_fields(
                acp::ToolCallUpdateFields::new().title("Invoked Chrome Playwright adapter"),
            );

            Ok(output)
        })
    }
}

struct ManagedChromePlaywrightInvocation {
    root_mode: AgentChromePayloadQueueRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    plugin_root: PathBuf,
    latest_queue_path: PathBuf,
    latest_runner_receipt_path: PathBuf,
    playwright_root: PathBuf,
    playwright_adapter_root: PathBuf,
    playwright_adapter_manifest: PathBuf,
    runner_script_path: PathBuf,
    dx_extension_root: PathBuf,
    managed_profile_root: PathBuf,
    execution_dir: PathBuf,
    artifacts_dir: PathBuf,
    request_path: PathBuf,
    execution_receipt_path: PathBuf,
}

impl ManagedChromePlaywrightInvocation {
    fn new(project_root: Option<PathBuf>, root_mode: AgentChromePayloadQueueRootMode) -> Self {
        let use_workspace = matches!(root_mode, AgentChromePayloadQueueRootMode::Workspace)
            && project_root.is_some();
        let zed_plugin_root = data_dir().join("agent-plugins");
        let (allowed_root, plugin_root, playwright_root, managed_profile_root) = if use_workspace {
            let workspace_root = project_root.as_ref().expect("workspace root checked above");
            let tools_root = workspace_root.join("tools");
            (
                tools_root.clone(),
                tools_root.join("agent-plugins"),
                tools_root.join("playwright"),
                tools_root.join("browser-profiles").join("chrome"),
            )
        } else {
            (
                zed_plugin_root.clone(),
                zed_plugin_root.clone(),
                zed_plugin_root.join("playwright"),
                zed_plugin_root.join("browser-profiles").join("chrome"),
            )
        };
        let playwright_adapter_root =
            playwright_root.join(AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME);
        let playwright_adapter_manifest = playwright_adapter_root.join("adapter-manifest.json");
        let runner_script_path =
            playwright_adapter_root.join(AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME);
        let latest_queue_path = plugin_root
            .join("chrome-payloads")
            .join(AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME);
        let latest_runner_receipt_path = plugin_root
            .join("chrome-receipts")
            .join(AGENT_CHROME_RUNNER_RECEIPT_FILE_NAME);
        let dx_extension_root = plugin_root.join("dx-chrome-extension");
        let execution_dir = plugin_root.join("chrome-executions");
        let artifacts_dir = execution_dir.join("artifacts");
        let timestamp = current_epoch_millis();
        let request_path =
            execution_dir.join(format!("managed-chrome-run-request-{timestamp}.json"));
        let execution_receipt_path =
            execution_dir.join(format!("managed-chrome-execution-receipt-{timestamp}.json"));

        Self {
            root_mode,
            project_root,
            allowed_root,
            plugin_root,
            latest_queue_path,
            latest_runner_receipt_path,
            playwright_root,
            playwright_adapter_root,
            playwright_adapter_manifest,
            runner_script_path,
            dx_extension_root,
            managed_profile_root,
            execution_dir,
            artifacts_dir,
            request_path,
            execution_receipt_path,
        }
    }

    fn validate_managed_paths(&self) -> Result<(), String> {
        for path in [
            &self.latest_queue_path,
            &self.latest_runner_receipt_path,
            &self.playwright_adapter_root,
            &self.playwright_adapter_manifest,
            &self.runner_script_path,
            &self.dx_extension_root,
            &self.managed_profile_root,
            &self.execution_dir,
            &self.artifacts_dir,
            &self.request_path,
            &self.execution_receipt_path,
        ] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing Chrome adapter invocation path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }
        Ok(())
    }

    fn readiness(&self, include_payload_packet: bool, execute_requested: bool) -> Value {
        let queue = self.read_queue(include_payload_packet);
        let runner_receipt = self.read_runner_receipt();
        let node = find_executable(&["node", "node.exe"]);
        let browser = find_browser_executable();
        let playwright_package = self
            .playwright_root
            .join("node_modules")
            .join("playwright")
            .join("package.json");
        let dx_extension_manifest = self.dx_extension_root.join("manifest.json");
        let adapter_manifest_ready = self.adapter_manifest_ready();
        let action = queue
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        let checks = vec![
            readiness_check(
                "queue.latest_payload",
                "Latest managed Chrome payload",
                queue.get("ready").and_then(Value::as_bool).unwrap_or(false),
                Some(self.latest_queue_path.clone()),
                "queue_blocker",
                "Queue a valid managed Chrome payload first.",
            ),
            readiness_check(
                "queue.safe_adapter_action",
                "Safe adapter action",
                is_safe_adapter_action(&action),
                None,
                "action_blocker",
                "Only open_url, screenshot, inspect_element, set_viewport, and wait_for_selector are adapter-enabled.",
            ),
            readiness_check(
                "runner_gate.ready_receipt",
                "Ready runner-gate receipt",
                runner_receipt
                    .get("ready")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                Some(self.latest_runner_receipt_path.clone()),
                "receipt_blocker",
                "Request a managed Chrome runner receipt after queue and bootstrap readiness pass.",
            ),
            readiness_check(
                "host.node",
                "Node.js runtime",
                node.is_some(),
                node.clone(),
                "host_blocker",
                "Node.js is required to invoke the managed Playwright adapter.",
            ),
            readiness_check(
                "host.chrome_or_edge",
                "Chrome or Edge executable",
                browser.is_some(),
                browser.clone(),
                "host_blocker",
                "Managed Chrome control needs Chrome, Edge, or Chromium on this OS.",
            ),
            readiness_check(
                "asset.playwright_package",
                "Managed Playwright package",
                playwright_package.is_file(),
                Some(playwright_package.clone()),
                "provision_required",
                "Install Playwright into the managed tools root.",
            ),
            readiness_check(
                "asset.playwright_adapter_manifest",
                "Managed Playwright adapter manifest",
                adapter_manifest_ready,
                Some(self.playwright_adapter_manifest.clone()),
                "provision_required",
                "Prepare the managed Playwright adapter artifact.",
            ),
            readiness_check(
                "asset.playwright_adapter_runner",
                "Managed Playwright adapter runner",
                self.runner_script_path.is_file(),
                Some(self.runner_script_path.clone()),
                "provision_required",
                "Prepare the managed Playwright runner script.",
            ),
            readiness_check(
                "asset.dx_chrome_extension",
                "DX Chrome extension manifest",
                dx_extension_manifest.is_file(),
                Some(dx_extension_manifest.clone()),
                "provision_required",
                "Download or unpack the DX Chrome extension into the managed plugin root.",
            ),
            readiness_check(
                "profile.managed_chrome",
                "Managed Chrome profile root",
                self.managed_profile_root.is_dir(),
                Some(self.managed_profile_root.clone()),
                "provision_required",
                "Create the managed Chrome profile root and never use a real browser profile.",
            ),
        ];

        let blockers = checks
            .iter()
            .filter(|check| !check.get("ready").and_then(Value::as_bool).unwrap_or(false))
            .cloned()
            .collect::<Vec<_>>();
        let status = if blockers.is_empty() {
            "ready_to_execute"
        } else {
            "blocked"
        };

        serde_json::json!({
            "status": status,
            "execute_requested": execute_requested,
            "action": action,
            "root_mode": self.root_mode_label(),
            "queue": queue,
            "runner_receipt": runner_receipt,
            "checks": checks,
            "blockers": blockers,
            "host": {
                "node": node.as_ref().map(path_string),
                "chrome_or_edge": browser.as_ref().map(path_string),
            },
        })
    }

    fn request_value(&self, readiness: &Value, timeout_ms: u64) -> Result<Value, String> {
        let execution_queue = self.read_queue(true);
        let queue_item = execution_queue
            .get("queue_item")
            .cloned()
            .ok_or_else(|| "Missing queue item for Chrome adapter request".to_string())?;
        let payload_packet = queue_item
            .get("payload_packet")
            .cloned()
            .ok_or_else(|| "Missing payload packet for Chrome adapter request".to_string())?;
        let browser_path = readiness
            .pointer("/host/chrome_or_edge")
            .and_then(Value::as_str)
            .map(str::to_string);

        Ok(serde_json::json!({
            "schema": AGENT_CHROME_PLAYWRIGHT_RUN_REQUEST_SCHEMA,
            "generated_at_ms": current_epoch_millis(),
            "source_tool": AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME,
            "queue_item": queue_item,
            "payload_packet": payload_packet,
            "runner_receipt": readiness.get("runner_receipt").cloned().unwrap_or(Value::Null),
            "roots": {
                "allowed_root": path_string(&self.allowed_root),
                "managed_root": path_string(&self.allowed_root),
                "plugin_root": path_string(&self.plugin_root),
                "playwright_root": path_string(&self.playwright_root),
                "adapter_root": path_string(&self.playwright_adapter_root),
                "dx_extension_root": path_string(&self.dx_extension_root),
                "managed_chrome_profile_root": path_string(&self.managed_profile_root),
                "artifacts_root": path_string(&self.artifacts_dir),
            },
            "browser": {
                "executable_path": browser_path,
            },
            "execution": {
                "timeout_ms": timeout_ms,
                "request_path": path_string(&self.request_path),
                "receipt_path": path_string(&self.execution_receipt_path),
            },
            "safety": {
                "managed_profile_only": true,
                "input_actions_blocked_by_tool": true,
                "permission_prompted_before_execution": true,
                "real_browser_profiles_touched": false,
            }
        }))
    }

    fn output(
        &self,
        input: &AgentChromePlaywrightInvokeToolInput,
        timeout_ms: u64,
        readiness: Value,
        process_result: Option<Value>,
        execution_receipt: Option<Value>,
        attempted_execution: bool,
    ) -> Result<String, String> {
        let result_status = if attempted_execution {
            execution_receipt
                .as_ref()
                .and_then(|receipt| receipt.get("outcome").and_then(Value::as_str))
                .unwrap_or("execution_attempted")
        } else if readiness
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| status == "ready_to_execute")
        {
            "ready_to_execute"
        } else {
            "blocked"
        };

        let output = serde_json::json!({
            "schema": AGENT_CHROME_PLAYWRIGHT_INVOCATION_RESULT_SCHEMA,
            "result": {
                "generated_at_ms": current_epoch_millis(),
                "status": result_status,
                "root_mode": self.root_mode_label(),
                "execute_adapter": input.execute_adapter,
                "attempted_execution": attempted_execution,
                "timeout_ms": timeout_ms,
                "request_path": path_string(&self.request_path),
                "execution_receipt_path": path_string(&self.execution_receipt_path),
            },
            "readiness": readiness,
            "process": process_result,
            "execution_receipt": execution_receipt,
            "next_actions": if attempted_execution {
                vec![
                    "Inspect the execution receipt, screenshot artifact path, or element inspection summary.",
                    "Keep click, type, key, and scroll on their separate input gates."
                ]
            } else {
                vec![
                    "Resolve any readiness blockers.",
                    "Call invoke_managed_chrome_playwright_adapter with execute_adapter=true only for non-input actions."
                ]
            },
            "safety": {
                "permission_required_for_execution": true,
                "safe_actions_only": ["open_url", "screenshot", "inspect_element", "set_viewport", "wait_for_selector"],
                "input_actions_blocked": ["click", "type_text", "press_key", "scroll"],
                "managed_profile_only": true,
                "real_browser_profiles_touched": false,
            }
        });

        serde_json::to_string_pretty(&output).map_err(|error| {
            format!("Failed to serialize Chrome adapter invocation output: {error}")
        })
    }

    fn run_adapter(&self, timeout_ms: u64, include_output: bool) -> Result<Value, String> {
        let node = find_executable(&["node", "node.exe"])
            .ok_or_else(|| "Node.js was not found on PATH".to_string())?;
        let mut command = Command::new(&node);
        command
            .arg(&self.runner_script_path)
            .arg("--request")
            .arg(&self.request_path)
            .arg("--receipt")
            .arg(&self.execution_receipt_path)
            .current_dir(&self.playwright_adapter_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|error| {
            format!(
                "Failed to spawn managed Chrome Playwright adapter {}: {error}",
                self.runner_script_path.display()
            )
        })?;
        let start = Instant::now();
        loop {
            if child
                .try_wait()
                .map_err(|error| format!("Failed to poll Chrome adapter process: {error}"))?
                .is_some()
            {
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("Failed to collect Chrome adapter output: {error}"))?;
                return Ok(process_output_value(false, include_output, output));
            }

            if start.elapsed() >= Duration::from_millis(timeout_ms) {
                let _ = child.kill();
                let output = child.wait_with_output().map_err(|error| {
                    format!("Failed to collect timed-out Chrome adapter output: {error}")
                })?;
                return Ok(process_output_value(true, include_output, output));
            }

            thread::sleep(Duration::from_millis(100));
        }
    }

    fn read_queue(&self, include_payload_packet: bool) -> Value {
        let bytes = match fs::read(&self.latest_queue_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return serde_json::json!({
                    "ready": false,
                    "state": "missing",
                    "path": path_string(&self.latest_queue_path),
                });
            }
            Err(error) => {
                return serde_json::json!({
                    "ready": false,
                    "state": "read_error",
                    "path": path_string(&self.latest_queue_path),
                    "details": error.to_string(),
                });
            }
        };
        let value = match serde_json::from_slice::<Value>(&bytes) {
            Ok(value) => value,
            Err(error) => {
                return serde_json::json!({
                    "ready": false,
                    "state": "parse_error",
                    "path": path_string(&self.latest_queue_path),
                    "bytes": bytes.len(),
                    "details": error.to_string(),
                });
            }
        };
        let queue_schema_ok = value
            .get("schema")
            .and_then(Value::as_str)
            .is_some_and(|schema| schema == AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA);
        let payload_schema_ok = value
            .pointer("/payload_packet/schema")
            .and_then(Value::as_str)
            .is_some_and(|schema| schema == AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA);
        let action = value
            .pointer("/payload_packet/payload/action")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let ready = queue_schema_ok && payload_schema_ok && is_supported_action(action);

        let mut summary = serde_json::json!({
            "ready": ready,
            "state": if ready { "ready" } else { "invalid_schema" },
            "path": path_string(&self.latest_queue_path),
            "bytes": bytes.len(),
            "queue_schema_ok": queue_schema_ok,
            "payload_schema_ok": payload_schema_ok,
            "action": action,
            "queued_at_ms": value.get("queued_at_ms").cloned(),
            "queue_item": value,
        });

        if !include_payload_packet {
            if let Some(queue_item) = summary.get_mut("queue_item") {
                if let Some(object) = queue_item.as_object_mut() {
                    object.remove("payload_packet");
                }
            }
        }

        summary
    }

    fn read_runner_receipt(&self) -> Value {
        let bytes = match fs::read(&self.latest_runner_receipt_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return serde_json::json!({
                    "ready": false,
                    "state": "missing",
                    "path": path_string(&self.latest_runner_receipt_path),
                });
            }
            Err(error) => {
                return serde_json::json!({
                    "ready": false,
                    "state": "read_error",
                    "path": path_string(&self.latest_runner_receipt_path),
                    "details": error.to_string(),
                });
            }
        };
        let value = match serde_json::from_slice::<Value>(&bytes) {
            Ok(value) => value,
            Err(error) => {
                return serde_json::json!({
                    "ready": false,
                    "state": "parse_error",
                    "path": path_string(&self.latest_runner_receipt_path),
                    "bytes": bytes.len(),
                    "details": error.to_string(),
                });
            }
        };
        let schema_ok = value
            .get("schema")
            .and_then(Value::as_str)
            .is_some_and(|schema| schema == AGENT_CHROME_RUNNER_RECEIPT_SCHEMA);
        let outcome = value
            .pointer("/result/outcome")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let ready = schema_ok && outcome == "ready_runner_adapter_pending";

        serde_json::json!({
            "ready": ready,
            "state": if ready { "ready" } else { "not_ready" },
            "path": path_string(&self.latest_runner_receipt_path),
            "bytes": bytes.len(),
            "schema_ok": schema_ok,
            "outcome": outcome,
        })
    }

    fn read_execution_receipt(&self) -> Option<Value> {
        let bytes = fs::read(&self.execution_receipt_path).ok()?;
        let mut value = serde_json::from_slice::<Value>(&bytes).ok()?;
        let schema_ok = value
            .get("schema")
            .and_then(Value::as_str)
            .is_some_and(|schema| schema == AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA);
        if let Some(object) = value.as_object_mut() {
            object.insert("schema_ok".to_string(), Value::Bool(schema_ok));
            object.insert(
                "path".to_string(),
                Value::String(path_string(&self.execution_receipt_path)),
            );
            object.insert(
                "bytes".to_string(),
                Value::Number(serde_json::Number::from(bytes.len() as u64)),
            );
        }
        Some(value)
    }

    fn adapter_manifest_ready(&self) -> bool {
        let bytes = match fs::read(&self.playwright_adapter_manifest) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        serde_json::from_slice::<Value>(&bytes)
            .ok()
            .and_then(|value| {
                value
                    .get("schema")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .is_some_and(|schema| schema == AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA)
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
}

fn readiness_check(
    id: &str,
    label: &str,
    ready: bool,
    path: Option<PathBuf>,
    missing_kind: &str,
    details: &str,
) -> Value {
    serde_json::json!({
        "id": id,
        "label": label,
        "state": if ready { "ready" } else { missing_kind },
        "ready": ready,
        "path": path.as_ref().map(path_string),
        "details": details,
    })
}

fn process_output_value(
    timed_out: bool,
    include_output: bool,
    output: std::process::Output,
) -> Value {
    let mut value = serde_json::json!({
        "timed_out": timed_out,
        "exit_code": output.status.code(),
        "success": output.status.success() && !timed_out,
    });
    if include_output {
        value["stdout"] = Value::String(truncate_output(&String::from_utf8_lossy(&output.stdout)));
        value["stderr"] = Value::String(truncate_output(&String::from_utf8_lossy(&output.stderr)));
    }
    value
}

fn truncate_output(output: &str) -> String {
    if output.chars().count() <= MAX_OUTPUT_CHARS {
        return output.to_string();
    }
    let tail = output
        .chars()
        .rev()
        .take(MAX_OUTPUT_CHARS)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("[truncated to last {MAX_OUTPUT_CHARS} chars]\n{tail}")
}

fn is_safe_adapter_action(action: &str) -> bool {
    matches!(
        action,
        "open_url" | "screenshot" | "inspect_element" | "set_viewport" | "wait_for_selector"
    )
}

fn is_supported_action(action: &str) -> bool {
    matches!(
        action,
        "open_url"
            | "click"
            | "type_text"
            | "press_key"
            | "scroll"
            | "screenshot"
            | "inspect_element"
            | "wait_for_selector"
            | "set_viewport"
    )
}

fn find_browser_executable() -> Option<PathBuf> {
    find_executable(&[
        "chrome",
        "chrome.exe",
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "msedge",
        "msedge.exe",
        "microsoft-edge",
    ])
    .or_else(|| existing_file(common_browser_candidates()))
}

fn common_browser_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if cfg!(target_os = "windows") {
        for env_name in ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"] {
            if let Some(root) = env_path(env_name) {
                candidates.push(
                    root.join("Google")
                        .join("Chrome")
                        .join("Application")
                        .join("chrome.exe"),
                );
                candidates.push(
                    root.join("Microsoft")
                        .join("Edge")
                        .join("Application")
                        .join("msedge.exe"),
                );
            }
        }
    } else if cfg!(target_os = "macos") {
        candidates.push(
            PathBuf::from("/Applications")
                .join("Google Chrome.app")
                .join("Contents")
                .join("MacOS")
                .join("Google Chrome"),
        );
        candidates.push(
            PathBuf::from("/Applications")
                .join("Microsoft Edge.app")
                .join("Contents")
                .join("MacOS")
                .join("Microsoft Edge"),
        );
    }

    candidates
}

fn find_executable(names: &[&str]) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn existing_file(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
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
