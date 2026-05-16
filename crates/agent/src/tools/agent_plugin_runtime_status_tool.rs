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
        AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
        AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA,
        AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME,
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
    agent_plugin_asset_provisioner_tool::{
        AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
        AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_FILE_NAME,
        AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA,
        AGENT_PLUGIN_ASSET_PROVISIONING_RESULT_SCHEMA, AGENT_PLUGIN_ASSET_READINESS_SUMMARY_SCHEMA,
    },
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
const AGENT_PLUGIN_BOOTSTRAP_READINESS_SCHEMA: &str = "zed.agent_plugins.bootstrap_readiness.v1";
const AGENT_PLUGIN_BOOTSTRAP_MANIFEST_SCHEMA: &str = "zed.agent_plugins.bootstrap_manifest.v1";
const AGENT_PLUGIN_BOOTSTRAP_PREPARE_REQUEST_SCHEMA: &str =
    "zed.agent_plugins.bootstrap_prepare_request.v1";
const AGENT_PLUGIN_BOOTSTRAP_ASSET_PLAN_SCHEMA: &str = "zed.agent_plugins.bootstrap_asset_plan.v1";
const AGENT_PLUGIN_MANAGED_ASSET_OPERATOR_RECIPE_SCHEMA: &str =
    "zed.agent_plugins.managed_asset_operator_recipe.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_BLOCKERS_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_blocker_summary.v1";
const MANAGED_CHROME_EXECUTION_RECEIPT_PREFIX: &str = "managed-chrome-execution-receipt-";

const MAX_HANDOFF_PREVIEW_BYTES: u64 = 1_048_576;
const OBSERVABILITY_FRESHNESS_WINDOW_MS: u64 = 24 * 60 * 60 * 1000;

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
    /// Include the final manual/runtime validation matrix for this plugin set.
    pub include_validation_matrix: bool,
    /// Include compact plugin observability profiles and runtime-green blockers.
    pub include_observability_profiles: bool,
}

