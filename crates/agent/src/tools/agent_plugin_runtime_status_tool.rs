use super::{
    agent_browser_payload_queue_inspect_tool::{
        AGENT_BROWSER_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
        AGENT_BROWSER_PAYLOAD_QUEUE_INSPECTION_SCHEMA,
    },
    agent_browser_payload_tool::{
        AGENT_BROWSER_PAYLOAD_QUEUE_FILE_NAME, AGENT_BROWSER_PAYLOAD_QUEUE_ITEM_SCHEMA,
        AGENT_BROWSER_PAYLOAD_QUEUE_TOOL_NAME, AGENT_BROWSER_PAYLOAD_STAGE_TOOL_NAME,
        AGENT_BROWSER_PAYLOAD_TOOL_NAME,
    },
    agent_chrome_payload_tool::{
        AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME, AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
        AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA, AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME,
        AGENT_CHROME_PAYLOAD_TOOL_NAME,
    },
    agent_chrome_playwright_adapter_tool::{
        AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA, AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME,
        AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME, AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME,
    },
    agent_chrome_playwright_execution_inspect_tool::{
        AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_RESULT_SCHEMA,
        AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
    },
    agent_chrome_playwright_invoke_tool::{
        AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME, AGENT_CHROME_PLAYWRIGHT_RUN_REQUEST_SCHEMA,
    },
    agent_chrome_runner_gate_tool::{
        AGENT_CHROME_RUNNER_GATE_TOOL_NAME, AGENT_CHROME_RUNNER_RECEIPT_FILE_NAME,
        AGENT_CHROME_RUNNER_RECEIPT_SCHEMA,
    },
    agent_pc_use_payload_queue_inspect_tool::{
        AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME, AGENT_PC_USE_PAYLOAD_QUEUE_INSPECTION_SCHEMA,
    },
    agent_pc_use_payload_tool::{
        AGENT_PC_USE_PAYLOAD_QUEUE_FILE_NAME, AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA,
        AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME, AGENT_PC_USE_PAYLOAD_STAGE_TOOL_NAME,
        AGENT_PC_USE_PAYLOAD_TOOL_NAME,
    },
    agent_pc_use_runner_gate_tool::{
        AGENT_PC_USE_RUNNER_GATE_TOOL_NAME, AGENT_PC_USE_RUNNER_RECEIPT_FILE_NAME,
        AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA,
    },
    agent_pc_use_runner_receipt_inspect_tool::{
        AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME,
        AGENT_PC_USE_RUNNER_RECEIPT_INSPECTION_SCHEMA,
    },
    agent_pc_use_target_manifest_tool::AGENT_PC_USE_TARGET_MANIFEST_TOOL_NAME,
    agent_pc_use_target_snapshot_tool::AGENT_PC_USE_TARGET_SNAPSHOT_TOOL_NAME,
    agent_pc_use_ui_snapshot_contract_tool::AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_TOOL_NAME,
    agent_pc_use_ui_snapshot_tool::AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME,
    agent_plugin_bootstrap_tool::AgentPluginBootstrapTool,
    agent_plugin_catalog_tool::AgentPluginCatalogTool,
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
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME: &str = "inspect_agent_plugin_runtime_status";
pub const AGENT_PLUGIN_RUNTIME_STATUS_SCHEMA: &str = "zed.agent_plugins.runtime_status.v1";

const MAX_HANDOFF_PREVIEW_BYTES: u64 = 1_048_576;

/// Summarizes Browser, managed Chrome, and PC-use plugin readiness without executing anything.
///
/// This read-only tool is the fastest way for an Agent Panel to decide which plugin tools are
/// available, which managed handoff files already exist, and which permissioned runner gates are
/// still required. It never writes files, launches browsers, runs Node, takes screenshots, or
/// dispatches input.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPluginRuntimeStatusToolInput {
    /// Prefer workspace-local roots under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentPluginRuntimeStatusRootMode,
    /// Include small summaries of the latest managed payload and receipt files.
    pub include_latest_handoffs: bool,
    /// Include host executable probes for Chrome/Playwright readiness.
    pub include_host_checks: bool,
    /// Include suggested next actions for blocked or partially provisioned runtimes.
    pub include_next_actions: bool,
    /// Include ordered safe workflow recipes for Browser, managed Chrome, and PC-use.
    pub include_workflows: bool,
}