impl Default for AgentPluginRuntimeStatusToolInput {
    fn default() -> Self {
        Self {
            root_mode: AgentPluginRuntimeStatusRootMode::Workspace,
            include_latest_handoffs: true,
            include_host_checks: true,
            include_next_actions: true,
            include_workflows: true,
            include_validation_matrix: true,
            include_observability_profiles: true,
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
    bootstrap_manifest: PathBuf,
    asset_provisioning_receipt: PathBuf,
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
        let bootstrap_manifest = plugin_root.join("agent-plugin-bootstrap.json");
        let asset_provisioning_receipt =
            plugin_root.join(AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_FILE_NAME);
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
            bootstrap_manifest,
            asset_provisioning_receipt,
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
            &self.bootstrap_manifest,
            &self.asset_provisioning_receipt,
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
    let bootstrap_readiness = input
        .include_host_checks
        .then(|| bootstrap_readiness(roots, host_checks.as_ref()));
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
            "prepare_managed_assets": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
        },
        "roots": roots_value(roots),
        "plugins": {
            "browser": browser,
            "chrome": chrome,
            "pc_use": pc_use,
        },
        "host": host_checks,
        "bootstrap_readiness": bootstrap_readiness,
        "runtime_green_blocker_summary": runtime_green_blocker_summary(status, roots),
        "workflow_recipes": input.include_workflows.then(workflow_recipes),
        "validation_matrix": input.include_validation_matrix.then(validation_matrix),
        "observability_profiles": input
            .include_observability_profiles
            .then(|| observability_profiles(status, roots)),
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
            "prepare_managed_assets": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
            "prepare_playwright_adapter": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
            "invoke_playwright_adapter": AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME,
            "inspect_playwright_executions": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
        },
        "schemas": {
            "payload_queue_item": AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA,
            "runner_receipt": AGENT_CHROME_RUNNER_RECEIPT_SCHEMA,
            "asset_provisioning_result": AGENT_PLUGIN_ASSET_PROVISIONING_RESULT_SCHEMA,
            "asset_provisioning_receipt": AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA,
            "managed_asset_operator_recipe": AGENT_PLUGIN_MANAGED_ASSET_OPERATOR_RECIPE_SCHEMA,
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
            "bootstrap_manifest": file_probe(
                &roots.bootstrap_manifest,
                Some(AGENT_PLUGIN_BOOTSTRAP_MANIFEST_SCHEMA),
                None,
                include_latest_handoff,
            ),
            "playwright_root": dir_probe(&roots.playwright_root),
            "playwright_package": file_probe(&playwright_package, None, None, false),
            "asset_provisioning_receipt": file_probe(
                &roots.asset_provisioning_receipt,
                Some(AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA),
                None,
                include_latest_handoff,
            ),
            "asset_readiness_summary": asset_readiness_summary_probe(&roots.asset_provisioning_receipt),
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
            "asset provisioning receipt confirms managed asset status",
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

fn bootstrap_readiness(roots: &AgentPluginRuntimeRoots, host_checks: Option<&Value>) -> Value {
    let playwright_package = roots
        .playwright_root
        .join("node_modules")
        .join("playwright")
        .join("package.json");
    let dx_extension_manifest = roots.dx_extension_root.join("manifest.json");
    let bootstrap_manifest_schema = json_file_schema(&roots.bootstrap_manifest);
    let bootstrap_manifest_ready =
        bootstrap_manifest_schema.as_deref() == Some(AGENT_PLUGIN_BOOTSTRAP_MANIFEST_SCHEMA);
    let adapter_manifest_ready = json_file_schema(&roots.chrome_adapter_manifest).as_deref()
        == Some(AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA);

    let checks = vec![
        runtime_bootstrap_check(
            "workspace.root",
            "Workspace root",
            roots.active_project_root.is_some(),
            roots.active_project_root.as_ref().map(path_string),
            "host_blocker",
            "A workspace root keeps managed plugin assets inside the active project.",
        ),
        runtime_bootstrap_check(
            "host.node",
            "Node.js runtime",
            host_probe_available(host_checks, "node"),
            host_probe_path(host_checks, "node"),
            "host_blocker",
            "Playwright and Chrome plugin bootstrapping need Node.js.",
        ),
        runtime_bootstrap_check(
            "host.npm",
            "npm package manager",
            host_probe_available(host_checks, "npm"),
            host_probe_path(host_checks, "npm"),
            "host_blocker",
            "Playwright package provisioning needs npm or a compatible npm executable.",
        ),
        runtime_bootstrap_check(
            "host.chrome_or_edge",
            "Chrome or Edge executable",
            host_probe_available(host_checks, "chrome_or_edge"),
            host_probe_path(host_checks, "chrome_or_edge"),
            "host_blocker",
            "Managed external Chrome execution needs Chrome, Edge, or Chromium.",
        ),
        runtime_bootstrap_check(
            "root.managed_base",
            "Managed runtime base root",
            roots.managed_base_root.is_dir(),
            Some(path_string(&roots.managed_base_root)),
            "provision_required",
            "Create this managed base root before writing plugin queues, assets, profiles, or receipts.",
        ),
        runtime_bootstrap_check(
            "root.plugin",
            "Managed plugin root",
            roots.plugin_root.is_dir(),
            Some(path_string(&roots.plugin_root)),
            "provision_required",
            "Create this managed plugin root before writing Browser, Chrome, or PC-use handoff files.",
        ),
        runtime_bootstrap_check(
            "root.playwright",
            "Managed Playwright root",
            roots.playwright_root.is_dir(),
            Some(path_string(&roots.playwright_root)),
            "provision_required",
            "Create this managed Playwright root before installing or preparing Playwright adapter files.",
        ),
        runtime_bootstrap_check(
            "root.dx_chrome_extension",
            "Managed DX Chrome extension root",
            roots.dx_extension_root.is_dir(),
            Some(path_string(&roots.dx_extension_root)),
            "provision_required",
            "Create this managed extension root before unpacking the DX Chrome extension.",
        ),
        runtime_bootstrap_check(
            "profile.managed_chrome",
            "Managed Chrome profile root",
            roots.managed_chrome_profile_root.is_dir(),
            Some(path_string(&roots.managed_chrome_profile_root)),
            "provision_required",
            "Create this profile root and never write into a user's real Chrome, Edge, or Firefox profile.",
        ),
        runtime_bootstrap_check(
            "asset.bootstrap_manifest",
            "Agent plugin bootstrap manifest",
            bootstrap_manifest_ready,
            Some(path_string(&roots.bootstrap_manifest)),
            "provision_required",
            "Write the bootstrap manifest so future agents can verify the managed-root policy before provisioning assets.",
        ),
        runtime_bootstrap_check(
            "asset.playwright_package",
            "Managed Playwright package",
            playwright_package.is_file(),
            Some(path_string(&playwright_package)),
            "provision_required",
            "Install Playwright into the managed tools root before launching external Chrome.",
        ),
        runtime_bootstrap_check(
            "asset.playwright_adapter_manifest",
            "Managed Playwright adapter manifest",
            adapter_manifest_ready,
            Some(path_string(&roots.chrome_adapter_manifest)),
            "provision_required",
            "Prepare the managed Playwright adapter artifact before launching external Chrome.",
        ),
        runtime_bootstrap_check(
            "asset.playwright_adapter_runner",
            "Managed Playwright adapter runner",
            roots.chrome_runner_script.is_file(),
            Some(path_string(&roots.chrome_runner_script)),
            "provision_required",
            "Prepare the managed Playwright runner script before launching external Chrome.",
        ),
        runtime_bootstrap_check(
            "asset.dx_chrome_extension",
            "DX Chrome extension manifest",
            dx_extension_manifest.is_file(),
            Some(path_string(&dx_extension_manifest)),
            "provision_required",
            "Use the managed asset provisioner to copy a local unpacked DX Chrome extension before loading managed Chrome with the bridge.",
        ),
    ];

    let host_blockers = runtime_readiness_issues(&checks, "host_blocker");
    let provision_required = runtime_readiness_issues(&checks, "provision_required");
    let status = if !host_blockers.is_empty() {
        "blocked_missing_host_dependencies"
    } else if !provision_required.is_empty() {
        "ready_to_provision"
    } else {
        "ready_for_managed_chrome_executor"
    };

    serde_json::json!({
        "schema": AGENT_PLUGIN_BOOTSTRAP_READINESS_SCHEMA,
        "generated_at_ms": current_epoch_millis(),
        "status": status,
        "phase_summary": runtime_bootstrap_phase_summary(&checks),
        "manifest": {
            "path": path_string(&roots.bootstrap_manifest),
            "expected_schema": AGENT_PLUGIN_BOOTSTRAP_MANIFEST_SCHEMA,
            "actual_schema": bootstrap_manifest_schema,
            "ready": bootstrap_manifest_ready,
        },
        "prepare_runtime_handoff": {
            "tool_name": AgentPluginBootstrapTool::NAME,
            "dry_run_payload": {
                "root_mode": "workspace",
                "create_managed_roots": false,
                "write_bootstrap_manifest": false
            },
            "workspace_payload": {
                "root_mode": "workspace",
                "create_managed_roots": true,
                "write_bootstrap_manifest": true
            },
            "zed_data_payload": {
                "root_mode": "zed_data",
                "create_managed_roots": true,
                "write_bootstrap_manifest": true
            },
            "requires_permission_for_writes": true,
            "downloads_packages": false,
            "launches_browser": false,
            "touches_real_browser_profiles": false,
        },
        "prepare_runtime_request": runtime_bootstrap_prepare_request(
            status,
            roots.active_project_root.is_some()
        ),
        "asset_provisioning_plan": runtime_bootstrap_asset_provisioning_plan(
            status,
            roots,
            &playwright_package,
            bootstrap_manifest_ready,
            adapter_manifest_ready,
            &dx_extension_manifest,
        ),
        "checks": checks,
        "host_blockers": host_blockers,
        "provision_required": provision_required,
        "safety": {
            "read_only": true,
            "writes_files": false,
            "runs_node": false,
            "launches_browser": false,
            "touches_real_browser_profiles": false,
        },
    })
}

fn runtime_bootstrap_asset_provisioning_plan(
    status: &str,
    roots: &AgentPluginRuntimeRoots,
    playwright_package: &Path,
    bootstrap_manifest_ready: bool,
    adapter_manifest_ready: bool,
    dx_extension_manifest: &Path,
) -> Value {
    let root_mode = runtime_request_root_mode(roots);
    let adapter_ready = adapter_manifest_ready && roots.chrome_runner_script.is_file();

    serde_json::json!({
        "schema": AGENT_PLUGIN_BOOTSTRAP_ASSET_PLAN_SCHEMA,
        "readiness_status": status,
        "safe_to_start_after_plan": status == "ready_for_managed_chrome_executor",
        "root_mode": root_mode,
        "operator_recipe": managed_asset_operator_recipe(root_mode),
        "steps": [
            {
                "id": "bootstrap.manifest",
                "label": "Agent plugin bootstrap manifest",
                "state": if bootstrap_manifest_ready { "ready" } else { "pending_prepare_runtime" },
                "path": path_string(&roots.bootstrap_manifest),
                "tool_name": AgentPluginBootstrapTool::NAME,
                "apply_payload": {
                    "root_mode": root_mode,
                    "create_managed_roots": true,
                    "write_bootstrap_manifest": true
                },
                "requires_authorization": true,
                "runs_node": false,
                "downloads_packages": false,
                "launches_browser": false
            },
            {
                "id": "playwright.package",
                "label": "Managed Playwright package",
                "state": if playwright_package.is_file() { "ready" } else { "pending_manual_or_future_provisioner" },
                "managed_root": path_string(&roots.playwright_root),
                "expected_package_json": path_string(playwright_package),
                "requires_authorization": true,
                "runs_node": true,
                "downloads_packages": true,
                "launches_browser": false,
                "touches_real_browser_profiles": false
            },
            {
                "id": "playwright.adapter",
                "label": "Managed Chrome Playwright adapter",
                "state": if adapter_ready { "ready" } else { "pending_prepare_managed_adapter" },
                "tool_name": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
                "managed_root": path_string(&roots.chrome_adapter_root),
                "expected_manifest": path_string(&roots.chrome_adapter_manifest),
                "expected_runner": path_string(&roots.chrome_runner_script),
                "dry_run_payload": {
                    "root_mode": root_mode,
                    "write_adapter_files": false,
                    "include_script_preview": false
                },
                "write_payload": {
                    "root_mode": root_mode,
                    "write_adapter_files": true,
                    "include_script_preview": false
                },
                "requires_authorization": true,
                "runs_node": false,
                "downloads_packages": false,
                "launches_browser": false
            },
            {
                "id": "dx.chrome_extension",
                "label": "Managed DX Chrome extension",
                "state": if dx_extension_manifest.is_file() { "ready" } else { "pending_manual_or_future_provisioner" },
                "managed_root": path_string(&roots.dx_extension_root),
                "expected_manifest": path_string(dx_extension_manifest),
                "requires_authorization": true,
                "runs_node": false,
                "downloads_packages": true,
                "launches_browser": false,
                "touches_real_browser_profiles": false
            }
        ],
        "after_asset_provisioning_verification": {
            "catalog_tool": AgentPluginCatalogTool::NAME,
            "runtime_status_tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
            "asset_provisioner_tool": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
            "asset_provisioning_result_schema": AGENT_PLUGIN_ASSET_PROVISIONING_RESULT_SCHEMA,
            "asset_provisioning_receipt_schema": AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA,
            "required_ready_checks": [
                "asset.bootstrap_manifest",
                "asset.provisioning_receipt",
                "asset.playwright_package",
                "asset.playwright_adapter_manifest",
                "asset.playwright_adapter_runner",
                "asset.dx_chrome_extension"
            ]
        },
        "safety": {
            "plan_is_metadata_only": true,
            "writes_files": false,
            "runs_node": false,
            "launches_browser": false,
            "dispatches_input": false,
            "touches_real_browser_profiles": false,
            "requires_receipts_before_executor_actions": true
        }
    })
}

fn managed_asset_operator_recipe(root_mode: &str) -> Value {
    serde_json::json!({
        "schema": AGENT_PLUGIN_MANAGED_ASSET_OPERATOR_RECIPE_SCHEMA,
        "root_mode": root_mode,
        "goal": "Prepare managed Browser and Chrome plugin assets in the safe order before any external Chrome execution.",
        "ordered_steps": [
            {
                "step": "inspect_bootstrap_readiness",
                "tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                "recommended_payload": {
                    "root_mode": root_mode,
                    "include_bootstrap_readiness": true,
                    "include_observability_profiles": true,
                    "include_next_actions": true
                },
                "writes_files": false,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            },
            {
                "step": "prepare_managed_roots",
                "tool": AgentPluginBootstrapTool::NAME,
                "recommended_payload": {
                    "root_mode": root_mode,
                    "create_managed_roots": true,
                    "write_bootstrap_manifest": true
                },
                "requires_authorization": true,
                "writes_files": true,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            },
            {
                "step": "write_asset_receipt",
                "tool": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
                "recommended_payload": {
                    "root_mode": root_mode,
                    "write_asset_receipt": true,
                    "copy_dx_chrome_extension": false,
                    "dx_chrome_extension_source_root": Value::Null,
                    "overwrite_existing_files": false,
                    "include_file_preview": true
                },
                "requires_authorization": true,
                "writes_files": true,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            },
            {
                "step": "copy_dx_chrome_extension_if_missing",
                "tool": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
                "recommended_payload": {
                    "root_mode": root_mode,
                    "write_asset_receipt": true,
                    "copy_dx_chrome_extension": true,
                    "dx_chrome_extension_source_root": "<local unpacked extension root>",
                    "overwrite_existing_files": false,
                    "include_file_preview": true
                },
                "requires_authorization": true,
                "writes_files": true,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            },
            {
                "step": "prepare_playwright_adapter",
                "tool": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
                "recommended_payload": {
                    "root_mode": root_mode,
                    "write_adapter_files": true,
                    "include_script_preview": false
                },
                "requires_authorization": true,
                "writes_files": true,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            },
            {
                "step": "inspect_runtime_status_again",
                "tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                "recommended_payload": {
                    "root_mode": root_mode,
                    "include_bootstrap_readiness": true,
                    "include_observability_profiles": true,
                    "include_latest_handoff": true,
                    "include_next_actions": true
                },
                "writes_files": false,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            },
            {
                "step": "final_windows_validation",
                "manual_command": "just run",
                "when": "only after managed asset status, adapter readiness, Browser/WebPreview receipts, managed Chrome receipts, and PC-use receipts are ready for a final runtime pass",
                "writes_files": false,
                "dispatches_input": "manual_validation_only"
            }
        ],
        "safety": {
            "recipe_is_metadata_only": true,
            "never_write_to_real_browser_profiles": true,
            "external_browser_launch_requires_later_permissioned_adapter_step": true,
            "input_dispatch_requires_webpreview_or_future_executor_receipts": true
        }
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
        "bootstrap_manifest": path_string(&roots.bootstrap_manifest),
        "asset_provisioning_receipt": path_string(&roots.asset_provisioning_receipt),
        "browser_queue_dir": path_string(&roots.browser_queue_dir),
        "chrome_queue_dir": path_string(&roots.chrome_queue_dir),
        "chrome_receipt_dir": path_string(&roots.chrome_receipt_dir),
        "chrome_execution_dir": path_string(&roots.chrome_execution_dir),
        "pc_use_root": path_string(&roots.pc_use_root),
    })
}