impl Default for AgentPluginRuntimeStatusToolInput {
    fn default() -> Self {
        Self {
            root_mode: AgentPluginRuntimeStatusRootMode::Workspace,
            include_latest_handoffs: true,
            include_host_checks: true,
            include_next_actions: true,
            include_workflows: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentPluginRuntimeStatusRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct AgentPluginRuntimeStatusTool {
    project: Entity<Project>,
}

impl AgentPluginRuntimeStatusTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentPluginRuntimeStatusTool {
    type Input = AgentPluginRuntimeStatusToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Inspect agent plugin runtime status".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let workspace_roots = cx.update(|cx| workspace_roots_for_project(&self.project, cx));
            let roots = AgentPluginRuntimeRoots::new(workspace_roots, input.root_mode);
            let result = inspect_runtime_status(&roots, &input);
            let status = result
                .pointer("/result/status")
                .and_then(Value::as_str)
                .unwrap_or("inspected");
            let output = serde_json::to_string_pretty(&result).map_err(|error| {
                format!("Failed to serialize agent plugin runtime status: {error}")
            })?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(match status {
                "ready_for_read_only_discovery" => "Agent plugin runtime is discoverable",
                "workspace_roots_missing" => "Agent plugin runtime is using Zed data roots",
                "blocked_unmanaged_paths" => "Agent plugin runtime paths need review",
                _ => "Inspected agent plugin runtime status",
            }));

            Ok(output)
        })
    }
}

struct AgentPluginRuntimeRoots {
    root_mode: AgentPluginRuntimeStatusRootMode,
    workspace_roots: Vec<PathBuf>,
    active_project_root: Option<PathBuf>,
    managed_base_root: PathBuf,
    plugin_root: PathBuf,
    playwright_root: PathBuf,
    dx_extension_root: PathBuf,
    managed_chrome_profile_root: PathBuf,
    browser_queue_dir: PathBuf,
    browser_latest_payload: PathBuf,
    chrome_queue_dir: PathBuf,
    chrome_latest_payload: PathBuf,
    chrome_receipt_dir: PathBuf,
    chrome_latest_runner_receipt: PathBuf,
    chrome_execution_dir: PathBuf,
    chrome_adapter_root: PathBuf,
    chrome_adapter_manifest: PathBuf,
    chrome_runner_script: PathBuf,
    pc_use_root: PathBuf,
    pc_use_payload_dir: PathBuf,
    pc_use_latest_payload: PathBuf,
    pc_use_receipt_dir: PathBuf,
    pc_use_latest_receipt: PathBuf,
}