fn runtime_request_root_mode(roots: &AgentPluginRuntimeRoots) -> &'static str {
    if roots.active_project_root.is_some()
        && matches!(roots.root_mode, AgentPluginRuntimeStatusRootMode::Workspace)
    {
        "workspace"
    } else {
        "zed_data"
    }
}

fn host_probe_available(host_checks: Option<&Value>, key: &str) -> bool {
    host_checks
        .and_then(|checks| checks.get(key))
        .and_then(|probe| probe.get("available"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn host_probe_path(host_checks: Option<&Value>, key: &str) -> Option<String> {
    host_checks
        .and_then(|checks| checks.get(key))
        .and_then(|probe| probe.get("path"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn runtime_bootstrap_check(
    id: &str,
    label: &str,
    ready: bool,
    path: Option<String>,
    missing_kind: &str,
    details: &str,
) -> Value {
    serde_json::json!({
        "id": id,
        "label": label,
        "state": if ready { "ready" } else { missing_kind },
        "ready": ready,
        "path": path,
        "details": details,
    })
}

fn runtime_readiness_issues(checks: &[Value], state: &str) -> Vec<Value> {
    checks
        .iter()
        .filter(|check| {
            check
                .get("state")
                .and_then(Value::as_str)
                .is_some_and(|check_state| check_state == state)
        })
        .cloned()
        .collect()
}

fn runtime_bootstrap_phase_summary(checks: &[Value]) -> Value {
    let host = runtime_bootstrap_phase("host_dependencies", checks, &["workspace.", "host."]);
    let roots = runtime_bootstrap_phase("managed_roots", checks, &["root.", "profile."]);
    let assets = runtime_bootstrap_phase("managed_assets", checks, &["asset."]);
    let ready_phase_count = [&host, &roots, &assets]
        .into_iter()
        .filter(|phase| phase.get("ready").and_then(Value::as_bool).unwrap_or(false))
        .count();

    serde_json::json!({
        "host_dependencies": host,
        "managed_roots": roots,
        "managed_assets": assets,
        "ready_phase_count": ready_phase_count,
        "total_phase_count": 3,
    })
}

fn runtime_bootstrap_phase(name: &str, checks: &[Value], prefixes: &[&str]) -> Value {
    let phase_checks = checks
        .iter()
        .filter(|check| {
            check
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| prefixes.iter().any(|prefix| id.starts_with(prefix)))
        })
        .cloned()
        .collect::<Vec<_>>();
    let total = phase_checks.len();
    let ready = phase_checks
        .iter()
        .filter(|check| check.get("ready").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let missing = phase_checks
        .iter()
        .filter_map(|check| {
            let is_ready = check.get("ready").and_then(Value::as_bool).unwrap_or(false);
            (!is_ready)
                .then(|| check.get("id").and_then(Value::as_str).map(str::to_owned))
                .flatten()
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "name": name,
        "ready": ready == total && total > 0,
        "ready_check_count": ready,
        "total_check_count": total,
        "missing": missing,
    })
}

fn runtime_bootstrap_prepare_request(status: &str, workspace_available: bool) -> Value {
    let root_mode = if workspace_available {
        "workspace"
    } else {
        "zed_data"
    };
    let should_prepare = status == "ready_to_provision";

    serde_json::json!({
        "schema": AGENT_PLUGIN_BOOTSTRAP_PREPARE_REQUEST_SCHEMA,
        "tool_name": AgentPluginBootstrapTool::NAME,
        "readiness_status": status,
        "should_call_prepare": should_prepare,
        "authorization_required": should_prepare,
        "recommended_payload": {
            "root_mode": root_mode,
            "create_managed_roots": should_prepare,
            "write_bootstrap_manifest": should_prepare
        },
        "dry_run_payload": {
            "root_mode": root_mode,
            "create_managed_roots": false,
            "write_bootstrap_manifest": false
        },
        "blocked_by": match status {
            "blocked_missing_host_dependencies" => vec!["host_dependencies"],
            "ready_for_managed_chrome_executor" => vec!["already_prepared"],
            _ => Vec::new(),
        },
        "after_prepare_verification": {
            "tool_name": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
            "payload": {
                "root_mode": root_mode,
                "include_host_checks": true,
                "include_bootstrap_readiness": true,
                "include_latest_handoffs": true,
                "include_next_actions": true
            }
        },
        "safety": {
            "writes_only_when_authorized": true,
            "downloads_packages": false,
            "launches_browser": false,
            "touches_real_browser_profiles": false,
            "workspace_preferred_when_available": true,
        },
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
    if !roots.dx_extension_root.join("manifest.json").is_file()
        || !roots.asset_provisioning_receipt.is_file()
    {
        actions.push(
            "Run prepare_agent_plugin_managed_assets to write an asset receipt or copy a local unpacked DX Chrome extension into the managed plugin root.",
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

fn runtime_green_blocker_summary(runtime_status: &str, roots: &AgentPluginRuntimeRoots) -> Value {
    let generated_at_ms = current_epoch_millis();
    let mut blockers = Vec::new();

    if runtime_status == "blocked_unmanaged_paths" {
        blockers.push(serde_json::json!({
            "area": "managed_roots",
            "id": "unmanaged_paths",
            "severity": "critical",
            "reason": "One or more plugin runtime paths are outside the selected managed root.",
            "next_actions": [
                "Inspect root_mode and managed root construction before any provisioning or execution.",
                AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                AgentPluginCatalogTool::NAME
            ]
        }));
    }

    blockers.push(serde_json::json!({
        "area": "browser_webpreview",
        "id": "manual_final_result_not_visible_to_runtime_status",
        "severity": "manual_required",
        "reason": "Runtime status cannot prove the in-memory WebPreview final validation result. The WebPreview final result must be copied, filled, imported, and sent before claiming runtime-green.",
        "next_actions": [
            "copy_agent_browser_final_validation_bundle",
            "copy_agent_browser_final_validation_result_template",
            "import_agent_browser_final_validation_result_from_clipboard",
            "send_agent_browser_final_validation_result_to_agent"
        ]
    }));

    if !roots.browser_latest_payload.is_file() {
        blockers.push(serde_json::json!({
            "area": "browser_webpreview",
            "id": "missing_latest_browser_payload",
            "severity": "runtime_evidence_missing",
            "reason": "No latest managed Browser payload is present for WebPreview import validation.",
            "next_actions": [
                AGENT_BROWSER_PAYLOAD_TOOL_NAME,
                AGENT_BROWSER_PAYLOAD_QUEUE_TOOL_NAME,
                AGENT_BROWSER_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                "import_agent_browser_action_payload_from_managed_queue"
            ]
        }));
    }

    let asset_summary = asset_readiness_summary_probe(&roots.asset_provisioning_receipt);
    if asset_summary
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        == false
    {
        blockers.push(serde_json::json!({
            "area": "managed_chrome.assets",
            "id": "asset_readiness_not_ready",
            "severity": "runtime_evidence_missing",
            "reason": "Managed Chrome assets are not fully ready according to the latest asset provisioning receipt.",
            "summary_state": asset_summary.get("state").and_then(Value::as_str),
            "summary_status": asset_summary.get("status").and_then(Value::as_str),
            "asset_blockers": asset_summary.get("blockers").cloned().unwrap_or_else(|| serde_json::json!([])),
            "asset_warnings": asset_summary.get("warnings").cloned().unwrap_or_else(|| serde_json::json!([])),
            "next_actions": asset_summary.get("next_actions").cloned().unwrap_or_else(|| serde_json::json!([
                AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
                AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME
            ]))
        }));
    }

    if !roots.chrome_adapter_manifest.is_file() || !roots.chrome_runner_script.is_file() {
        blockers.push(serde_json::json!({
            "area": "managed_chrome.adapter",
            "id": "missing_playwright_adapter_files",
            "severity": "runtime_evidence_missing",
            "reason": "The managed Playwright adapter manifest and runner script must exist before external Chrome execution.",
            "next_actions": [
                AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
                AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME
            ]
        }));
    }

    if !roots.chrome_latest_payload.is_file() {
        blockers.push(serde_json::json!({
            "area": "managed_chrome.execution",
            "id": "missing_latest_chrome_payload",
            "severity": "runtime_evidence_missing",
            "reason": "No latest managed Chrome payload exists for the runner gate and adapter invocation.",
            "next_actions": [
                AGENT_CHROME_PAYLOAD_TOOL_NAME,
                AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME,
                AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME
            ]
        }));
    }

    if !roots.chrome_latest_runner_receipt.is_file() {
        blockers.push(serde_json::json!({
            "area": "managed_chrome.execution",
            "id": "missing_chrome_runner_gate_receipt",
            "severity": "runtime_evidence_missing",
            "reason": "No managed Chrome runner-gate receipt exists for the latest queued payload.",
            "next_actions": [
                AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                AGENT_CHROME_RUNNER_GATE_TOOL_NAME
            ]
        }));
    }

    let latest_chrome_execution_receipt = latest_prefixed_json_file_probe(
        &roots.chrome_execution_dir,
        MANAGED_CHROME_EXECUTION_RECEIPT_PREFIX,
        Some(AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA),
        generated_at_ms,
    );
    if latest_chrome_execution_receipt
        .get("latest_file")
        .and_then(Value::as_str)
        .is_none()
    {
        blockers.push(serde_json::json!({
            "area": "managed_chrome.execution",
            "id": "missing_chrome_execution_receipt",
            "severity": "runtime_evidence_missing",
            "reason": "No managed Chrome execution receipt exists from a safe adapter invocation.",
            "next_actions": [
                AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME,
                AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
                "copy_managed_chrome_execution_status"
            ]
        }));
    }

    if !roots.pc_use_latest_payload.is_file() {
        blockers.push(serde_json::json!({
            "area": "pc_use",
            "id": "missing_pc_use_payload",
            "severity": "runtime_evidence_missing",
            "reason": "No latest PC-use payload exists for the Zed-window receipt-gated path.",
            "next_actions": [
                AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME,
                AGENT_PC_USE_PAYLOAD_TOOL_NAME,
                AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME,
                AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME
            ]
        }));
    }

    if !roots.pc_use_latest_receipt.is_file() {
        blockers.push(serde_json::json!({
            "area": "pc_use",
            "id": "missing_pc_use_runner_receipt",
            "severity": "runtime_evidence_missing",
            "reason": "No PC-use runner-gate receipt exists for the managed Zed-window path.",
            "next_actions": [
                AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                AGENT_PC_USE_RUNNER_GATE_TOOL_NAME,
                AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME
            ]
        }));
    }

    let runtime_evidence_blockers = blockers
        .iter()
        .filter(|blocker| {
            blocker
                .get("severity")
                .and_then(Value::as_str)
                .is_some_and(|severity| {
                    severity == "runtime_evidence_missing" || severity == "critical"
                })
        })
        .count();
    let status = if runtime_evidence_blockers > 0 {
        "blocked_missing_runtime_evidence"
    } else {
        "manual_webpreview_final_result_required"
    };

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_BLOCKERS_SCHEMA,
        "generated_at_ms": generated_at_ms,
        "status": status,
        "runtime_status": runtime_status,
        "runtime_green_candidate": false,
        "blocker_count": blockers.len(),
        "runtime_evidence_blocker_count": runtime_evidence_blockers,
        "manual_blocker_count": blockers.len().saturating_sub(runtime_evidence_blockers),
        "blockers": blockers,
        "latest_evidence": {
            "asset_readiness_summary": asset_summary,
            "managed_chrome_execution_receipt": latest_chrome_execution_receipt,
            "browser_latest_payload": proof_file_probe(
                &roots.browser_latest_payload,
                Some(AGENT_BROWSER_PAYLOAD_QUEUE_ITEM_SCHEMA),
                Some("/payload_packet/schema"),
                generated_at_ms,
            ),
            "pc_use_latest_runner_receipt": proof_file_probe(
                &roots.pc_use_latest_receipt,
                Some(AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA),
                None,
                generated_at_ms,
            ),
        },
        "runtime_green_requires": [
            "no critical or runtime_evidence_missing blockers",
            "WebPreview final validation result imported with runtime_green_candidate=true",
            "one final Windows just run pass when the user is ready"
        ],
        "safety": {
            "summary_is_read_only": true,
            "writes_files": false,
            "runs_node": false,
            "launches_browser": false,
            "dispatches_input": false,
            "touches_real_browser_profiles": false
        }
    })
}

fn observability_profiles(runtime_status: &str, roots: &AgentPluginRuntimeRoots) -> Value {
    let summary_status = if runtime_status == "ready_for_read_only_discovery" {
        "profiles_ready_runtime_validation_pending"
    } else {
        "profiles_available_with_runtime_status_blockers"
    };

    serde_json::json!({
        "status": summary_status,
        "runtime_status": runtime_status,
        "overall_code_score": 94,
        "runtime_green_blocker": "Browser, managed Chrome, and PC-use profiles still need one final Windows runtime validation pass plus imported manual result evidence before the product can be called runtime-green.",
        "proof_freshness": observability_proof_freshness(roots),
        "plugins": {
            "browser": {
                "id": "zed.browser",
                "status": "code_complete_pending_windows_runtime_validation",
                "code_score": 99,
                "proof_handoffs": {
                    "validation_progress": "copy_agent_browser_executor_validation_progress",
                    "final_bundle": "copy_agent_browser_final_validation_bundle",
                    "final_result_template": "copy_agent_browser_final_validation_result_template",
                    "final_result_import": "import_agent_browser_final_validation_result_from_clipboard",
                    "final_result_send": "send_agent_browser_final_validation_result_to_agent"
                },
                "watch_surfaces": [
                    "editor caret and typing latency",
                    "WebPreview focus after navigation or reload",
                    "native click/type/key/scroll/history/cache receipts",
                    "managed Chrome execution receipts",
                    "PC-use queue and runner receipts"
                ]
            },
            "managed_chrome": {
                "id": "zed.chrome",
                "status": "managed_adapter_ready_pending_windows_runtime_validation",
                "code_score": 94,
                "proof_handoffs": {
                    "asset_provisioner_tool": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
                    "queue_inspection_tool": AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                    "runner_gate_tool": AGENT_CHROME_RUNNER_GATE_TOOL_NAME,
                    "adapter_prepare_tool": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
                    "adapter_invoke_tool": AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME,
                    "execution_inspect_tool": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
                    "webpreview_status_copy": "copy_managed_chrome_execution_status",
                    "webpreview_status_send": "send_managed_chrome_execution_status_to_agent"
                },
                "watch_surfaces": [
                    "managed workspace or Zed-data roots only",
                    "asset provisioning receipts prove managed assets were prepared before Chrome execution",
                    "real Chrome, Edge, and Firefox profiles stay untouched",
                    "adapter execution remains limited to open_url, screenshot, set_viewport, and wait_for_selector",
                    "click, type, key, and scroll stay blocked in the managed adapter",
                    "runner and execution receipts stay inspectable"
                ]
            },
            "pc_use": {
                "id": "zed.pc_use",
                "status": "payload_and_receipt_gates_ready_pending_ui_executor_validation",
                "code_score": 90,
                "proof_handoffs": {
                    "context_tool": AGENT_PC_USE_INSPECT_TOOL_NAME,
                    "target_manifest_tool": AGENT_PC_USE_TARGET_MANIFEST_TOOL_NAME,
                    "target_snapshot_tool": AGENT_PC_USE_TARGET_SNAPSHOT_TOOL_NAME,
                    "ui_snapshot_contract_tool": AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_TOOL_NAME,
                    "ui_snapshot_tool": AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME,
                    "payload_queue_inspect_tool": AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                    "runner_receipts_tool": AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME,
                    "webpreview_status_copy": "copy_pc_use_status",
                    "webpreview_status_send": "send_pc_use_status_to_agent"
                },
                "watch_surfaces": [
                    "read-only or managed-root-scoped operations only",
                    "future UI snapshot target ids require matching snapshot receipt ids",
                    "no OS-wide desktop control",
                    "no focus, click, type, screenshot, or process launch in the current gate",
                    "runner receipts stay auditable"
                ]
            }
        },
        "final_proof_order": [
            "run just run once for final Windows runtime validation",
            "prove normal editor typing and caret behavior",
            "prove WebPreview keyboard, pointer, wheel, navigation, and receipt handoffs",
            "prove managed Chrome safe adapter receipts without real-profile writes",
            "prove PC-use context, target, queue, and runner receipt chain stays non-dispatching",
            "fill and import the Browser final validation result template"
        ]
    })
}

fn observability_proof_freshness(roots: &AgentPluginRuntimeRoots) -> Value {
    let generated_at_ms = current_epoch_millis();
    let mut missing = Vec::new();
    let mut stale = Vec::new();

    track_required_proof_file(
        "browser.latest_payload",
        &roots.browser_latest_payload,
        generated_at_ms,
        &mut missing,
        &mut stale,
    );
    track_required_proof_file(
        "chrome.latest_payload",
        &roots.chrome_latest_payload,
        generated_at_ms,
        &mut missing,
        &mut stale,
    );
    track_required_proof_file(
        "chrome.latest_runner_receipt",
        &roots.chrome_latest_runner_receipt,
        generated_at_ms,
        &mut missing,
        &mut stale,
    );
    track_required_proof_file(
        "chrome.asset_provisioning_receipt",
        &roots.asset_provisioning_receipt,
        generated_at_ms,
        &mut missing,
        &mut stale,
    );
    track_required_proof_file(
        "chrome.adapter_manifest",
        &roots.chrome_adapter_manifest,
        generated_at_ms,
        &mut missing,
        &mut stale,
    );
    track_required_proof_file(
        "pc_use.latest_payload",
        &roots.pc_use_latest_payload,
        generated_at_ms,
        &mut missing,
        &mut stale,
    );
    track_required_proof_file(
        "pc_use.latest_runner_receipt",
        &roots.pc_use_latest_receipt,
        generated_at_ms,
        &mut missing,
        &mut stale,
    );

    let status = if !missing.is_empty() {
        "missing_runtime_evidence"
    } else if !stale.is_empty() {
        "stale_runtime_evidence"
    } else {
        "latest_runtime_evidence_present"
    };
    let recovery_actions = observability_recovery_actions(&missing, &stale);

    serde_json::json!({
        "status": status,
        "generated_at_ms": generated_at_ms,
        "freshness_window_ms": OBSERVABILITY_FRESHNESS_WINDOW_MS,
        "missing_required_files": missing,
        "stale_required_files": stale,
        "recovery_actions": recovery_actions,
        "required_files": {
            "browser_latest_payload": proof_file_probe(
                &roots.browser_latest_payload,
                Some(AGENT_BROWSER_PAYLOAD_QUEUE_ITEM_SCHEMA),
                Some("/payload_packet/schema"),
                generated_at_ms,
            ),
            "chrome_latest_payload": proof_file_probe(
                &roots.chrome_latest_payload,
                Some(AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA),
                Some("/payload_packet/schema"),
                generated_at_ms,
            ),
            "chrome_latest_runner_receipt": proof_file_probe(
                &roots.chrome_latest_runner_receipt,
                Some(AGENT_CHROME_RUNNER_RECEIPT_SCHEMA),
                None,
                generated_at_ms,
            ),
            "chrome_asset_provisioning_receipt": proof_file_probe(
                &roots.asset_provisioning_receipt,
                Some(AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA),
                None,
                generated_at_ms,
            ),
            "chrome_asset_readiness_summary": asset_readiness_summary_probe(
                &roots.asset_provisioning_receipt,
            ),
            "chrome_adapter_manifest": proof_file_probe(
                &roots.chrome_adapter_manifest,
                Some(AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA),
                None,
                generated_at_ms,
            ),
            "pc_use_latest_payload": proof_file_probe(
                &roots.pc_use_latest_payload,
                Some(AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA),
                Some("/payload_packet/schema"),
                generated_at_ms,
            ),
            "pc_use_latest_runner_receipt": proof_file_probe(
                &roots.pc_use_latest_receipt,
                Some(AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA),
                None,
                generated_at_ms,
            ),
        },
        "latest_optional_execution_files": {
            "chrome_execution": latest_json_file_probe(&roots.chrome_execution_dir, generated_at_ms),
        },
        "manual_session_state_required": [
            "WebPreview final validation bundle copy/send state",
            "WebPreview final result template copy/send state",
            "WebPreview imported final result state"
        ],
    })
}

fn observability_recovery_actions(missing: &[&str], stale: &[&str]) -> Value {
    let mut actions = Vec::with_capacity(missing.len() + stale.len() + 1);
    for label in missing {
        actions.push(observability_recovery_action(*label, "missing"));
    }
    for label in stale {
        actions.push(observability_recovery_action(*label, "stale"));
    }

    if actions.is_empty() {
        actions.push(serde_json::json!({
            "target": "manual_windows_runtime_result",
            "reason": "latest_managed_evidence_present",
            "steps": [
                "Run one final just run pass when runtime validation is intended.",
                "Exercise editor typing/caret, WebPreview input, managed Chrome safe actions, and PC-use receipt gates.",
                "Fill and import the Browser final validation result template."
            ],
            "dispatches_input": false
        }));
    }

    serde_json::json!({
        "status": if missing.is_empty() && stale.is_empty() {
            "ready_for_manual_runtime_result"
        } else {
            "managed_evidence_refresh_required"
        },
        "actions": actions
    })
}

fn observability_recovery_action(label: &str, reason: &str) -> Value {
    match label {
        "browser.latest_payload" => serde_json::json!({
            "target": label,
            "reason": reason,
            "steps": [
                AGENT_BROWSER_PAYLOAD_TOOL_NAME,
                AGENT_BROWSER_PAYLOAD_QUEUE_TOOL_NAME,
                AGENT_BROWSER_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                "import_agent_browser_action_payload_from_managed_queue"
            ],
            "dispatches_input": false
        }),
        "chrome.latest_payload" => serde_json::json!({
            "target": label,
            "reason": reason,
            "steps": [
                AGENT_CHROME_PAYLOAD_TOOL_NAME,
                AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME,
                AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME
            ],
            "dispatches_input": false
        }),
        "chrome.latest_runner_receipt" => serde_json::json!({
            "target": label,
            "reason": reason,
            "steps": [
                AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                AGENT_CHROME_RUNNER_GATE_TOOL_NAME
            ],
            "dispatches_input": false
        }),
        "chrome.asset_provisioning_receipt" => serde_json::json!({
            "target": label,
            "reason": reason,
            "steps": [
                AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
                AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                AgentPluginCatalogTool::NAME
            ],
            "required_payload_hint": {
                "write_asset_receipt": true,
                "copy_dx_chrome_extension": "set true only with a local unpacked extension source",
                "dx_chrome_extension_source_root": "required only when copying the DX Chrome extension"
            },
            "dispatches_input": false
        }),
        "chrome.adapter_manifest" => serde_json::json!({
            "target": label,
            "reason": reason,
            "steps": [
                AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
                AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME
            ],
            "dispatches_input": false
        }),
        "pc_use.latest_payload" => serde_json::json!({
            "target": label,
            "reason": reason,
            "steps": [
                AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME,
                AGENT_PC_USE_PAYLOAD_TOOL_NAME,
                AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME,
                AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME
            ],
            "dispatches_input": false
        }),
        "pc_use.latest_runner_receipt" => serde_json::json!({
            "target": label,
            "reason": reason,
            "steps": [
                AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                AGENT_PC_USE_RUNNER_GATE_TOOL_NAME,
                AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME
            ],
            "dispatches_input": false
        }),
        _ => serde_json::json!({
            "target": label,
            "reason": reason,
            "steps": [
                AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                AgentPluginCatalogTool::NAME
            ],
            "dispatches_input": false
        }),
    }
}

fn track_required_proof_file(
    label: &'static str,
    path: &Path,
    generated_at_ms: u64,
    missing: &mut Vec<&'static str>,
    stale: &mut Vec<&'static str>,
) {
    let metadata = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata,
        _ => {
            missing.push(label);
            return;
        }
    };

    if modified_at_ms(&metadata)
        .map(|modified_at_ms| {
            generated_at_ms.saturating_sub(modified_at_ms) > OBSERVABILITY_FRESHNESS_WINDOW_MS
        })
        .unwrap_or(false)
    {
        stale.push(label);
    }
}

fn proof_file_probe(
    path: &Path,
    expected_schema: Option<&str>,
    nested_schema_pointer: Option<&str>,
    generated_at_ms: u64,
) -> Value {
    let mut probe = file_probe(path, expected_schema, nested_schema_pointer, true);
    if let Some(modified_at_ms) = probe.get("modified_at_ms").and_then(Value::as_u64) {
        probe["age_seconds"] = serde_json::json!(age_seconds(generated_at_ms, modified_at_ms));
        probe["fresh_within_window"] = serde_json::json!(
            generated_at_ms.saturating_sub(modified_at_ms) <= OBSERVABILITY_FRESHNESS_WINDOW_MS
        );
    }
    probe
}

fn latest_json_file_probe(directory: &Path, generated_at_ms: u64) -> Value {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return serde_json::json!({
                "directory": path_string(directory),
                "exists": false,
                "latest_file": null,
            });
        }
        Err(error) => {
            return serde_json::json!({
                "directory": path_string(directory),
                "exists": false,
                "latest_file": null,
                "error": error.to_string(),
            });
        }
    };

    let mut latest_path = None;
    let mut latest_modified_at_ms = None;
    let mut latest_byte_len = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let modified_ms = modified_at_ms(&metadata).unwrap_or_default();
        if latest_modified_at_ms
            .map(|latest| modified_ms > latest)
            .unwrap_or(true)
        {
            latest_byte_len = metadata.len();
            latest_modified_at_ms = Some(modified_ms);
            latest_path = Some(path);
        }
    }

    serde_json::json!({
        "directory": path_string(directory),
        "exists": true,
        "latest_file": latest_path.as_ref().map(path_string),
        "latest_modified_at_ms": latest_modified_at_ms,
        "latest_age_seconds": latest_modified_at_ms.map(|modified_at_ms| age_seconds(generated_at_ms, modified_at_ms)),
        "latest_byte_len": latest_path.as_ref().map(|_| latest_byte_len),
    })
}

fn latest_prefixed_json_file_probe(
    directory: &Path,
    file_name_prefix: &str,
    expected_schema: Option<&str>,
    generated_at_ms: u64,
) -> Value {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return serde_json::json!({
                "directory": path_string(directory),
                "file_name_prefix": file_name_prefix,
                "exists": false,
                "latest_file": null,
            });
        }
        Err(error) => {
            return serde_json::json!({
                "directory": path_string(directory),
                "file_name_prefix": file_name_prefix,
                "exists": false,
                "latest_file": null,
                "error": error.to_string(),
            });
        }
    };

    let mut latest_path = None;
    let mut latest_modified_at_ms = None;
    let mut latest_byte_len = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with(file_name_prefix)
            || path.extension().and_then(|extension| extension.to_str()) != Some("json")
        {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let modified_ms = modified_at_ms(&metadata).unwrap_or_default();
        if latest_modified_at_ms
            .map(|latest| modified_ms > latest)
            .unwrap_or(true)
        {
            latest_byte_len = metadata.len();
            latest_modified_at_ms = Some(modified_ms);
            latest_path = Some(path);
        }
    }

    let json = latest_path.as_ref().map(|path| {
        if latest_byte_len <= MAX_HANDOFF_PREVIEW_BYTES {
            json_summary(path, expected_schema, None)
        } else {
            serde_json::json!({
                "state": "skipped_too_large",
                "max_preview_bytes": MAX_HANDOFF_PREVIEW_BYTES,
            })
        }
    });

    serde_json::json!({
        "directory": path_string(directory),
        "file_name_prefix": file_name_prefix,
        "exists": true,
        "latest_file": latest_path.as_ref().map(path_string),
        "latest_modified_at_ms": latest_modified_at_ms,
        "latest_age_seconds": latest_modified_at_ms.map(|modified_at_ms| age_seconds(generated_at_ms, modified_at_ms)),
        "latest_byte_len": latest_path.as_ref().map(|_| latest_byte_len),
        "json": json,
    })
}

fn workflow_recipes() -> Value {
    serde_json::json!({
        "managed_asset_operator_recipe": managed_asset_operator_recipe("workspace_or_zed_data"),
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

fn validation_matrix() -> Value {
    serde_json::json!({
        "status": "manual_windows_validation_required_before_claiming_runtime_green",
        "final_runtime_command": "just run",
        "lightweight_checks": [
            "rustfmt touched Rust files",
            "git diff --check",
            "git diff --cached --check",
            "targeted rg for tool registration, catalog payloads, and status schema strings"
        ],
        "browser_webpreview": {
            "goal": "Prove the in-app Browser plugin path can hand off payloads and execute only through permissioned WebPreview UI gates.",
            "cases": [
                "compose a click, type_text, press_key, and scroll payload with the Agent tool",
                "queue the payload in managed workspace or Zed-data roots",
                "inspect the queue before importing it",
                "import from WebPreview and confirm an import receipt is copied or sent to Agent Panel",
                "execute click, type_text, press_key, scroll, back, forward, reload, viewport, and cache-only reset through the permissioned WebPreview executor",
                "confirm screenshot, selected-area capture, annotated screenshot, inspect element, DevTools, and responsive metadata remain discoverable"
            ],
            "must_not_regress": [
                "normal editor typing appears immediately",
                "editor caret remains visible while typing",
                "WebPreview hover, click, right-click, wheel, keyboard focus, and URL editor focus remain separated",
                "hidden or inactive previews cannot receive stale input"
            ]
        },
        "managed_chrome": {
            "goal": "Prove the managed external Chrome plugin path can prepare and run only safe Playwright actions from managed profiles.",
            "cases": [
                "inspect runtime host probes for Node, npm, Chrome or Edge, Playwright package, DX extension manifest, and managed profile roots",
                "compose and queue open_url, screenshot, set_viewport, and wait_for_selector payloads",
                "inspect the managed Chrome queue before any runner request",
                "write a runner-gate receipt without launching Chrome",
                "prepare the managed Playwright adapter files",
                "invoke the adapter only for allowlisted safe actions after a runner-gate receipt exists",
                "inspect execution request and receipt summaries after invocation"
            ],
            "blocked_until_future_gate": [
                "click",
                "type_text",
                "press_key",
                "scroll"
            ],
            "must_not_regress": [
                "real browser profiles are never mutated in place",
                "managed Chrome roots stay under workspace tools or Zed data",
                "blocked actions produce auditable receipts instead of silent success"
            ]
        },
        "pc_use": {
            "goal": "Prove the PC-use plugin path remains target-aware, receipt-gated, and non-desktop-controlling until a true UI executor exists.",
            "cases": [
                "inspect Zed window context and managed PC-use roots",
                "inspect target manifest, target snapshot, UI snapshot contract, and current UI snapshot",
                "compose read-only or future input payloads with matching surface and target ids",
                "require target_snapshot_id for future focus, click, and type target ids",
                "queue and inspect the managed PC-use payload",
                "write and inspect runner-gate receipts"
            ],
            "blocked_until_future_gate": [
                "actual_focus",
                "actual_click",
                "actual_type",
                "os_wide_desktop_control",
                "pixel_screenshot"
            ],
            "must_not_regress": [
                "no screenshots are taken from this read-only status flow",
                "no Zed focus, keyboard, mouse, or OS-wide input is dispatched",
                "unknown zed: target namespaces are not treated as input-ready"
            ]
        },
        "evidence_to_capture": [
            "latest Agent tool JSON output for list_agent_plugins and inspect_agent_plugin_runtime_status",
            "WebPreview import and executor receipts for Browser payloads",
            "managed Chrome queue, runner-gate, adapter request, and execution receipt summaries",
            "PC-use payload queue and runner-gate receipt summaries",
            "manual Windows notes for editor typing/caret and WebPreview focus regression checks"
        ],
        "local_artifacts_to_leave_uncommitted": [
            ".codex-*.log",
            "models/",
            "tools/",
            "inspirations/"
        ]
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

fn asset_readiness_summary_probe(path: &Path) -> Value {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return serde_json::json!({
                "state": "missing_receipt",
                "path": path_string(path),
            });
        }
        Err(error) => {
            return serde_json::json!({
                "state": "read_error",
                "path": path_string(path),
                "error": error.to_string(),
            });
        }
    };
    let parsed = match serde_json::from_slice::<Value>(&bytes) {
        Ok(parsed) => parsed,
        Err(error) => {
            return serde_json::json!({
                "state": "parse_error",
                "path": path_string(path),
                "error": error.to_string(),
            });
        }
    };

    let receipt_schema = parsed.get("schema").and_then(Value::as_str);
    let summary = parsed.get("asset_readiness_summary");
    let summary_schema = summary
        .and_then(|summary| summary.get("schema"))
        .and_then(Value::as_str);
    let receipt_schema_matches =
        receipt_schema == Some(AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA);
    let summary_schema_matches =
        summary_schema == Some(AGENT_PLUGIN_ASSET_READINESS_SUMMARY_SCHEMA);
    let state = if receipt_schema_matches && summary_schema_matches {
        "ready"
    } else if !receipt_schema_matches {
        "receipt_schema_mismatch"
    } else {
        "summary_missing_or_schema_mismatch"
    };

    serde_json::json!({
        "state": state,
        "path": path_string(path),
        "receipt_schema": receipt_schema,
        "receipt_schema_matches": receipt_schema_matches,
        "summary_schema": summary_schema,
        "summary_schema_matches": summary_schema_matches,
        "status": summary.and_then(|summary| summary.get("status")).and_then(Value::as_str),
        "ready": summary.and_then(|summary| summary.get("ready")).and_then(Value::as_bool),
        "ready_count": summary.and_then(|summary| summary.get("ready_count")).and_then(Value::as_u64),
        "required_count": summary.and_then(|summary| summary.get("required_count")).and_then(Value::as_u64),
        "blockers": summary.and_then(|summary| summary.get("blockers")).cloned().unwrap_or_else(|| serde_json::json!([])),
        "warnings": summary.and_then(|summary| summary.get("warnings")).cloned().unwrap_or_else(|| serde_json::json!([])),
        "next_actions": summary.and_then(|summary| summary.get("next_actions")).cloned().unwrap_or_else(|| serde_json::json!([])),
    })
}

fn json_file_schema(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice::<Value>(&bytes)
        .ok()?
        .get("schema")
        .and_then(Value::as_str)
        .map(str::to_owned)
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

fn age_seconds(generated_at_ms: u64, modified_at_ms: u64) -> u64 {
    generated_at_ms.saturating_sub(modified_at_ms) / 1000
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