impl AgentPluginRuntimeRoots {
    fn new(workspace_roots: Vec<PathBuf>, root_mode: AgentPluginRuntimeStatusRootMode) -> Self {
        let active_project_root = workspace_roots.first().cloned();
        let use_workspace = matches!(root_mode, AgentPluginRuntimeStatusRootMode::Workspace)
            && active_project_root.is_some();
        let zed_plugin_root = data_dir().join("agent-plugins");
        let (managed_base_root, plugin_root, playwright_root, managed_chrome_profile_root) =
            if use_workspace {
                let tools_root = active_project_root
                    .as_ref()
                    .expect("workspace root checked above")
                    .join("tools");
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

        let dx_extension_root = plugin_root.join("dx-chrome-extension");
        let browser_queue_dir = plugin_root.join("browser-payloads");
        let browser_latest_payload = browser_queue_dir.join(AGENT_BROWSER_PAYLOAD_QUEUE_FILE_NAME);
        let chrome_queue_dir = plugin_root.join("chrome-payloads");
        let chrome_latest_payload = chrome_queue_dir.join(AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME);
        let chrome_receipt_dir = plugin_root.join("chrome-receipts");
        let chrome_latest_runner_receipt =
            chrome_receipt_dir.join(AGENT_CHROME_RUNNER_RECEIPT_FILE_NAME);
        let chrome_execution_dir = plugin_root.join("chrome-executions");
        let chrome_adapter_root = playwright_root.join(AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME);
        let chrome_adapter_manifest = chrome_adapter_root.join("adapter-manifest.json");
        let chrome_runner_script =
            chrome_adapter_root.join(AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME);
        let pc_use_root = plugin_root.join("pc-use");
        let pc_use_payload_dir = pc_use_root.join("payloads");
        let pc_use_latest_payload = pc_use_payload_dir.join(AGENT_PC_USE_PAYLOAD_QUEUE_FILE_NAME);
        let pc_use_receipt_dir = pc_use_root.join("receipts");
        let pc_use_latest_receipt = pc_use_receipt_dir.join(AGENT_PC_USE_RUNNER_RECEIPT_FILE_NAME);

        Self {
            root_mode,
            workspace_roots,
            active_project_root,
            managed_base_root,
            plugin_root,
            playwright_root,
            dx_extension_root,
            managed_chrome_profile_root,
            browser_queue_dir,
            browser_latest_payload,
            chrome_queue_dir,
            chrome_latest_payload,
            chrome_receipt_dir,
            chrome_latest_runner_receipt,
            chrome_execution_dir,
            chrome_adapter_root,
            chrome_adapter_manifest,
            chrome_runner_script,
            pc_use_root,
            pc_use_payload_dir,
            pc_use_latest_payload,
            pc_use_receipt_dir,
            pc_use_latest_receipt,
        }
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            AgentPluginRuntimeStatusRootMode::Workspace if self.active_project_root.is_some() => {
                "workspace"
            }
            AgentPluginRuntimeStatusRootMode::Workspace => "zed_data_fallback",
            AgentPluginRuntimeStatusRootMode::ZedData => "zed_data",
        }
    }

    fn managed_path_valid(&self, path: &Path) -> bool {
        path.starts_with(&self.managed_base_root)
    }

    fn all_managed_paths_valid(&self) -> bool {
        [
            &self.plugin_root,
            &self.playwright_root,
            &self.dx_extension_root,
            &self.managed_chrome_profile_root,
            &self.browser_queue_dir,
            &self.browser_latest_payload,
            &self.chrome_queue_dir,
            &self.chrome_latest_payload,
            &self.chrome_receipt_dir,
            &self.chrome_latest_runner_receipt,
            &self.chrome_execution_dir,
            &self.chrome_adapter_root,
            &self.chrome_adapter_manifest,
            &self.chrome_runner_script,
            &self.pc_use_root,
            &self.pc_use_payload_dir,
            &self.pc_use_latest_payload,
            &self.pc_use_receipt_dir,
            &self.pc_use_latest_receipt,
        ]
        .into_iter()
        .all(|path| self.managed_path_valid(path))
    }
}

fn inspect_runtime_status(
    roots: &AgentPluginRuntimeRoots,
    input: &AgentPluginRuntimeStatusToolInput,
) -> Value {
    let managed_paths_valid = roots.all_managed_paths_valid();
    let status = if !managed_paths_valid {
        "blocked_unmanaged_paths"
    } else if roots.active_project_root.is_none()
        && matches!(input.root_mode, AgentPluginRuntimeStatusRootMode::Workspace)
    {
        "workspace_roots_missing"
    } else {
        "ready_for_read_only_discovery"
    };

    let host_checks = input.include_host_checks.then(host_checks);
    let browser = browser_status(roots, input.include_latest_handoffs);
    let chrome = chrome_status(roots, input.include_latest_handoffs, host_checks.as_ref());
    let pc_use = pc_use_status(roots, input.include_latest_handoffs);

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_STATUS_SCHEMA,
        "result": {
            "generated_at_ms": current_epoch_millis(),
            "status": status,
            "root_mode": roots.root_mode_label(),
            "managed_paths_valid": managed_paths_valid,
            "visible_worktree_count": roots.workspace_roots.len(),
            "active_project_root": roots.active_project_root.as_ref().map(path_string),
        },
        "tools": {
            "catalog": AgentPluginCatalogTool::NAME,
            "runtime_status": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
            "prepare_runtime": AgentPluginBootstrapTool::NAME,
        },
        "roots": roots_value(roots),
        "plugins": {
            "browser": browser,
            "chrome": chrome,
            "pc_use": pc_use,
        },
        "host": host_checks,
        "workflow_recipes": input.include_workflows.then(workflow_recipes),
        "next_actions": input.include_next_actions.then(|| next_actions(status, roots)),
        "safety": {
            "read_only": true,
            "writes_files": false,
            "runs_node": false,
            "launches_browser": false,
            "takes_screenshot": false,
            "dispatches_mouse": false,
            "dispatches_keyboard": false,
            "touches_real_browser_profiles": false,
            "permissioned_tools_still_require_user_visible_gates": true,
        }
    })
}

fn browser_status(roots: &AgentPluginRuntimeRoots, include_latest_handoff: bool) -> Value {
    serde_json::json!({
        "id": "zed.browser",
        "status": "available",
        "scope": "in_app_web_preview",
        "external_process_required": false,
        "tools": {
            "compose_payload": AGENT_BROWSER_PAYLOAD_TOOL_NAME,
            "stage_payload": AGENT_BROWSER_PAYLOAD_STAGE_TOOL_NAME,
            "queue_payload": AGENT_BROWSER_PAYLOAD_QUEUE_TOOL_NAME,
            "inspect_payload_queue": AGENT_BROWSER_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
        },
        "schemas": {
            "payload_queue_item": AGENT_BROWSER_PAYLOAD_QUEUE_ITEM_SCHEMA,
            "payload_queue_inspection": AGENT_BROWSER_PAYLOAD_QUEUE_INSPECTION_SCHEMA,
        },
        "managed_paths": {
            "queue_dir": dir_probe(&roots.browser_queue_dir),
            "latest_payload": file_probe(
                &roots.browser_latest_payload,
                Some(AGENT_BROWSER_PAYLOAD_QUEUE_ITEM_SCHEMA),
                Some("/payload_packet/schema"),
                include_latest_handoff,
            ),
        },
        "requirements_before_input": [
            "active WebPreview session",
            "interactive browser action unlock",
            "fresh preflight and native trace receipt",
            "matching executor receipt after dispatch"
        ],
    })
}

fn chrome_status(
    roots: &AgentPluginRuntimeRoots,
    include_latest_handoff: bool,
    host_checks: Option<&Value>,
) -> Value {
    let node_ready = host_checks
        .and_then(|checks| checks.pointer("/node/available"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let browser_ready = host_checks
        .and_then(|checks| checks.pointer("/chrome_or_edge/available"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let playwright_package = roots
        .playwright_root
        .join("node_modules")
        .join("playwright")
        .join("package.json");
    let dx_extension_manifest = roots.dx_extension_root.join("manifest.json");
    let adapter_ready =
        roots.chrome_adapter_manifest.is_file() && roots.chrome_runner_script.is_file();
    let provisioned = node_ready
        && browser_ready
        && playwright_package.is_file()
        && dx_extension_manifest.is_file()
        && adapter_ready;

    serde_json::json!({
        "id": "zed.chrome",
        "status": if provisioned { "ready_for_permissioned_adapter" } else { "available_needs_managed_assets" },
        "scope": "managed_external_chrome",
        "external_process_required": true,
        "tools": {
            "compose_payload": AGENT_CHROME_PAYLOAD_TOOL_NAME,
            "queue_payload": AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME,
            "inspect_payload_queue": AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
            "request_payload_run": AGENT_CHROME_RUNNER_GATE_TOOL_NAME,
            "prepare_playwright_adapter": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
            "invoke_playwright_adapter": AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME,
            "inspect_playwright_executions": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
        },
        "schemas": {
            "payload_queue_item": AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA,
            "runner_receipt": AGENT_CHROME_RUNNER_RECEIPT_SCHEMA,
            "playwright_adapter_manifest": AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA,
            "playwright_run_request": AGENT_CHROME_PLAYWRIGHT_RUN_REQUEST_SCHEMA,
            "playwright_execution_inspection": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_RESULT_SCHEMA,
        },
        "managed_paths": {
            "queue_dir": dir_probe(&roots.chrome_queue_dir),
            "latest_payload": file_probe(
                &roots.chrome_latest_payload,
                Some(AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA),
                Some("/payload_packet/schema"),
                include_latest_handoff,
            ),
            "receipt_dir": dir_probe(&roots.chrome_receipt_dir),
            "latest_runner_receipt": file_probe(
                &roots.chrome_latest_runner_receipt,
                Some(AGENT_CHROME_RUNNER_RECEIPT_SCHEMA),
                None,
                include_latest_handoff,
            ),
            "execution_dir": dir_probe(&roots.chrome_execution_dir),
            "playwright_root": dir_probe(&roots.playwright_root),
            "playwright_package": file_probe(&playwright_package, None, None, false),
            "adapter_root": dir_probe(&roots.chrome_adapter_root),
            "adapter_manifest": file_probe(
                &roots.chrome_adapter_manifest,
                Some(AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA),
                None,
                include_latest_handoff,
            ),
            "runner_script": file_probe(&roots.chrome_runner_script, None, None, false),
            "dx_extension_root": dir_probe(&roots.dx_extension_root),
            "dx_extension_manifest": file_probe(&dx_extension_manifest, None, None, false),
            "managed_profile_root": dir_probe(&roots.managed_chrome_profile_root),
        },
        "requirements_before_execution": [
            "managed Chrome or Edge executable is available",
            "Playwright package is installed under the managed tools root",
            "DX Chrome extension is unpacked under the managed plugin root",
            "permissioned invoke tool is called with execute_adapter=true",
            "adapter receipt is inspected after every execution"
        ],
    })
}

fn pc_use_status(roots: &AgentPluginRuntimeRoots, include_latest_handoff: bool) -> Value {
    serde_json::json!({
        "id": "zed.pc_use",
        "status": "read_only_snapshot_available_future_input_gated",
        "scope": "zed_window_only",
        "external_process_required": false,
        "tools": {
            "inspect_window_context": "inspect_zed_window_context",
            "inspect_targets": AGENT_PC_USE_TARGET_MANIFEST_TOOL_NAME,
            "inspect_target_snapshot": AGENT_PC_USE_TARGET_SNAPSHOT_TOOL_NAME,
            "inspect_ui_snapshot_contract": AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_TOOL_NAME,
            "inspect_ui_snapshot": AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME,
            "compose_payload": AGENT_PC_USE_PAYLOAD_TOOL_NAME,
            "stage_payload": AGENT_PC_USE_PAYLOAD_STAGE_TOOL_NAME,
            "queue_payload": AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME,
            "inspect_payload_queue": AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
            "request_payload_run": AGENT_PC_USE_RUNNER_GATE_TOOL_NAME,
            "inspect_runner_receipts": AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME,
        },
        "schemas": {
            "payload_queue_item": AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA,
            "payload_queue_inspection": AGENT_PC_USE_PAYLOAD_QUEUE_INSPECTION_SCHEMA,
            "runner_receipt": AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA,
            "runner_receipt_inspection": AGENT_PC_USE_RUNNER_RECEIPT_INSPECTION_SCHEMA,
        },
        "managed_paths": {
            "pc_use_root": dir_probe(&roots.pc_use_root),
            "payload_dir": dir_probe(&roots.pc_use_payload_dir),
            "latest_payload": file_probe(
                &roots.pc_use_latest_payload,
                Some(AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA),
                Some("/payload_packet/schema"),
                include_latest_handoff,
            ),
            "receipt_dir": dir_probe(&roots.pc_use_receipt_dir),
            "latest_runner_receipt": file_probe(
                &roots.pc_use_latest_receipt,
                Some(AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA),
                None,
                include_latest_handoff,
            ),
        },
        "requirements_before_future_input": [
            "fresh inspect_zed_pc_use_ui_snapshot result",
            "input-ready target id from the live snapshot",
            "matching target_snapshot_id in any focus/click/type payload",
            "user-visible permission gate",
            "runner receipt after any future execution"
        ],
    })
}

fn host_checks() -> Value {
    let node = find_executable(&["node.exe", "node"]);
    let npm = find_executable(&["npm.cmd", "npm.exe", "npm"]);
    let chrome_or_edge = find_executable(&[
        "chrome.exe",
        "msedge.exe",
        "chromium.exe",
        "google-chrome.exe",
        "chrome",
        "msedge",
        "chromium",
        "google-chrome",
    ]);

    serde_json::json!({
        "node": executable_probe(node),
        "npm": executable_probe(npm),
        "chrome_or_edge": executable_probe(chrome_or_edge),
    })
}

fn roots_value(roots: &AgentPluginRuntimeRoots) -> Value {
    serde_json::json!({
        "workspace_roots": roots.workspace_roots.iter().map(path_string).collect::<Vec<_>>(),
        "managed_base_root": path_string(&roots.managed_base_root),
        "plugin_root": path_string(&roots.plugin_root),
        "playwright_root": path_string(&roots.playwright_root),
        "dx_chrome_extension_root": path_string(&roots.dx_extension_root),
        "managed_chrome_profile_root": path_string(&roots.managed_chrome_profile_root),
        "browser_queue_dir": path_string(&roots.browser_queue_dir),
        "chrome_queue_dir": path_string(&roots.chrome_queue_dir),
        "chrome_receipt_dir": path_string(&roots.chrome_receipt_dir),
        "chrome_execution_dir": path_string(&roots.chrome_execution_dir),
        "pc_use_root": path_string(&roots.pc_use_root),
    })
}

fn next_actions(status: &str, roots: &AgentPluginRuntimeRoots) -> Vec<&'static str> {
    if status == "blocked_unmanaged_paths" {
        return vec![
            "Inspect managed root construction before writing or executing any plugin handoffs.",
            "Run list_agent_plugins and inspect_agent_plugin_runtime_status again after correcting paths.",
        ];
    }

    let mut actions = Vec::new();
    if roots.active_project_root.is_none() {
        actions.push(
            "Open a workspace to use workspace-local plugin roots, or select root_mode=zed_data.",
        );
    }
    if !roots.browser_latest_payload.is_file() {
        actions.push(
            "Queue a browser payload only when WebPreview import and permission gates are intended.",
        );
    }
    if !roots.chrome_adapter_manifest.is_file() || !roots.chrome_runner_script.is_file() {
        actions.push(
            "Prepare the managed Chrome Playwright adapter before external Chrome execution.",
        );
    }
    if !roots.pc_use_latest_payload.is_file() {
        actions.push("Use inspect_zed_pc_use_ui_snapshot before composing any future Zed-window input payload.");
    }
    if actions.is_empty() {
        actions.push("Use the plugin-specific inspect tools before any permissioned execution.");
    }
    actions
}

fn workflow_recipes() -> Value {
    serde_json::json!({
        "browser_webpreview_payload": {
            "goal": "Move a validated Agent Browser action into the in-app WebPreview without dispatching input from Agent tools.",
            "safe_for": ["click", "type_text", "press_key", "scroll"],
            "ordered_steps": [
                {
                    "step": "compose",
                    "tool": AGENT_BROWSER_PAYLOAD_TOOL_NAME,
                    "writes_files": false,
                    "dispatches_input": false,
                    "notes": "Validate selector/text/key/scroll payload shape first."
                },
                {
                    "step": "queue",
                    "tool": AGENT_BROWSER_PAYLOAD_QUEUE_TOOL_NAME,
                    "writes_files": true,
                    "dispatches_input": false,
                    "notes": "Writes only a managed handoff file after permission."
                },
                {
                    "step": "inspect_queue",
                    "tool": AGENT_BROWSER_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                    "writes_files": false,
                    "dispatches_input": false,
                    "notes": "Confirm queue item schema/action before WebPreview import."
                },
                {
                    "step": "webpreview_import",
                    "surface": "WebPreview More menu",
                    "writes_files": false,
                    "dispatches_input": false,
                    "notes": "Import creates an import receipt but still does not execute input."
                },
                {
                    "step": "webpreview_execute",
                    "surface": "permissioned WebPreview executor",
                    "required_gates": [
                        "interactive permission unlock",
                        "fresh action preflight",
                        "native trace receipt",
                        "dispatch QA checklist",
                        "executor receipt"
                    ]
                }
            ],
        },
        "chrome_managed_playwright_safe_actions": {
            "goal": "Run managed external Chrome only for currently allowlisted Playwright actions.",
            "safe_for": ["open_url", "screenshot", "set_viewport", "wait_for_selector"],
            "blocked_actions": ["click", "type_text", "press_key", "scroll"],
            "ordered_steps": [
                {
                    "step": "inspect_runtime",
                    "tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                    "notes": "Check host, Playwright, DX extension, adapter, and managed profile readiness."
                },
                {
                    "step": "compose_and_queue_payload",
                    "tools": [AGENT_CHROME_PAYLOAD_TOOL_NAME, AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME],
                    "notes": "Queue only managed-profile payloads."
                },
                {
                    "step": "inspect_queue",
                    "tool": AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                    "notes": "Confirm queue item and managed asset readiness."
                },
                {
                    "step": "request_runner_gate",
                    "tool": AGENT_CHROME_RUNNER_GATE_TOOL_NAME,
                    "notes": "Write a permissioned runner-gate receipt without launching Chrome."
                },
                {
                    "step": "prepare_adapter",
                    "tool": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
                    "notes": "Write or verify the managed adapter artifact."
                },
                {
                    "step": "invoke_adapter",
                    "tool": AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME,
                    "required_gates": [
                        "execute_adapter=true",
                        "safe action only",
                        "managed profile root",
                        "runner-gate receipt"
                    ]
                },
                {
                    "step": "inspect_execution",
                    "tool": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
                    "notes": "Read request and receipt summaries after execution."
                }
            ],
        },
        "pc_use_zed_window_future_input": {
            "goal": "Prepare future Zed-window PC-use input while keeping current tools read-only or receipt-gated.",
            "safe_for_now": ["inspect_context", "inspect_targets", "inspect_snapshots", "compose_payload", "queue_payload", "runner_gate_receipt"],
            "still_blocked": ["actual_focus", "actual_click", "actual_type", "actual_os_desktop_control"],
            "ordered_steps": [
                {
                    "step": "inspect_contract",
                    "tools": [
                        AGENT_PC_USE_TARGET_MANIFEST_TOOL_NAME,
                        AGENT_PC_USE_TARGET_SNAPSHOT_TOOL_NAME,
                        AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_TOOL_NAME,
                        AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME
                    ],
                    "notes": "Use live snapshot target ids and keep the producing snapshot id."
                },
                {
                    "step": "compose_payload",
                    "tool": AGENT_PC_USE_PAYLOAD_TOOL_NAME,
                    "required_fields": ["target_id", "target_snapshot_id"],
                    "notes": "Focus/click/type payloads require a snapshot receipt for future input-ready target ids."
                },
                {
                    "step": "queue_and_inspect",
                    "tools": [
                        AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME,
                        AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME
                    ],
                    "notes": "Managed handoff only; no desktop action is dispatched."
                },
                {
                    "step": "request_runner_gate",
                    "tool": AGENT_PC_USE_RUNNER_GATE_TOOL_NAME,
                    "notes": "Writes an auditable future-executor gate receipt without controlling Zed."
                },
                {
                    "step": "inspect_receipts",
                    "tool": AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME,
                    "notes": "Report latest blocked or ready-future receipt state."
                }
            ],
        }
    })
}

fn dir_probe(path: &Path) -> Value {
    match fs::metadata(path) {
        Ok(metadata) => serde_json::json!({
            "path": path_string(path),
            "exists": true,
            "is_dir": metadata.is_dir(),
            "modified_at_ms": modified_at_ms(&metadata),
            "json_file_count": json_file_count(path),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => serde_json::json!({
            "path": path_string(path),
            "exists": false,
            "is_dir": false,
        }),
        Err(error) => serde_json::json!({
            "path": path_string(path),
            "exists": false,
            "is_dir": false,
            "error": error.to_string(),
        }),
    }
}

fn file_probe(
    path: &Path,
    expected_schema: Option<&str>,
    nested_schema_pointer: Option<&str>,
    include_json_summary: bool,
) -> Value {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return serde_json::json!({
                "path": path_string(path),
                "exists": false,
                "is_file": false,
            });
        }
        Err(error) => {
            return serde_json::json!({
                "path": path_string(path),
                "exists": false,
                "is_file": false,
                "error": error.to_string(),
            });
        }
    };

    let mut probe = serde_json::json!({
        "path": path_string(path),
        "exists": true,
        "is_file": metadata.is_file(),
        "byte_len": metadata.len(),
        "modified_at_ms": modified_at_ms(&metadata),
    });

    if metadata.is_file() && include_json_summary {
        let summary = if metadata.len() <= MAX_HANDOFF_PREVIEW_BYTES {
            json_summary(path, expected_schema, nested_schema_pointer)
        } else {
            serde_json::json!({
                "state": "skipped_too_large",
                "max_preview_bytes": MAX_HANDOFF_PREVIEW_BYTES,
            })
        };
        probe["json"] = summary;
    }

    probe
}

fn json_summary(
    path: &Path,
    expected_schema: Option<&str>,
    nested_schema_pointer: Option<&str>,
) -> Value {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return serde_json::json!({
                "state": "read_error",
                "error": error.to_string(),
            });
        }
    };
    let parsed = match serde_json::from_slice::<Value>(&bytes) {
        Ok(parsed) => parsed,
        Err(error) => {
            return serde_json::json!({
                "state": "parse_error",
                "error": error.to_string(),
            });
        }
    };

    let schema = parsed.get("schema").and_then(Value::as_str);
    let nested_schema = nested_schema_pointer
        .and_then(|pointer| parsed.pointer(pointer))
        .and_then(Value::as_str);
    let schema_matches = expected_schema
        .map(|expected| schema == Some(expected) || nested_schema == Some(expected))
        .unwrap_or(true);

    serde_json::json!({
        "state": if schema_matches { "valid_json" } else { "schema_mismatch" },
        "schema": schema,
        "nested_schema": nested_schema,
        "expected_schema": expected_schema,
        "schema_matches": schema_matches,
        "top_level_keys": parsed
            .as_object()
            .map(|object| object.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default(),
    })
}

fn executable_probe(path: Option<PathBuf>) -> Value {
    serde_json::json!({
        "available": path.is_some(),
        "path": path.as_ref().map(path_string),
    })
}

fn find_executable(candidates: &[&str]) -> Option<PathBuf> {
    for candidate in candidates {
        let candidate_path = PathBuf::from(candidate);
        if candidate_path.components().count() > 1 && candidate_path.is_file() {
            return Some(candidate_path);
        }
    }

    let path_var = env::var_os("PATH")?;
    for directory in env::split_paths(&path_var) {
        for candidate in candidates {
            let path = directory.join(candidate);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

fn json_file_count(path: &Path) -> Option<usize> {
    let directory = fs::read_dir(path).ok()?;
    let mut count = 0;
    for entry in directory.flatten() {
        if entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("json")
        {
            count += 1;
        }
    }
    Some(count)
}

fn workspace_roots_for_project(project: &Entity<Project>, cx: &App) -> Vec<PathBuf> {
    project
        .read(cx)
        .visible_worktrees(cx)
        .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf())
        .collect()
}

fn modified_at_ms(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|modified| epoch_millis(modified))
}

fn current_epoch_millis() -> u64 {
    epoch_millis(SystemTime::now()).unwrap_or_default()
}

fn epoch_millis(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}
