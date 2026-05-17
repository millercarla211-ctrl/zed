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
    agent_chrome_payload_queue_inspect_tool::AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
    agent_chrome_payload_tool::{
        AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME, AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA,
        AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME, AGENT_CHROME_PAYLOAD_TOOL_NAME,
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
    agent_pc_use_inspect_tool::AGENT_PC_USE_INSPECT_TOOL_NAME,
    agent_pc_use_payload_queue_inspect_tool::{
        AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME, AGENT_PC_USE_PAYLOAD_QUEUE_INSPECTION_SCHEMA,
    },
    agent_pc_use_payload_tool::{
        AGENT_PC_USE_PAYLOAD_QUEUE_FILE_NAME, AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA,
        AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME, AGENT_PC_USE_PAYLOAD_SCHEMA,
        AGENT_PC_USE_PAYLOAD_STAGE_TOOL_NAME, AGENT_PC_USE_PAYLOAD_TOOL_NAME,
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
const AGENT_PLUGIN_RUNTIME_GREEN_SCORECARD_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_readiness_scorecard.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_OPERATOR_HANDOFF_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_operator_handoff.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_PROOF_PATH_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_proof_path.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_CLAIM_GATE_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_claim_gate.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_CLAIM_READINESS_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_claim_readiness.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_REPORT_GATE_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_report_gate.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_REPORT_BADGE_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_report_badge.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_GUIDE_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_final_proof_guide.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_GUIDE_SUMMARY_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_final_proof_guide_summary.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_FINAL_REPORT_PACKET_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_final_report_packet.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_FINAL_REPORT_PACKET_SUMMARY_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_final_report_packet_summary.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_REPORT_READINESS_CARD_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_report_readiness_card.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_REPORT_READINESS_CARD_SUMMARY_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_report_readiness_card_summary.v1";
const AGENT_PLUGIN_RUNTIME_OBSERVABILITY_DIGEST_SCHEMA: &str =
    "zed.agent_plugins.runtime_observability_digest.v1";
const AGENT_PLUGIN_RUNTIME_OBSERVABILITY_MATRIX_SCHEMA: &str =
    "zed.agent_plugins.runtime_observability_plugin_matrix.v1";
const AGENT_PLUGIN_RUNTIME_OBSERVABILITY_WATCH_ROLLUP_SCHEMA: &str =
    "zed.agent_plugins.runtime_observability_regression_watch_rollup.v1";
const AGENT_PLUGIN_PC_USE_PROOF_SUMMARY_SCHEMA: &str = "zed.agent_plugins.pc_use.proof_summary.v1";
const AGENT_BROWSER_FINAL_VALIDATION_RESULT_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_validation_result.v1";
const AGENT_BROWSER_FINAL_VALIDATION_RESULT_IMPORT_RECEIPT_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_validation_result_import_receipt.v1";
const AGENT_BROWSER_FINAL_PROOF_AUDIT_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_proof_audit.v1";
const AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_AUDIT_SUMMARY_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_final_proof_audit_summary.v1";
const AGENT_BROWSER_FINAL_VALIDATION_DIR_NAME: &str = "browser-final-validation";
const AGENT_BROWSER_FINAL_VALIDATION_RESULT_FILE_NAME: &str =
    "latest-agent-browser-final-validation-result.json";
const MANAGED_CHROME_EXECUTION_RECEIPT_PREFIX: &str = "managed-chrome-execution-receipt-";
const MANAGED_CHROME_RUNNER_READY_OUTCOME: &str = "ready_runner_adapter_pending";
const MANAGED_CHROME_EXECUTION_READY_OUTCOME: &str = "completed";
const PC_USE_RUNNER_READY_OUTCOME: &str = "ready_future_executor_pending";

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
    /// Include the compact runtime observability digest for quick Agent Panel status checks.
    pub include_observability_digest: bool,
    /// Include the canonical runtime-green proof path tying digest, handoff, and final proof fields together.
    pub include_runtime_green_proof_path: bool,
    /// Include the compact runtime-green claim gate for quick Agent Panel status checks.
    pub include_runtime_green_claim_gate: bool,
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
            include_observability_digest: true,
            include_runtime_green_proof_path: true,
            include_runtime_green_claim_gate: true,
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
    browser_final_validation_dir: PathBuf,
    browser_final_validation_latest_result: PathBuf,
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
        let browser_final_validation_dir =
            plugin_root.join(AGENT_BROWSER_FINAL_VALIDATION_DIR_NAME);
        let browser_final_validation_latest_result =
            browser_final_validation_dir.join(AGENT_BROWSER_FINAL_VALIDATION_RESULT_FILE_NAME);
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
            browser_final_validation_dir,
            browser_final_validation_latest_result,
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
            &self.browser_final_validation_dir,
            &self.browser_final_validation_latest_result,
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
    let runtime_green_blocker_summary = runtime_green_blocker_summary(status, roots);
    let runtime_green_readiness_scorecard =
        runtime_green_readiness_scorecard(status, &runtime_green_blocker_summary);
    let runtime_observability_digest_value = runtime_observability_digest(
        status,
        roots,
        &runtime_green_blocker_summary,
        &runtime_green_readiness_scorecard,
    );
    let runtime_observability_digest = input
        .include_observability_digest
        .then(|| runtime_observability_digest_value.clone());
    let runtime_green_operator_handoff = runtime_green_operator_handoff(
        runtime_request_root_mode(roots),
        status,
        &runtime_green_blocker_summary,
        &runtime_green_readiness_scorecard,
    );
    let runtime_green_proof_path_value = runtime_green_proof_path(
        status,
        roots,
        &runtime_green_blocker_summary,
        &runtime_green_readiness_scorecard,
        &runtime_observability_digest_value,
        &runtime_green_operator_handoff,
    );
    let runtime_green_proof_path = input
        .include_runtime_green_proof_path
        .then(|| runtime_green_proof_path_value.clone());
    let runtime_green_claim_gate_value = runtime_green_claim_gate(&runtime_green_proof_path_value);
    let runtime_green_claim_readiness_value = runtime_green_claim_readiness(
        &runtime_green_proof_path_value,
        &runtime_green_claim_gate_value,
    );
    let runtime_green_report_gate_value =
        runtime_green_report_gate(&runtime_green_claim_readiness_value);
    let runtime_green_report_badge_value =
        runtime_green_report_badge(&runtime_green_report_gate_value);
    let runtime_green_final_proof_guide_value =
        runtime_green_final_proof_guide(&runtime_green_report_gate_value, roots.root_mode_label());
    let runtime_green_final_proof_guide_summary =
        runtime_green_final_proof_guide_summary(&runtime_green_final_proof_guide_value);
    let runtime_green_final_report_packet_value = runtime_green_final_report_packet(
        &runtime_green_report_gate_value,
        &runtime_green_final_proof_guide_value,
        roots.root_mode_label(),
    );
    let runtime_green_final_proof_audit_value = runtime_green_final_proof_audit(
        &runtime_green_claim_readiness_value,
        &runtime_green_report_gate_value,
        &runtime_green_final_report_packet_value,
    );
    let runtime_green_final_report_packet_summary =
        runtime_green_final_report_packet_summary(&runtime_green_final_report_packet_value);
    let runtime_green_report_readiness_card_value = runtime_green_report_readiness_card(
        &runtime_green_claim_readiness_value,
        &runtime_green_report_gate_value,
        &runtime_green_final_report_packet_value,
        &runtime_green_final_proof_audit_value,
    );
    let runtime_green_final_proof_audit_summary =
        runtime_green_final_proof_audit_summary(&runtime_green_final_proof_audit_value);
    let runtime_green_report_readiness_card_summary =
        runtime_green_report_readiness_card_summary(&runtime_green_report_readiness_card_value);
    let runtime_green_claim_gate = input
        .include_runtime_green_claim_gate
        .then(|| runtime_green_claim_gate_value.clone());
    let runtime_green_claim_gate_summary = runtime_green_claim_gate
        .as_ref()
        .map(runtime_green_claim_gate_summary);

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
        "runtime_green_blocker_summary": runtime_green_blocker_summary,
        "runtime_green_readiness_scorecard": runtime_green_readiness_scorecard,
        "runtime_green_operator_handoff": runtime_green_operator_handoff,
        "runtime_observability_digest": runtime_observability_digest,
        "runtime_green_proof_path": runtime_green_proof_path,
        "runtime_green_claim_gate": runtime_green_claim_gate,
        "runtime_green_claim_gate_summary": runtime_green_claim_gate_summary,
        "runtime_green_claim_readiness": runtime_green_claim_readiness_value,
        "runtime_green_report_gate": runtime_green_report_gate_value,
        "runtime_green_report_badge": runtime_green_report_badge_value,
        "runtime_green_final_proof_guide": runtime_green_final_proof_guide_value,
        "runtime_green_final_proof_guide_summary": runtime_green_final_proof_guide_summary,
        "runtime_green_final_proof_audit": runtime_green_final_proof_audit_value,
        "runtime_green_final_proof_audit_summary": runtime_green_final_proof_audit_summary,
        "runtime_green_final_report_packet": runtime_green_final_report_packet_value,
        "runtime_green_final_report_packet_summary": runtime_green_final_report_packet_summary,
        "runtime_green_report_readiness_card": runtime_green_report_readiness_card_value,
        "runtime_green_report_readiness_card_summary": runtime_green_report_readiness_card_summary,
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
            "final_validation_result": AGENT_BROWSER_FINAL_VALIDATION_RESULT_SCHEMA,
            "final_validation_result_import_receipt": AGENT_BROWSER_FINAL_VALIDATION_RESULT_IMPORT_RECEIPT_SCHEMA,
            "final_proof_audit": AGENT_BROWSER_FINAL_PROOF_AUDIT_SCHEMA,
            "final_proof_audit_summary": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_AUDIT_SUMMARY_SCHEMA,
            "runtime_green_final_proof_guide": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_GUIDE_SCHEMA,
            "runtime_green_final_proof_guide_summary": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_GUIDE_SUMMARY_SCHEMA,
            "runtime_green_final_report_packet": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_REPORT_PACKET_SCHEMA,
            "runtime_green_final_report_packet_summary": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_REPORT_PACKET_SUMMARY_SCHEMA,
            "runtime_green_report_readiness_card": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_READINESS_CARD_SCHEMA,
            "runtime_green_report_readiness_card_summary": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_READINESS_CARD_SUMMARY_SCHEMA,
        },
        "managed_paths": {
            "queue_dir": dir_probe(&roots.browser_queue_dir),
            "latest_payload": file_probe(
                &roots.browser_latest_payload,
                Some(AGENT_BROWSER_PAYLOAD_QUEUE_ITEM_SCHEMA),
                Some("/payload_packet/schema"),
                include_latest_handoff,
            ),
            "final_validation_dir": dir_probe(&roots.browser_final_validation_dir),
            "latest_final_validation_result": file_probe(
                &roots.browser_final_validation_latest_result,
                Some(AGENT_BROWSER_FINAL_VALIDATION_RESULT_SCHEMA),
                None,
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
    let proof_summary = pc_use_proof_summary(roots, current_epoch_millis());
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
        "proof_summary": proof_summary,
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
                    "include_observability_digest": true,
                    "include_runtime_green_proof_path": true,
                    "include_runtime_green_claim_gate": true,
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
                    "include_observability_digest": true,
                    "include_runtime_green_proof_path": true,
                    "include_runtime_green_claim_gate": true,
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
        "browser_final_validation_dir": path_string(&roots.browser_final_validation_dir),
        "browser_final_validation_latest_result": path_string(&roots.browser_final_validation_latest_result),
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
    let browser_final_validation_result = browser_final_validation_result_probe(
        &roots.browser_final_validation_latest_result,
        generated_at_ms,
    );
    let final_result_runtime_green_candidate = browser_final_validation_result
        .pointer("/summary/runtime_green_candidate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let chrome_runner_receipt = outcome_receipt_probe(
        &roots.chrome_latest_runner_receipt,
        Some(AGENT_CHROME_RUNNER_RECEIPT_SCHEMA),
        "/result/outcome",
        &[MANAGED_CHROME_RUNNER_READY_OUTCOME],
        generated_at_ms,
    );
    let chrome_runner_ready = receipt_probe_ready(&chrome_runner_receipt);
    let latest_chrome_execution_receipt = latest_prefixed_outcome_receipt_probe(
        &roots.chrome_execution_dir,
        MANAGED_CHROME_EXECUTION_RECEIPT_PREFIX,
        Some(AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA),
        "/outcome",
        &[MANAGED_CHROME_EXECUTION_READY_OUTCOME],
        generated_at_ms,
    );
    let latest_chrome_execution_receipt_present = latest_chrome_execution_receipt
        .get("latest_file")
        .and_then(Value::as_str)
        .is_some();
    let latest_chrome_execution_ready = receipt_probe_ready(&latest_chrome_execution_receipt);
    let pc_use_runner_receipt = outcome_receipt_probe(
        &roots.pc_use_latest_receipt,
        Some(AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA),
        "/result/outcome",
        &[PC_USE_RUNNER_READY_OUTCOME],
        generated_at_ms,
    );
    let pc_use_runner_ready = receipt_probe_ready(&pc_use_runner_receipt);

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

    if !final_result_runtime_green_candidate {
        blockers.push(serde_json::json!({
            "area": "browser_webpreview",
            "id": "final_validation_result_not_runtime_green",
            "severity": "manual_required",
            "reason": "The managed WebPreview final validation result is missing or does not yet prove runtime_green_candidate=true.",
            "latest_result": browser_final_validation_result.get("summary").cloned().unwrap_or_else(|| serde_json::json!({})),
            "next_actions": [
                "copy_agent_browser_final_validation_bundle",
                "copy_agent_browser_final_validation_result_template",
                "import_agent_browser_final_validation_result_from_clipboard",
                "copy_agent_browser_final_validation_result_import_receipt",
                "send_agent_browser_final_validation_result_to_agent",
                AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME
            ]
        }));
    }

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
    } else if !chrome_runner_ready {
        blockers.push(serde_json::json!({
            "area": "managed_chrome.execution",
            "id": "chrome_runner_gate_receipt_not_ready",
            "severity": "runtime_evidence_missing",
            "reason": "The latest managed Chrome runner-gate receipt exists but does not report the ready outcome required before adapter execution.",
            "classification": chrome_runner_receipt.get("classification").cloned().unwrap_or_else(|| serde_json::json!({})),
            "next_actions": [
                AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                AGENT_CHROME_RUNNER_GATE_TOOL_NAME,
                AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME
            ]
        }));
    }

    if !latest_chrome_execution_receipt_present {
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
    } else if !latest_chrome_execution_ready {
        blockers.push(serde_json::json!({
            "area": "managed_chrome.execution",
            "id": "chrome_execution_receipt_not_completed",
            "severity": "runtime_evidence_missing",
            "reason": "The latest managed Chrome execution receipt exists but its outcome is not a completed safe adapter action.",
            "classification": latest_chrome_execution_receipt.get("classification").cloned().unwrap_or_else(|| serde_json::json!({})),
            "next_actions": [
                AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
                AGENT_CHROME_PAYLOAD_TOOL_NAME,
                AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME,
                AGENT_CHROME_RUNNER_GATE_TOOL_NAME,
                AGENT_CHROME_PLAYWRIGHT_INVOKE_TOOL_NAME
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
    } else if !pc_use_runner_ready {
        blockers.push(serde_json::json!({
            "area": "pc_use",
            "id": "pc_use_runner_receipt_not_ready",
            "severity": "runtime_evidence_missing",
            "reason": "The latest PC-use runner-gate receipt exists but does not report the ready future-executor outcome.",
            "classification": pc_use_runner_receipt.get("classification").cloned().unwrap_or_else(|| serde_json::json!({})),
            "next_actions": [
                AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                AGENT_PC_USE_RUNNER_GATE_TOOL_NAME,
                AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME,
                AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME
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
    } else if final_result_runtime_green_candidate {
        "runtime_green_candidate"
    } else {
        "manual_webpreview_final_result_required"
    };

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_BLOCKERS_SCHEMA,
        "generated_at_ms": generated_at_ms,
        "status": status,
        "runtime_status": runtime_status,
        "runtime_green_candidate": runtime_evidence_blockers == 0 && final_result_runtime_green_candidate,
        "blocker_count": blockers.len(),
        "runtime_evidence_blocker_count": runtime_evidence_blockers,
        "manual_blocker_count": blockers.len().saturating_sub(runtime_evidence_blockers),
        "blockers": blockers,
        "latest_evidence": {
            "browser_final_validation_result": browser_final_validation_result,
            "asset_readiness_summary": asset_summary,
            "managed_chrome_execution_receipt": latest_chrome_execution_receipt,
            "managed_chrome_runner_receipt": chrome_runner_receipt,
            "browser_latest_payload": proof_file_probe(
                &roots.browser_latest_payload,
                Some(AGENT_BROWSER_PAYLOAD_QUEUE_ITEM_SCHEMA),
                Some("/payload_packet/schema"),
                generated_at_ms,
            ),
            "managed_chrome_latest_payload": proof_file_probe(
                &roots.chrome_latest_payload,
                Some(AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA),
                Some("/payload_packet/schema"),
                generated_at_ms,
            ),
            "managed_chrome_adapter_manifest": proof_file_probe(
                &roots.chrome_adapter_manifest,
                Some(AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA),
                None,
                generated_at_ms,
            ),
            "managed_chrome_runner_script": file_probe(
                &roots.chrome_runner_script,
                None,
                None,
                false,
            ),
            "pc_use_latest_payload": proof_file_probe(
                &roots.pc_use_latest_payload,
                Some(AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA),
                Some("/payload_packet/schema"),
                generated_at_ms,
            ),
            "pc_use_latest_runner_receipt": pc_use_runner_receipt,
            "pc_use_proof_summary": pc_use_proof_summary(roots, generated_at_ms),
        },
        "runtime_green_requires": [
            "no critical or runtime_evidence_missing blockers",
            "managed WebPreview final validation result file reports runtime_green_candidate=true",
            "managed Chrome runner receipt outcome == ready_runner_adapter_pending",
            "managed Chrome execution receipt outcome == completed",
            "PC-use runner receipt outcome == ready_future_executor_pending",
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

fn runtime_green_readiness_scorecard(runtime_status: &str, blocker_summary: &Value) -> Value {
    let blockers = blocker_summary
        .get("blockers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let managed_root_blockers = blockers_for_area_exact(&blockers, "managed_roots");
    let browser_blockers = blockers_for_area_exact(&blockers, "browser_webpreview");
    let chrome_blockers = blockers_for_area_prefix(&blockers, "managed_chrome");
    let pc_use_blockers = blockers_for_area_exact(&blockers, "pc_use");

    let browser_final_result_ready = scorecard_bool_at(
        blocker_summary,
        "/latest_evidence/browser_final_validation_result/summary/runtime_green_candidate",
    );
    let browser_payload_ready = scorecard_bool_at(
        blocker_summary,
        "/latest_evidence/browser_latest_payload/exists",
    );
    let chrome_assets_ready = scorecard_bool_at(
        blocker_summary,
        "/latest_evidence/asset_readiness_summary/ready",
    );
    let chrome_payload_ready = scorecard_bool_at(
        blocker_summary,
        "/latest_evidence/managed_chrome_latest_payload/exists",
    );
    let chrome_adapter_manifest_ready = scorecard_bool_at(
        blocker_summary,
        "/latest_evidence/managed_chrome_adapter_manifest/exists",
    );
    let chrome_runner_script_ready = scorecard_bool_at(
        blocker_summary,
        "/latest_evidence/managed_chrome_runner_script/exists",
    );
    let chrome_runner_receipt_ready = scorecard_bool_at(
        blocker_summary,
        "/latest_evidence/managed_chrome_runner_receipt/classification/ready",
    );
    let chrome_execution_receipt_ready = scorecard_bool_at(
        blocker_summary,
        "/latest_evidence/managed_chrome_execution_receipt/classification/ready",
    );
    let pc_use_payload_ready = scorecard_bool_at(
        blocker_summary,
        "/latest_evidence/pc_use_latest_payload/exists",
    );
    let pc_use_runner_receipt_ready = scorecard_bool_at(
        blocker_summary,
        "/latest_evidence/pc_use_latest_runner_receipt/classification/ready",
    );

    let roots_ready = managed_root_blockers.is_empty();
    let browser_ready = roots_ready
        && browser_final_result_ready
        && browser_payload_ready
        && browser_blockers.is_empty();
    let chrome_ready = roots_ready
        && chrome_assets_ready
        && chrome_payload_ready
        && chrome_adapter_manifest_ready
        && chrome_runner_script_ready
        && chrome_runner_receipt_ready
        && chrome_execution_receipt_ready
        && chrome_blockers.is_empty();
    let pc_use_ready = roots_ready
        && pc_use_payload_ready
        && pc_use_runner_receipt_ready
        && pc_use_blockers.is_empty();

    let lanes = vec![
        runtime_green_scorecard_lane(
            "browser_webpreview",
            "Browser/WebPreview",
            browser_ready,
            serde_json::json!({
                "latest_browser_payload_present": browser_payload_ready,
                "final_validation_result_runtime_green": browser_final_result_ready,
            }),
            &browser_blockers,
            "Managed Browser payload plus imported WebPreview final validation result.",
        ),
        runtime_green_scorecard_lane(
            "managed_chrome",
            "Managed Chrome/Playwright",
            chrome_ready,
            serde_json::json!({
                "asset_readiness_ready": chrome_assets_ready,
                "latest_chrome_payload_present": chrome_payload_ready,
                "adapter_manifest_present": chrome_adapter_manifest_ready,
                "runner_script_present": chrome_runner_script_ready,
                "runner_receipt_ready": chrome_runner_receipt_ready,
                "execution_receipt_completed": chrome_execution_receipt_ready,
            }),
            &chrome_blockers,
            "Managed assets, adapter files, queued payload, runner gate, and completed adapter receipt.",
        ),
        runtime_green_scorecard_lane(
            "pc_use",
            "Zed PC-use",
            pc_use_ready,
            serde_json::json!({
                "latest_pc_use_payload_present": pc_use_payload_ready,
                "runner_receipt_ready": pc_use_runner_receipt_ready,
            }),
            &pc_use_blockers,
            "Managed PC-use payload plus future-executor runner-gate receipt.",
        ),
    ];
    let ready_lane_count = lanes
        .iter()
        .filter(|lane| lane.get("ready").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let runtime_green_candidate = blocker_summary
        .get("runtime_green_candidate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let runtime_evidence_blocker_count = blocker_summary
        .get("runtime_evidence_blocker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let manual_blocker_count = blocker_summary
        .get("manual_blocker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let status = if !roots_ready {
        "blocked_unmanaged_paths"
    } else if runtime_green_candidate {
        "runtime_green_candidate"
    } else if ready_lane_count == lanes.len() {
        "ready_for_final_runtime_claim"
    } else if ready_lane_count > 0 {
        "partially_ready"
    } else {
        "blocked_or_pending_evidence"
    };

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_SCORECARD_SCHEMA,
        "status": status,
        "runtime_status": runtime_status,
        "runtime_green_candidate": runtime_green_candidate,
        "summary_status": blocker_summary.get("status").and_then(Value::as_str),
        "generated_from_schema": blocker_summary.get("schema").and_then(Value::as_str),
        "totals": {
            "lane_count": lanes.len(),
            "ready_lane_count": ready_lane_count,
            "blocker_count": blocker_summary.get("blocker_count").and_then(Value::as_u64).unwrap_or(0),
            "runtime_evidence_blocker_count": runtime_evidence_blocker_count,
            "manual_blocker_count": manual_blocker_count,
            "managed_root_blocker_count": managed_root_blockers.len(),
        },
        "managed_root_blockers": runtime_green_lane_blockers(&managed_root_blockers),
        "lanes": lanes,
        "reads_from": [
            "runtime_green_blocker_summary.blockers",
            "runtime_green_blocker_summary.latest_evidence",
            "runtime_green_blocker_summary.runtime_green_candidate"
        ],
        "safety": {
            "scorecard_is_read_only": true,
            "writes_files": false,
            "runs_node": false,
            "launches_browser": false,
            "dispatches_input": false,
            "touches_real_browser_profiles": false
        }
    })
}

fn runtime_green_scorecard_lane(
    id: &'static str,
    label: &'static str,
    ready: bool,
    checks: Value,
    blockers: &[Value],
    ready_requirement: &'static str,
) -> Value {
    serde_json::json!({
        "id": id,
        "label": label,
        "status": runtime_green_lane_status(ready, blockers),
        "ready": ready,
        "ready_requirement": ready_requirement,
        "checks": checks,
        "blocker_count": blockers.len(),
        "blockers": runtime_green_lane_blockers(blockers),
        "primary_next_actions": runtime_green_lane_next_actions(blockers),
    })
}

fn runtime_green_lane_status(ready: bool, blockers: &[Value]) -> &'static str {
    if ready {
        return "ready";
    }

    if blockers.iter().any(|blocker| {
        blocker
            .get("severity")
            .and_then(Value::as_str)
            .is_some_and(|severity| severity == "critical")
    }) {
        "blocked"
    } else if blockers.iter().any(|blocker| {
        blocker
            .get("severity")
            .and_then(Value::as_str)
            .is_some_and(|severity| severity == "runtime_evidence_missing")
    }) {
        "pending_runtime_evidence"
    } else if blockers.iter().any(|blocker| {
        blocker
            .get("severity")
            .and_then(Value::as_str)
            .is_some_and(|severity| severity == "manual_required")
    }) {
        "manual_required"
    } else {
        "pending_runtime_evidence"
    }
}

fn runtime_green_lane_blockers(blockers: &[Value]) -> Vec<Value> {
    blockers
        .iter()
        .map(|blocker| {
            serde_json::json!({
                "area": blocker.get("area").and_then(Value::as_str),
                "id": blocker.get("id").and_then(Value::as_str),
                "severity": blocker.get("severity").and_then(Value::as_str),
                "reason": blocker.get("reason").and_then(Value::as_str),
                "next_actions": blocker
                    .get("next_actions")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
            })
        })
        .collect()
}

fn runtime_green_lane_next_actions(blockers: &[Value]) -> Value {
    blockers
        .iter()
        .find_map(|blocker| blocker.get("next_actions").cloned())
        .unwrap_or_else(|| serde_json::json!([]))
}

fn blockers_for_area_exact(blockers: &[Value], area: &str) -> Vec<Value> {
    blockers
        .iter()
        .filter(|blocker| blocker.get("area").and_then(Value::as_str) == Some(area))
        .cloned()
        .collect()
}

fn blockers_for_area_prefix(blockers: &[Value], area_prefix: &str) -> Vec<Value> {
    blockers
        .iter()
        .filter(|blocker| {
            blocker
                .get("area")
                .and_then(Value::as_str)
                .is_some_and(|area| {
                    area == area_prefix
                        || area
                            .strip_prefix(area_prefix)
                            .is_some_and(|suffix| suffix.starts_with('.'))
                })
        })
        .cloned()
        .collect()
}

fn scorecard_bool_at(value: &Value, pointer: &str) -> bool {
    value
        .pointer(pointer)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn runtime_observability_digest(
    runtime_status: &str,
    roots: &AgentPluginRuntimeRoots,
    blocker_summary: &Value,
    scorecard: &Value,
) -> Value {
    let proof_freshness = observability_proof_freshness(roots);
    let plugin_matrix =
        runtime_observability_plugin_matrix(&proof_freshness, blocker_summary, scorecard);
    let missing_required_files = proof_freshness
        .get("missing_required_files")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let stale_required_files = proof_freshness
        .get("stale_required_files")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let blocker_count = blocker_summary
        .get("blocker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let runtime_evidence_blocker_count = blocker_summary
        .get("runtime_evidence_blocker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let manual_blocker_count = blocker_summary
        .get("manual_blocker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let runtime_green_candidate = blocker_summary
        .get("runtime_green_candidate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let ready_lane_count = scorecard
        .pointer("/totals/ready_lane_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let lane_count = scorecard
        .pointer("/totals/lane_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let status = if runtime_green_candidate {
        "runtime_green_candidate"
    } else if runtime_status != "ready_for_read_only_discovery" {
        "runtime_status_blocked"
    } else if runtime_evidence_blocker_count > 0 || missing_required_files > 0 {
        "runtime_evidence_refresh_required"
    } else if stale_required_files > 0 {
        "stale_runtime_evidence"
    } else if manual_blocker_count > 0 {
        "manual_validation_required"
    } else if lane_count > 0 && ready_lane_count == lane_count {
        "ready_for_final_runtime_validation"
    } else {
        "operator_action_required"
    };

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_OBSERVABILITY_DIGEST_SCHEMA,
        "status": status,
        "runtime_status": runtime_status,
        "runtime_green_candidate": runtime_green_candidate,
        "scorecard_status": scorecard.get("status").and_then(Value::as_str),
        "blocker_summary_status": blocker_summary.get("status").and_then(Value::as_str),
        "totals": {
            "lane_count": lane_count,
            "ready_lane_count": ready_lane_count,
            "blocker_count": blocker_count,
            "runtime_evidence_blocker_count": runtime_evidence_blocker_count,
            "manual_blocker_count": manual_blocker_count,
            "missing_required_file_count": missing_required_files,
            "stale_required_file_count": stale_required_files,
            "regression_watch_lane_count": plugin_matrix
                .pointer("/regression_watch_rollup/watched_plugin_count")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            "regression_watch_must_recheck_count": plugin_matrix
                .pointer("/regression_watch_rollup/total_must_recheck_count")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            "regression_watch_must_not_regress_count": plugin_matrix
                .pointer("/regression_watch_rollup/total_must_not_regress_count")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        },
        "freshness": {
            "status": proof_freshness.get("status").and_then(Value::as_str),
            "freshness_window_ms": proof_freshness.get("freshness_window_ms").and_then(Value::as_u64),
            "missing_required_files": proof_freshness
                .get("missing_required_files")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
            "stale_required_files": proof_freshness
                .get("stale_required_files")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
            "recovery_actions": proof_freshness
                .get("recovery_actions")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
        },
        "lane_statuses": scorecard
            .get("lanes")
            .and_then(Value::as_array)
            .map(|lanes| lanes.iter().map(runtime_observability_digest_lane).collect::<Vec<_>>())
            .unwrap_or_default(),
        "plugin_matrix": plugin_matrix,
        "next_operator_packet": {
            "schema": AGENT_PLUGIN_RUNTIME_GREEN_OPERATOR_HANDOFF_SCHEMA,
            "field": "runtime_green_operator_handoff",
            "tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
            "payload": runtime_green_status_inspect_payload(runtime_request_root_mode(roots))
        },
        "reads_from": [
            "runtime_green_blocker_summary",
            "runtime_green_readiness_scorecard",
            "observability_proof_freshness",
            "runtime_observability_plugin_matrix"
        ],
        "safety": {
            "digest_is_read_only": true,
            "writes_files": false,
            "runs_node": false,
            "launches_browser": false,
            "dispatches_input": false,
            "touches_real_browser_profiles": false
        }
    })
}

fn runtime_observability_digest_lane(lane: &Value) -> Value {
    serde_json::json!({
        "id": lane.get("id").and_then(Value::as_str),
        "status": lane.get("status").and_then(Value::as_str),
        "ready": lane.get("ready").and_then(Value::as_bool).unwrap_or(false),
        "blocker_count": lane.get("blocker_count").and_then(Value::as_u64).unwrap_or(0),
        "primary_next_actions": lane
            .get("primary_next_actions")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
    })
}

fn runtime_observability_plugin_matrix(
    proof_freshness: &Value,
    blocker_summary: &Value,
    scorecard: &Value,
) -> Value {
    let rows = scorecard
        .get("lanes")
        .and_then(Value::as_array)
        .map(|lanes| {
            lanes
                .iter()
                .map(|lane| {
                    runtime_observability_plugin_matrix_row(lane, proof_freshness, blocker_summary)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let plugin_count = rows.len();
    let first_priority = rows
        .iter()
        .find(|row| !row.get("ready").and_then(Value::as_bool).unwrap_or(false))
        .and_then(|row| row.get("next_action"))
        .cloned();
    let ready_plugin_count = rows
        .iter()
        .filter(|row| row.get("ready").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let pending_plugin_count = plugin_count.saturating_sub(ready_plugin_count);
    let regression_watch_rollup = runtime_observability_plugin_regression_watch_rollup(&rows);
    let status = if plugin_count == 0 {
        "empty"
    } else if pending_plugin_count == 0 {
        "all_plugins_have_required_runtime_evidence"
    } else {
        "plugin_runtime_evidence_pending"
    };

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_OBSERVABILITY_MATRIX_SCHEMA,
        "status": status,
        "plugin_count": plugin_count,
        "ready_plugin_count": ready_plugin_count,
        "pending_plugin_count": pending_plugin_count,
        "first_priority": first_priority,
        "regression_watch_rollup": regression_watch_rollup,
        "rows": rows,
        "reads_from": [
            "runtime_green_readiness_scorecard.lanes",
            "runtime_green_blocker_summary.latest_evidence",
            "observability_proof_freshness.required_files",
            "observability_proof_freshness.receipt_classifications",
            "observability_proof_freshness.recovery_actions",
            "runtime_observability_plugin_regression_watch",
            "runtime_observability_plugin_regression_watch_rollup"
        ],
        "safety": {
            "matrix_is_read_only": true,
            "writes_files": false,
            "runs_node": false,
            "launches_browser": false,
            "dispatches_input": false,
            "touches_real_browser_profiles": false,
        }
    })
}

fn runtime_observability_plugin_regression_watch_rollup(rows: &[Value]) -> Value {
    let watch_rows = rows
        .iter()
        .filter_map(|row| {
            let watch = row.get("regression_watch")?;
            let status = watch
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown_plugin_lane");
            if status == "unknown_plugin_lane" {
                return None;
            }

            Some(serde_json::json!({
                "lane_id": row.get("lane_id").and_then(Value::as_str).unwrap_or("unknown"),
                "plugin_id": row.get("plugin_id").and_then(Value::as_str).unwrap_or("unknown"),
                "ready": row.get("ready").and_then(Value::as_bool).unwrap_or(false),
                "status": status,
                "focus": watch.get("focus").cloned().unwrap_or(Value::Null),
                "first_must_recheck": watch
                    .get("must_recheck")
                    .and_then(Value::as_array)
                    .and_then(|items| items.first())
                    .cloned()
                    .unwrap_or(Value::Null),
                "first_must_not_regress": watch
                    .get("must_not_regress")
                    .and_then(Value::as_array)
                    .and_then(|items| items.first())
                    .cloned()
                    .unwrap_or(Value::Null),
                "proof_source_count": watch
                    .get("proof_sources")
                    .and_then(Value::as_array)
                    .map(|items| items.len())
                    .unwrap_or(0),
                "after_action_validation": watch
                    .get("after_action_validation")
                    .cloned()
                    .unwrap_or(Value::Null),
            }))
        })
        .collect::<Vec<_>>();
    let total_must_recheck_count = rows
        .iter()
        .map(|row| {
            row.pointer("/regression_watch/must_recheck")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .unwrap_or(0)
        })
        .sum::<usize>();
    let total_must_not_regress_count = rows
        .iter()
        .map(|row| {
            row.pointer("/regression_watch/must_not_regress")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .unwrap_or(0)
        })
        .sum::<usize>();
    let total_proof_source_count = rows
        .iter()
        .map(|row| {
            row.pointer("/regression_watch/proof_sources")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .unwrap_or(0)
        })
        .sum::<usize>();

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_OBSERVABILITY_WATCH_ROLLUP_SCHEMA,
        "status": if watch_rows.is_empty() {
            "no_regression_watch_lanes"
        } else {
            "regression_watch_required"
        },
        "watched_plugin_count": watch_rows.len(),
        "total_must_recheck_count": total_must_recheck_count,
        "total_must_not_regress_count": total_must_not_regress_count,
        "total_proof_source_count": total_proof_source_count,
        "first_watch": watch_rows.first().cloned(),
        "watch_rows": watch_rows,
        "safety": {
            "rollup_is_read_only": true,
            "writes_files": false,
            "runs_node": false,
            "launches_browser": false,
            "dispatches_input": false,
            "touches_real_browser_profiles": false,
        }
    })
}

fn runtime_observability_plugin_matrix_row(
    lane: &Value,
    proof_freshness: &Value,
    blocker_summary: &Value,
) -> Value {
    let lane_id = lane.get("id").and_then(Value::as_str).unwrap_or("unknown");
    let ready = lane.get("ready").and_then(Value::as_bool).unwrap_or(false);
    let proof = runtime_observability_plugin_proof(lane_id, proof_freshness, blocker_summary);
    let freshness = runtime_observability_plugin_freshness(lane_id, proof_freshness);
    let next_action = runtime_observability_plugin_next_action(lane_id, ready, lane, &freshness);
    let handoff = runtime_observability_plugin_handoff(lane_id);

    serde_json::json!({
        "lane_id": lane_id,
        "plugin_id": runtime_observability_plugin_id(lane_id),
        "label": lane.get("label").and_then(Value::as_str),
        "status": lane.get("status").and_then(Value::as_str),
        "ready": ready,
        "code_score": runtime_observability_plugin_code_score(lane_id),
        "ready_requirement": lane.get("ready_requirement").and_then(Value::as_str),
        "blocker_count": lane.get("blocker_count").and_then(Value::as_u64).unwrap_or(0),
        "first_blocker": lane
            .get("blockers")
            .and_then(Value::as_array)
            .and_then(|blockers| blockers.first())
            .cloned(),
        "primary_next_actions": lane
            .get("primary_next_actions")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "proof": proof,
        "evidence_freshness": freshness,
        "next_action": next_action,
        "handoff": handoff,
        "regression_watch": runtime_observability_plugin_regression_watch(lane_id),
        "watch_surfaces": runtime_observability_plugin_watch_surfaces(lane_id),
        "safety": {
            "read_only_summary": true,
            "writes_files": false,
            "runs_node": false,
            "launches_browser": false,
            "dispatches_input": false,
            "touches_real_browser_profiles": false,
        }
    })
}

fn runtime_observability_plugin_proof(
    lane_id: &str,
    proof_freshness: &Value,
    blocker_summary: &Value,
) -> Value {
    match lane_id {
        "browser_webpreview" => serde_json::json!({
            "status": blocker_summary
                .pointer("/latest_evidence/browser_final_validation_result/summary/state")
                .and_then(Value::as_str)
                .unwrap_or("missing_result"),
            "required_field": "runtime_green_blocker_summary.latest_evidence.browser_final_validation_result.summary.runtime_green_candidate",
            "runtime_green_candidate": blocker_summary
                .pointer("/latest_evidence/browser_final_validation_result/summary/runtime_green_candidate")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "latest_payload": proof_freshness.pointer("/required_files/browser_latest_payload").cloned(),
            "final_validation_result": blocker_summary.pointer("/latest_evidence/browser_final_validation_result/summary").cloned(),
        }),
        "managed_chrome" => serde_json::json!({
            "status": proof_freshness
                .pointer("/receipt_classifications/managed_chrome_execution_receipt/classification/state")
                .and_then(Value::as_str)
                .unwrap_or("missing_receipt"),
            "required_field": "runtime_green_blocker_summary.latest_evidence.managed_chrome_execution_receipt.classification.outcome == completed",
            "runner_receipt_ready": proof_freshness
                .pointer("/receipt_classifications/managed_chrome_runner_receipt/classification/ready")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "execution_receipt_ready": proof_freshness
                .pointer("/receipt_classifications/managed_chrome_execution_receipt/classification/ready")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "asset_readiness": proof_freshness.pointer("/required_files/chrome_asset_readiness_summary").cloned(),
            "latest_execution": proof_freshness.pointer("/latest_optional_execution_files/chrome_execution").cloned(),
        }),
        "pc_use" => serde_json::json!({
            "status": blocker_summary
                .pointer("/latest_evidence/pc_use_proof_summary/status")
                .and_then(Value::as_str)
                .unwrap_or("payload_missing"),
            "required_field": "runtime_green_blocker_summary.latest_evidence.pc_use_latest_runner_receipt.classification.outcome == ready_future_executor_pending",
            "payload_ready": blocker_summary
                .pointer("/latest_evidence/pc_use_proof_summary/payload_ready")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "runner_receipt_ready": blocker_summary
                .pointer("/latest_evidence/pc_use_proof_summary/runner_receipt_ready")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "target_snapshot_id_present": blocker_summary
                .pointer("/latest_evidence/pc_use_proof_summary/payload/target_snapshot_id_present")
                .and_then(Value::as_bool),
            "proof_summary": blocker_summary.pointer("/latest_evidence/pc_use_proof_summary").cloned(),
        }),
        _ => serde_json::json!({
            "status": "unknown_plugin_lane",
            "required_field": Value::Null,
        }),
    }
}

fn runtime_observability_plugin_freshness(lane_id: &str, proof_freshness: &Value) -> Value {
    let missing = runtime_observability_plugin_filtered_labels(
        lane_id,
        proof_freshness.get("missing_required_files"),
    );
    let stale = runtime_observability_plugin_filtered_labels(
        lane_id,
        proof_freshness.get("stale_required_files"),
    );
    let recovery_actions = runtime_observability_plugin_filtered_recovery_actions(
        lane_id,
        proof_freshness.pointer("/recovery_actions/actions"),
    );
    let first_refresh_target = missing.first().cloned().or_else(|| stale.first().cloned());
    let status = if !missing.is_empty() {
        "missing_runtime_evidence"
    } else if !stale.is_empty() {
        "stale_runtime_evidence"
    } else {
        "managed_evidence_current"
    };

    serde_json::json!({
        "status": status,
        "freshness_window_ms": proof_freshness.get("freshness_window_ms").and_then(Value::as_u64),
        "missing_required_files": missing,
        "stale_required_files": stale,
        "first_refresh_target": first_refresh_target,
        "recovery_actions": {
            "status": if recovery_actions.is_empty() { "no_refresh_action_required" } else { "refresh_action_required" },
            "actions": recovery_actions,
        },
        "read_only": true,
    })
}

fn runtime_observability_plugin_filtered_labels(
    lane_id: &str,
    labels: Option<&Value>,
) -> Vec<String> {
    labels
        .and_then(Value::as_array)
        .map(|labels| {
            labels
                .iter()
                .filter_map(Value::as_str)
                .filter(|label| runtime_observability_plugin_label_matches(lane_id, label))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn runtime_observability_plugin_filtered_recovery_actions(
    lane_id: &str,
    actions: Option<&Value>,
) -> Vec<Value> {
    actions
        .and_then(Value::as_array)
        .map(|actions| {
            actions
                .iter()
                .filter(|action| {
                    action
                        .get("target")
                        .and_then(Value::as_str)
                        .is_some_and(|target| {
                            runtime_observability_plugin_label_matches(lane_id, target)
                        })
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn runtime_observability_plugin_label_matches(lane_id: &str, label: &str) -> bool {
    match lane_id {
        "browser_webpreview" => label.starts_with("browser."),
        "managed_chrome" => label.starts_with("chrome."),
        "pc_use" => label.starts_with("pc_use."),
        _ => false,
    }
}

fn runtime_observability_plugin_next_action(
    lane_id: &str,
    ready: bool,
    lane: &Value,
    freshness: &Value,
) -> Value {
    let freshness_status = freshness
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let recovery_action = freshness
        .pointer("/recovery_actions/actions/0")
        .cloned()
        .unwrap_or(Value::Null);
    let recovery_step = recovery_action
        .pointer("/steps/0")
        .and_then(Value::as_str)
        .map(str::to_string);
    let lane_action = lane
        .pointer("/primary_next_actions/0")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| runtime_observability_plugin_default_next_action(lane_id).map(str::to_string));
    let uses_recovery_action = recovery_step.is_some();
    let action = recovery_step.or(lane_action);
    let writes_files = recovery_action
        .get("writes_files")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let status = if ready {
        "no_action_required"
    } else if freshness_status == "missing_runtime_evidence" {
        "refresh_missing_evidence"
    } else if freshness_status == "stale_runtime_evidence" {
        "refresh_stale_evidence"
    } else {
        "complete_required_proof"
    };

    serde_json::json!({
        "status": status,
        "action": action,
        "source": if uses_recovery_action { "observability_proof_freshness.recovery_actions" } else { "runtime_green_readiness_scorecard.primary_next_actions" },
        "target": freshness.get("first_refresh_target").and_then(Value::as_str),
        "lane_id": lane_id,
        "dispatches_input": false,
        "writes_files": writes_files,
        "recovery_action": recovery_action,
    })
}

fn runtime_observability_plugin_default_next_action(lane_id: &str) -> Option<&'static str> {
    match lane_id {
        "browser_webpreview" => Some("copy_agent_browser_final_validation_bundle"),
        "managed_chrome" => Some(AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME),
        "pc_use" => Some(AGENT_PC_USE_RUNNER_GATE_TOOL_NAME),
        _ => None,
    }
}

fn runtime_observability_plugin_id(lane_id: &str) -> &'static str {
    match lane_id {
        "browser_webpreview" => "zed.browser",
        "managed_chrome" => "zed.chrome",
        "pc_use" => "zed.pc_use",
        _ => "unknown",
    }
}

fn runtime_observability_plugin_code_score(lane_id: &str) -> u64 {
    match lane_id {
        "browser_webpreview" => 99,
        "managed_chrome" => 94,
        "pc_use" => 91,
        _ => 0,
    }
}

fn runtime_observability_plugin_handoff(lane_id: &str) -> Value {
    match lane_id {
        "browser_webpreview" => serde_json::json!({
            "copy_action": "copy_agent_browser_executor_validation_progress",
            "send_action": "send_agent_browser_executor_validation_progress_to_agent",
            "final_bundle_action": "copy_agent_browser_final_validation_bundle",
            "final_result_template_action": "copy_agent_browser_final_validation_result_template",
        }),
        "managed_chrome" => serde_json::json!({
            "queue_inspection_tool": AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
            "runner_gate_tool": AGENT_CHROME_RUNNER_GATE_TOOL_NAME,
            "execution_inspect_tool": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
            "webpreview_status_copy": "copy_managed_chrome_execution_status",
            "webpreview_status_send": "send_managed_chrome_execution_status_to_agent",
        }),
        "pc_use" => serde_json::json!({
            "payload_queue_inspect_tool": AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
            "runner_receipts_tool": AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME,
            "runtime_status_proof_summary_field": "plugins.pc_use.proof_summary",
            "webpreview_status_copy": "copy_pc_use_status",
            "webpreview_status_send": "send_pc_use_status_to_agent",
        }),
        _ => serde_json::json!({}),
    }
}

fn runtime_observability_plugin_regression_watch(lane_id: &str) -> Value {
    match lane_id {
        "browser_webpreview" => serde_json::json!({
            "status": "manual_runtime_watch_required",
            "focus": "Protect editor typing, caret position, WebPreview focus, and Browser executor receipt discipline.",
            "must_recheck": [
                "Editor typing remains responsive before and after WebPreview Browser actions.",
                "WebPreview focus returns to the expected surface after navigation, reload, tab switch, and URL edit.",
                "Native click, type, key, scroll, history, and cache receipts remain fresh before any final runtime-green claim.",
                "Final validation result import keeps durable evidence non-empty for every required manual check."
            ],
            "must_not_regress": [
                "No browser action should route keyboard input to the editor or stale WebPreview item unexpectedly.",
                "No read-only handoff should dispatch click, type, key, scroll, screenshot, or process-launch behavior.",
                "No runtime-green report should be possible without the imported final validation result."
            ],
            "proof_sources": [
                "agent_browser_executor_validation_progress",
                "agent_browser_native_dispatch_receipt_matrix",
                "agent_browser_final_validation_result_import_receipt",
                "runtime_green_claim_gate"
            ],
            "after_action_validation": "Refresh the runtime observability digest, then copy the final proof audit before reporting status.",
            "read_only": true,
        }),
        "managed_chrome" => serde_json::json!({
            "status": "managed_profile_watch_required",
            "focus": "Keep managed Chrome/Playwright proof isolated to managed roots and safe read-only action receipts.",
            "must_recheck": [
                "Managed roots stay under workspace or Zed-data plugin directories.",
                "Real Chrome, Edge, and Firefox profiles are not read or written.",
                "Adapter receipts still show only allowlisted safe actions: open_url, screenshot, inspect_element, dom_snapshot, runtime_events, set_viewport, or wait_for_selector.",
                "Click, type, key, and scroll remain blocked in the managed Chrome adapter."
            ],
            "must_not_regress": [
                "No Playwright execution should launch against a real user profile.",
                "No managed Chrome receipt should hide failed requests, timeout status, or action outcome.",
                "No catalog should advertise blocked input actions as executable."
            ],
            "proof_sources": [
                "agent-plugin-asset-provisioning.json",
                "managed_chrome_runner_receipt",
                "managed_chrome_execution_receipt",
                "managed_chrome_execution_status"
            ],
            "after_action_validation": "Inspect managed Chrome executions and re-read runtime status before advancing the runtime-green claim gate.",
            "read_only": true,
        }),
        "pc_use" => serde_json::json!({
            "status": "future_executor_watch_required",
            "focus": "Keep PC-use proof auditable while OS-wide control, screenshots, focus changes, and input dispatch stay disabled.",
            "must_recheck": [
                "Every future input-ready payload references a matching UI snapshot target id and snapshot receipt id.",
                "Runner receipts stay inspectable before any future executor exists.",
                "Context, target manifest, and UI snapshot tools remain read-only.",
                "PC-use status keeps explicit no-screenshot, no-focus, no-input, no-process, and no-OS-control flags."
            ],
            "must_not_regress": [
                "No PC-use path should take screenshots, focus Zed, click, type, launch processes, or control the OS.",
                "No guessed target id should pass as input-ready without a matching snapshot receipt.",
                "No future-executor receipt should be treated as real execution proof."
            ],
            "proof_sources": [
                "inspect_zed_window_context",
                "inspect_zed_pc_use_targets",
                "inspect_zed_pc_use_ui_snapshot",
                "inspect_zed_pc_use_runner_receipts"
            ],
            "after_action_validation": "Re-read PC-use proof summary and runtime claim gate before reporting PC-use readiness.",
            "read_only": true,
        }),
        _ => serde_json::json!({
            "status": "unknown_plugin_lane",
            "focus": Value::Null,
            "must_recheck": [],
            "must_not_regress": [],
            "proof_sources": [],
            "after_action_validation": Value::Null,
            "read_only": true,
        }),
    }
}

fn runtime_observability_plugin_watch_surfaces(lane_id: &str) -> Vec<&'static str> {
    match lane_id {
        "browser_webpreview" => vec![
            "editor caret and typing latency",
            "WebPreview focus after navigation or reload",
            "native click/type/key/scroll/history/cache receipts",
            "manual final validation result evidence",
        ],
        "managed_chrome" => vec![
            "managed workspace or Zed-data roots only",
            "real Chrome, Edge, and Firefox profiles stay untouched",
            "safe Playwright action execution receipts",
            "click, type, key, and scroll stay blocked in the managed adapter",
        ],
        "pc_use" => vec![
            "future UI snapshot target ids require matching snapshot receipt ids",
            "runner receipts stay auditable before any future executor exists",
            "no focus, click, type, screenshot, or process launch in the current gate",
            "OS-wide desktop control stays blocked by default",
        ],
        _ => Vec::new(),
    }
}

fn runtime_green_proof_path(
    runtime_status: &str,
    roots: &AgentPluginRuntimeRoots,
    blocker_summary: &Value,
    scorecard: &Value,
    digest: &Value,
    operator_handoff: &Value,
) -> Value {
    let root_mode = runtime_request_root_mode(roots);
    let runtime_green_candidate = blocker_summary
        .get("runtime_green_candidate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let runtime_evidence_blocker_count = blocker_summary
        .get("runtime_evidence_blocker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let manual_blocker_count = blocker_summary
        .get("manual_blocker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let missing_required_file_count = digest
        .pointer("/totals/missing_required_file_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let stale_required_file_count = digest
        .pointer("/totals/stale_required_file_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let regression_watch_rollup = digest
        .pointer("/plugin_matrix/regression_watch_rollup")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let ready_lane_count = scorecard
        .pointer("/totals/ready_lane_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let lane_count = scorecard
        .pointer("/totals/lane_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let browser_final_validation_candidate = blocker_summary
        .pointer("/latest_evidence/browser_final_validation_result/summary/runtime_green_candidate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let current_best_next = operator_handoff
        .get("current_best_next")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let current_best_next_lane = current_best_next
        .get("lane_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let current_best_next_label = current_best_next
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or(&current_best_next_lane)
        .to_string();
    let current_best_next_status = current_best_next
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let status = if runtime_green_candidate {
        "runtime_green_candidate"
    } else if runtime_status != "ready_for_read_only_discovery" {
        "runtime_status_blocked"
    } else if runtime_evidence_blocker_count > 0 || missing_required_file_count > 0 {
        "runtime_evidence_required"
    } else if stale_required_file_count > 0 {
        "fresh_runtime_evidence_required"
    } else if !browser_final_validation_candidate || manual_blocker_count > 0 {
        "manual_final_result_required"
    } else if lane_count > 0 && ready_lane_count == lane_count {
        "ready_for_final_runtime_validation"
    } else {
        "operator_action_required"
    };
    let claim_gate_status = if runtime_green_candidate {
        "ready"
    } else if runtime_evidence_blocker_count > 0 || missing_required_file_count > 0 {
        "runtime_evidence_required"
    } else if stale_required_file_count > 0 {
        "fresh_runtime_evidence_required"
    } else if !browser_final_validation_candidate || manual_blocker_count > 0 {
        "manual_final_result_required"
    } else if lane_count > 0 && ready_lane_count < lane_count {
        "pending_runtime_lane"
    } else {
        "manual_review_required"
    };

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_PROOF_PATH_SCHEMA,
        "status": status,
        "runtime_status": runtime_status,
        "root_mode": root_mode,
        "runtime_green_candidate": runtime_green_candidate,
        "current": {
            "digest_status": digest.get("status").and_then(Value::as_str),
            "operator_handoff_status": operator_handoff.get("status").and_then(Value::as_str),
            "scorecard_status": scorecard.get("status").and_then(Value::as_str),
            "blocker_summary_status": blocker_summary.get("status").and_then(Value::as_str),
            "current_best_next": current_best_next,
            "regression_watch_rollup": regression_watch_rollup.clone(),
            "final_validation_result": blocker_summary
                .pointer("/latest_evidence/browser_final_validation_result/summary")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
        },
        "claim_gate": {
            "ready": runtime_green_candidate,
            "ready_lane_count": ready_lane_count,
            "lane_count": lane_count,
            "runtime_evidence_blocker_count": runtime_evidence_blocker_count,
            "manual_blocker_count": manual_blocker_count,
            "missing_required_file_count": missing_required_file_count,
            "stale_required_file_count": stale_required_file_count,
            "browser_final_validation_runtime_green": browser_final_validation_candidate,
            "requires_final_windows_just_run": true,
            "claim_only_when": [
                "runtime_green_candidate == true",
                "ready_lane_count == lane_count",
                "runtime_evidence_blocker_count == 0",
                "manual_blocker_count == 0",
                "missing_required_file_count == 0",
                "stale_required_file_count == 0",
                "browser_final_validation_runtime_green == true"
            ]
        },
        "operator_summary": {
            "status": claim_gate_status,
            "ready_lane_fraction": format!("{ready_lane_count}/{lane_count}"),
            "runtime_evidence_blocker_count": runtime_evidence_blocker_count,
            "manual_blocker_count": manual_blocker_count,
            "missing_required_file_count": missing_required_file_count,
            "stale_required_file_count": stale_required_file_count,
            "regression_watch_status": regression_watch_rollup.get("status").and_then(Value::as_str),
            "regression_watch_lane_count": regression_watch_rollup
                .get("watched_plugin_count")
                .and_then(Value::as_u64),
            "first_regression_watch_status": regression_watch_rollup
                .pointer("/first_watch/status")
                .and_then(Value::as_str),
            "first_pending_lane_id": current_best_next_lane,
            "first_pending_lane_label": current_best_next_label,
            "first_pending_lane_status": current_best_next_status,
            "browser_final_validation_runtime_green": browser_final_validation_candidate,
            "can_claim_runtime_green": runtime_green_candidate,
            "next_operator_step": if runtime_green_candidate {
                "Run the final manual Windows runtime proof before reporting runtime-green."
            } else {
                "Resolve the first pending evidence lane, then re-read runtime_green_proof_path."
            }
        },
        "proof_files": {
            "browser_final_validation_result": path_string(&roots.browser_final_validation_latest_result),
            "browser_latest_payload": path_string(&roots.browser_latest_payload),
            "managed_chrome_latest_payload": path_string(&roots.chrome_latest_payload),
            "managed_chrome_runner_receipt": path_string(&roots.chrome_latest_runner_receipt),
            "managed_chrome_execution_dir": path_string(&roots.chrome_execution_dir),
            "pc_use_latest_payload": path_string(&roots.pc_use_latest_payload),
            "pc_use_runner_receipt": path_string(&roots.pc_use_latest_receipt),
        },
        "agent_read_sequence": [
            {
                "step": "inspect_runtime_status",
                "tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                "payload": runtime_green_status_inspect_payload(root_mode),
                "reads_fields": [
                    "runtime_green_proof_path",
                    "runtime_observability_digest",
                    "runtime_observability_digest.plugin_matrix",
                    "runtime_observability_digest.plugin_matrix.regression_watch_rollup",
                    "runtime_green_operator_handoff",
                    "runtime_green_report_gate",
                    "runtime_green_report_badge",
                    "runtime_green_final_proof_guide",
                    "runtime_green_final_proof_guide_summary",
                    "runtime_green_final_proof_audit",
                    "runtime_green_final_proof_audit_summary",
                    "runtime_green_final_report_packet",
                    "runtime_green_final_report_packet_summary",
                    "runtime_green_report_readiness_card",
                    "runtime_green_report_readiness_card_summary"
                ],
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "step": "read_digest",
                "field": "runtime_observability_digest",
                "purpose": "Summarize lane health, plugin matrix proof state, regression-watch pressure, stale/missing proof evidence, and immediate recovery actions.",
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "step": "follow_operator_handoff",
                "field": "runtime_green_operator_handoff.current_best_next",
                "purpose": "Choose the next Browser/WebPreview, managed Chrome, PC-use, or final validation action.",
                "writes_files": "only when a permissioned tool explicitly writes managed receipts",
                "dispatches_input": false
            },
            {
                "step": "import_final_result_when_available",
                "webpreview_actions": [
                    "copy_agent_browser_final_validation_result_template",
                    "import_agent_browser_final_validation_result_from_clipboard",
                    "copy_agent_browser_final_validation_result",
                    "send_agent_browser_final_validation_result_to_agent"
                ],
                "purpose": "Record the manual Windows runtime proof before any runtime-green claim.",
                "managed_proof_write": "only after explicit WebPreview import action",
                "dispatches_input": false
            }
        ],
        "webpreview_packets": {
            "runtime_green_proof_path": {
                "schema": AGENT_PLUGIN_RUNTIME_GREEN_PROOF_PATH_SCHEMA,
                "copy_action": "copy_agent_plugin_runtime_green_proof_path",
                "send_action": "send_agent_plugin_runtime_green_proof_path_to_agent"
            },
            "runtime_green_claim_gate": {
                "schema": AGENT_PLUGIN_RUNTIME_GREEN_CLAIM_GATE_SCHEMA,
                "copy_action": "copy_agent_plugin_runtime_green_claim_gate",
                "send_action": "send_agent_plugin_runtime_green_claim_gate_to_agent"
            },
            "runtime_green_claim_readiness": {
                "schema": AGENT_PLUGIN_RUNTIME_GREEN_CLAIM_READINESS_SCHEMA,
                "copy_action": "copy_agent_plugin_runtime_green_claim_readiness",
                "send_action": "send_agent_plugin_runtime_green_claim_readiness_to_agent"
            },
            "runtime_green_report_gate": {
                "schema": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_GATE_SCHEMA,
                "badge_schema": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_BADGE_SCHEMA,
                "copy_action": "copy_agent_plugin_runtime_green_report_gate",
                "send_action": "send_agent_plugin_runtime_green_report_gate_to_agent"
            },
            "runtime_green_report_badge": {
                "schema": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_BADGE_SCHEMA,
                "source": "runtime_green_report_gate.badge",
                "copy_action": "copy_agent_plugin_runtime_green_report_gate",
                "send_action": "send_agent_plugin_runtime_green_report_gate_to_agent"
            },
            "runtime_green_final_proof_guide": {
                "schema": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_GUIDE_SCHEMA,
                "summary_schema": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_GUIDE_SUMMARY_SCHEMA,
                "runtime_status_summary_field": "runtime_green_final_proof_guide_summary",
                "source": "runtime_green_report_gate",
                "copy_action": "copy_agent_plugin_runtime_green_final_proof_guide",
                "send_action": "send_agent_plugin_runtime_green_final_proof_guide_to_agent"
            },
            "runtime_green_final_proof_audit": {
                "schema": AGENT_BROWSER_FINAL_PROOF_AUDIT_SCHEMA,
                "source": "runtime_green_claim_readiness + runtime_green_report_gate",
                "copy_action": "copy_agent_browser_final_proof_audit",
                "send_action": "send_agent_browser_final_proof_audit_to_agent"
            },
            "runtime_green_final_report_packet": {
                "schema": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_REPORT_PACKET_SCHEMA,
                "summary_schema": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_REPORT_PACKET_SUMMARY_SCHEMA,
                "runtime_status_summary_field": "runtime_green_final_report_packet_summary",
                "source": "runtime_green_report_gate + final_validation_result_import_receipt",
                "copy_action": "copy_agent_plugin_runtime_green_final_report_packet",
                "send_action": "send_agent_plugin_runtime_green_final_report_packet_to_agent"
            },
            "runtime_green_report_readiness_card": {
                "schema": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_READINESS_CARD_SCHEMA,
                "summary_schema": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_READINESS_CARD_SUMMARY_SCHEMA,
                "runtime_status_summary_field": "runtime_green_report_readiness_card_summary",
                "source": "runtime_green_claim_readiness + runtime_green_report_gate + runtime_green_final_report_packet + runtime_green_final_proof_audit",
                "copy_action": "copy_agent_plugin_runtime_green_report_readiness_card",
                "send_action": "send_agent_plugin_runtime_green_report_readiness_card_to_agent"
            },
            "runtime_observability_digest": {
                "schema": AGENT_PLUGIN_RUNTIME_OBSERVABILITY_DIGEST_SCHEMA,
                "plugin_matrix_schema": AGENT_PLUGIN_RUNTIME_OBSERVABILITY_MATRIX_SCHEMA,
                "regression_watch_rollup_schema": AGENT_PLUGIN_RUNTIME_OBSERVABILITY_WATCH_ROLLUP_SCHEMA,
                "copy_action": "copy_agent_plugin_runtime_observability_digest",
                "send_action": "send_agent_plugin_runtime_observability_digest_to_agent"
            },
            "runtime_green_operator_handoff": {
                "schema": AGENT_PLUGIN_RUNTIME_GREEN_OPERATOR_HANDOFF_SCHEMA,
                "copy_action": "copy_agent_plugin_runtime_green_handoff",
                "send_action": "send_agent_plugin_runtime_green_handoff_to_agent"
            }
        },
        "reads_from": [
            "runtime_observability_digest",
            "runtime_green_operator_handoff",
            "runtime_green_blocker_summary",
            "runtime_green_readiness_scorecard",
            "runtime_green_final_proof_guide_summary",
            "runtime_green_final_proof_audit",
            "runtime_green_final_proof_audit_summary",
            "runtime_green_final_report_packet",
            "runtime_green_final_report_packet_summary",
            "runtime_green_report_readiness_card",
            "runtime_green_report_readiness_card_summary"
        ],
        "safety": {
            "proof_path_is_read_only": true,
            "writes_files": false,
            "runs_node": false,
            "launches_browser": false,
            "dispatches_input": false,
            "touches_real_browser_profiles": false
        }
    })
}

fn runtime_green_claim_gate(proof_path: &Value) -> Value {
    let claim_gate = proof_path
        .get("claim_gate")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let operator_summary = proof_path
        .get("operator_summary")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let status = operator_summary
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let root_mode = proof_path
        .get("root_mode")
        .and_then(Value::as_str)
        .unwrap_or("workspace");
    let ready_lane_count = claim_gate
        .get("ready_lane_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let lane_count = claim_gate
        .get("lane_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let runtime_green_candidate = proof_path
        .get("runtime_green_candidate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let current_best_next = proof_path
        .pointer("/current/current_best_next")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let next_required_proof =
        runtime_green_next_required_proof(&current_best_next, runtime_green_candidate);
    let final_operator_checklist = runtime_green_final_operator_checklist(
        &next_required_proof,
        runtime_green_candidate,
        root_mode,
    );
    let regression_watch_rollup = proof_path
        .pointer("/current/regression_watch_rollup")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_CLAIM_GATE_SCHEMA,
        "status": status,
        "runtime_status": proof_path.get("runtime_status").and_then(Value::as_str),
        "root_mode": root_mode,
        "runtime_green_candidate": runtime_green_candidate,
        "ready_lane_count": ready_lane_count,
        "lane_count": lane_count,
        "ready_lane_fraction": operator_summary
            .get("ready_lane_fraction")
            .and_then(Value::as_str)
            .map(|fraction| fraction.to_string())
            .unwrap_or_else(|| format!("{ready_lane_count}/{lane_count}")),
        "runtime_evidence_blocker_count": operator_summary.get("runtime_evidence_blocker_count").and_then(Value::as_u64),
        "manual_blocker_count": operator_summary.get("manual_blocker_count").and_then(Value::as_u64),
        "missing_required_file_count": operator_summary.get("missing_required_file_count").and_then(Value::as_u64),
        "stale_required_file_count": operator_summary.get("stale_required_file_count").and_then(Value::as_u64),
        "first_pending_lane_id": operator_summary.get("first_pending_lane_id").and_then(Value::as_str),
        "first_pending_lane_label": operator_summary.get("first_pending_lane_label").and_then(Value::as_str),
        "first_pending_lane_status": operator_summary.get("first_pending_lane_status").and_then(Value::as_str),
        "browser_final_validation_runtime_green": operator_summary.get("browser_final_validation_runtime_green").and_then(Value::as_bool),
        "can_claim_runtime_green": operator_summary.get("can_claim_runtime_green").and_then(Value::as_bool),
        "claim_only_when": claim_gate
            .get("claim_only_when")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "requires_final_windows_just_run": claim_gate.get("requires_final_windows_just_run").and_then(Value::as_bool).unwrap_or(true),
        "final_manual_command": "just run",
        "next_operator_step": operator_summary.get("next_operator_step").and_then(Value::as_str),
        "next_required_proof": next_required_proof,
        "final_operator_checklist": final_operator_checklist,
        "regression_watch": {
            "schema": AGENT_PLUGIN_RUNTIME_OBSERVABILITY_WATCH_ROLLUP_SCHEMA,
            "status": regression_watch_rollup.get("status").and_then(Value::as_str),
            "watched_plugin_count": regression_watch_rollup
                .get("watched_plugin_count")
                .and_then(Value::as_u64),
            "total_must_recheck_count": regression_watch_rollup
                .get("total_must_recheck_count")
                .and_then(Value::as_u64),
            "total_must_not_regress_count": regression_watch_rollup
                .get("total_must_not_regress_count")
                .and_then(Value::as_u64),
            "first_watch": regression_watch_rollup.get("first_watch").cloned(),
            "copy_action": "copy_agent_plugin_runtime_observability_digest",
            "send_action": "send_agent_plugin_runtime_observability_digest_to_agent",
            "source_field": "runtime_observability_digest.plugin_matrix.regression_watch_rollup",
            "review_before_runtime_green_claim": true,
        },
        "copy_action": "copy_agent_plugin_runtime_green_claim_gate",
        "send_action": "send_agent_plugin_runtime_green_claim_gate_to_agent",
        "proof_path_copy_action": "copy_agent_plugin_runtime_green_proof_path",
        "proof_path_send_action": "send_agent_plugin_runtime_green_proof_path_to_agent",
        "source_field": "runtime_green_proof_path.claim_gate",
        "read_only": true,
        "writes_files": false,
        "runs_node": false,
        "launches_browser": false,
        "dispatches_input": false,
    })
}

fn runtime_green_claim_gate_summary(claim_gate: &Value) -> Value {
    let checklist = claim_gate
        .get("final_operator_checklist")
        .unwrap_or(&Value::Null);

    serde_json::json!({
        "status": claim_gate.get("status").and_then(Value::as_str),
        "ready_lane_fraction": claim_gate
            .get("ready_lane_fraction")
            .and_then(Value::as_str),
        "first_pending_lane_label": claim_gate
            .get("first_pending_lane_label")
            .and_then(Value::as_str),
        "first_pending_lane_status": claim_gate
            .get("first_pending_lane_status")
            .and_then(Value::as_str),
        "can_claim_runtime_green": claim_gate
            .get("can_claim_runtime_green")
            .and_then(Value::as_bool),
        "next_required_proof_id": claim_gate
            .pointer("/next_required_proof/required_proof_id")
            .and_then(Value::as_str),
        "next_recommended_action": claim_gate
            .pointer("/next_required_proof/recommended_action")
            .and_then(Value::as_str),
        "regression_watch_status": claim_gate.pointer("/regression_watch/status").and_then(Value::as_str),
        "regression_watch_lane_count": claim_gate
            .pointer("/regression_watch/watched_plugin_count")
            .and_then(Value::as_u64),
        "first_regression_watch_status": claim_gate
            .pointer("/regression_watch/first_watch/status")
            .and_then(Value::as_str),
        "final_operator_checklist": {
            "status": checklist.get("status").and_then(Value::as_str),
            "can_run_final_manual_command": checklist
                .get("can_run_final_manual_command")
                .and_then(Value::as_bool),
            "final_manual_command": checklist
                .get("final_manual_command")
                .and_then(Value::as_str),
            "first_required_proof_id": checklist
                .get("first_required_proof_id")
                .and_then(Value::as_str),
            "first_recommended_tool": checklist
                .get("first_recommended_tool")
                .and_then(Value::as_str),
            "first_recommended_action": checklist
                .get("first_recommended_action")
                .and_then(Value::as_str),
            "ordered_check_count": checklist
                .get("ordered_checks")
                .and_then(Value::as_array)
                .map(Vec::len),
            "may_report_runtime_green": checklist
                .pointer("/reporting_policy/may_report_runtime_green")
                .and_then(Value::as_bool),
            "requires_imported_final_result": checklist
                .pointer("/reporting_policy/requires_imported_final_result")
                .and_then(Value::as_bool),
            "read_only": checklist.get("read_only").and_then(Value::as_bool),
        },
        "copy_action": claim_gate.get("copy_action").and_then(Value::as_str),
        "send_action": claim_gate.get("send_action").and_then(Value::as_str),
        "read_only": claim_gate.get("read_only").and_then(Value::as_bool),
    })
}

fn runtime_green_claim_readiness(proof_path: &Value, claim_gate: &Value) -> Value {
    let runtime_green_candidate = proof_path
        .get("runtime_green_candidate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let final_result_summary = proof_path
        .pointer("/current/final_validation_result")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let final_result_present = final_result_summary
        .get("state")
        .and_then(Value::as_str)
        .map(|state| state != "missing_result")
        .unwrap_or_else(|| {
            final_result_summary
                .as_object()
                .map_or(false, |object| !object.is_empty())
        });
    let final_result_runtime_green = final_result_summary
        .get("runtime_green_candidate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let can_report_runtime_green = runtime_green_candidate && final_result_runtime_green;
    let status = if can_report_runtime_green {
        "ready_to_report_runtime_green"
    } else if !runtime_green_candidate {
        "runtime_evidence_required"
    } else if !final_result_present {
        "final_validation_result_missing"
    } else if !final_result_runtime_green {
        "final_validation_result_not_runtime_green"
    } else {
        "manual_review_required"
    };
    let checklist = claim_gate
        .get("final_operator_checklist")
        .unwrap_or(&Value::Null);
    let regression_watch_rollup = claim_gate
        .get("regression_watch")
        .cloned()
        .or_else(|| {
            proof_path
                .pointer("/current/regression_watch_rollup")
                .cloned()
        })
        .unwrap_or_else(|| serde_json::json!({}));

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_CLAIM_READINESS_SCHEMA,
        "status": status,
        "root_mode": proof_path.get("root_mode").and_then(Value::as_str),
        "can_report_runtime_green": can_report_runtime_green,
        "runtime_green_candidate": runtime_green_candidate,
        "final_result_present": final_result_present,
        "final_result_runtime_green": final_result_runtime_green,
        "final_manual_command": "just run",
        "claim_gate_status": claim_gate.get("status").and_then(Value::as_str),
        "ready_lane_fraction": claim_gate.get("ready_lane_fraction").and_then(Value::as_str),
        "first_pending_lane_id": claim_gate
            .get("first_pending_lane_id")
            .and_then(Value::as_str),
        "first_pending_lane_label": claim_gate
            .get("first_pending_lane_label")
            .and_then(Value::as_str),
        "first_pending_lane_status": claim_gate
            .get("first_pending_lane_status")
            .and_then(Value::as_str),
        "next_required_proof_id": claim_gate
            .pointer("/next_required_proof/required_proof_id")
            .and_then(Value::as_str),
        "next_recommended_action": claim_gate
            .pointer("/next_required_proof/recommended_action")
            .and_then(Value::as_str),
        "next_recommended_tool": claim_gate
            .pointer("/next_required_proof/recommended_tool")
            .and_then(Value::as_str),
        "regression_watch": {
            "schema": AGENT_PLUGIN_RUNTIME_OBSERVABILITY_WATCH_ROLLUP_SCHEMA,
            "status": regression_watch_rollup.get("status").and_then(Value::as_str),
            "watched_plugin_count": regression_watch_rollup
                .get("watched_plugin_count")
                .and_then(Value::as_u64),
            "total_must_recheck_count": regression_watch_rollup
                .get("total_must_recheck_count")
                .and_then(Value::as_u64),
            "total_must_not_regress_count": regression_watch_rollup
                .get("total_must_not_regress_count")
                .and_then(Value::as_u64),
            "first_watch": regression_watch_rollup.get("first_watch").cloned(),
            "copy_action": "copy_agent_plugin_runtime_observability_digest",
            "send_action": "send_agent_plugin_runtime_observability_digest_to_agent",
            "source_field": "runtime_observability_digest.plugin_matrix.regression_watch_rollup",
            "review_before_runtime_green_claim": true,
        },
        "final_operator_checklist": {
            "status": checklist.get("status").and_then(Value::as_str),
            "can_run_final_manual_command": checklist
                .get("can_run_final_manual_command")
                .and_then(Value::as_bool),
            "first_required_proof_id": checklist
                .get("first_required_proof_id")
                .and_then(Value::as_str),
            "first_recommended_tool": checklist
                .get("first_recommended_tool")
                .and_then(Value::as_str),
            "ordered_check_count": checklist
                .get("ordered_checks")
                .and_then(Value::as_array)
                .map(Vec::len),
        },
        "final_validation_result_summary": final_result_summary,
        "copy_action": "copy_agent_plugin_runtime_green_claim_readiness",
        "send_action": "send_agent_plugin_runtime_green_claim_readiness_to_agent",
        "claim_gate_copy_action": "copy_agent_plugin_runtime_green_claim_gate",
        "claim_gate_send_action": "send_agent_plugin_runtime_green_claim_gate_to_agent",
        "proof_path_copy_action": "copy_agent_plugin_runtime_green_proof_path",
        "proof_path_send_action": "send_agent_plugin_runtime_green_proof_path_to_agent",
        "reporting_policy": {
            "may_report_runtime_green": can_report_runtime_green,
            "requires_runtime_green_candidate": true,
            "requires_imported_final_result": true,
            "requires_final_manual_command": "just run",
            "requires_regression_watch_review_before_manual_claim": true
        },
        "read_only": true,
        "writes_files": false,
        "runs_node": false,
        "launches_browser": false,
        "dispatches_input": false,
    })
}

fn runtime_green_report_gate(readiness: &Value) -> Value {
    let can_report = readiness
        .get("can_report_runtime_green")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let runtime_green_candidate = readiness
        .get("runtime_green_candidate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let final_result_present = readiness
        .get("final_result_present")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let final_result_runtime_green = readiness
        .get("final_result_runtime_green")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let status = if can_report {
        "ready_to_report_runtime_green"
    } else if !runtime_green_candidate {
        "blocked_by_runtime_evidence"
    } else if !final_result_present {
        "blocked_by_missing_final_result"
    } else if !final_result_runtime_green {
        "blocked_by_final_result"
    } else {
        "blocked_by_manual_review"
    };
    let blocker = if can_report {
        "none"
    } else if !runtime_green_candidate {
        "runtime_green_candidate_false"
    } else if !final_result_present {
        "final_validation_result_missing"
    } else if !final_result_runtime_green {
        "final_validation_result_not_runtime_green"
    } else {
        "manual_review_required"
    };
    let label = if can_report {
        "Runtime-green ready to report"
    } else {
        "Runtime-green blocked by proof"
    };
    let next_action = readiness
        .get("next_recommended_action")
        .and_then(Value::as_str)
        .unwrap_or("copy_agent_plugin_runtime_green_claim_readiness");
    let badge =
        runtime_green_report_badge_from_fields(status, label, can_report, blocker, next_action);

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_GATE_SCHEMA,
        "status": status,
        "label": label,
        "severity": if can_report { "success" } else { "blocked" },
        "can_report_runtime_green": can_report,
        "blocker": blocker,
        "next_action": next_action,
        "badge": badge,
        "ready_lane_fraction": readiness.get("ready_lane_fraction").and_then(Value::as_str),
        "claim_readiness_status": readiness.get("status").and_then(Value::as_str),
        "claim_gate_status": readiness.get("claim_gate_status").and_then(Value::as_str),
        "next_required_proof_id": readiness.get("next_required_proof_id").and_then(Value::as_str),
        "regression_watch": readiness.get("regression_watch").cloned(),
        "final_result_present": final_result_present,
        "final_result_runtime_green": final_result_runtime_green,
        "final_manual_command": "just run",
        "source_schema": readiness.get("schema").and_then(Value::as_str),
        "copy_action": "copy_agent_plugin_runtime_green_report_gate",
        "send_action": "send_agent_plugin_runtime_green_report_gate_to_agent",
        "claim_readiness_copy_action": "copy_agent_plugin_runtime_green_claim_readiness",
        "claim_readiness_send_action": "send_agent_plugin_runtime_green_claim_readiness_to_agent",
        "reporting_policy": {
            "may_report_runtime_green": can_report,
            "requires_runtime_green_candidate": true,
            "requires_imported_final_result": true,
            "requires_final_manual_command": "just run",
            "requires_regression_watch_review_before_manual_claim": true
        },
        "read_only": true,
        "writes_files": false,
        "runs_node": false,
        "launches_browser": false,
        "dispatches_input": false,
    })
}

fn runtime_green_report_badge(report_gate: &Value) -> Value {
    report_gate.get("badge").cloned().unwrap_or_else(|| {
        let status = report_gate
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let label = report_gate
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("Runtime-green report gate");
        let can_report = report_gate
            .get("can_report_runtime_green")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let blocker = report_gate
            .get("blocker")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let next_action = report_gate
            .get("next_action")
            .and_then(Value::as_str)
            .unwrap_or("copy_agent_plugin_runtime_green_report_gate");

        runtime_green_report_badge_from_fields(status, label, can_report, blocker, next_action)
    })
}

fn runtime_green_report_badge_from_fields(
    status: &str,
    label: &str,
    can_report: bool,
    blocker: &str,
    next_action: &str,
) -> Value {
    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_BADGE_SCHEMA,
        "label": label,
        "status": status,
        "tone": if can_report { "success" } else { "blocked" },
        "icon": if can_report { "check" } else { "shield-alert" },
        "text": if can_report {
            "Runtime-green can be reported"
        } else {
            "Runtime-green proof still blocked"
        },
        "can_report_runtime_green": can_report,
        "blocker": blocker,
        "next_action": next_action,
        "required_before_status_claim": true,
        "visible_in": [
            "agent_panel",
            "webpreview_status_packet",
            "final_validation_bundle",
            "agent_plugin_catalog"
        ],
        "copy_action": "copy_agent_plugin_runtime_green_report_gate",
        "send_action": "send_agent_plugin_runtime_green_report_gate_to_agent",
        "read_only": true,
        "writes_files": false,
        "runs_node": false,
        "launches_browser": false,
        "dispatches_input": false,
    })
}

fn runtime_green_final_proof_guide(report_gate: &Value, root_mode: &str) -> Value {
    let badge = runtime_green_report_badge(report_gate);
    let can_report = report_gate
        .get("can_report_runtime_green")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let blocker = report_gate
        .get("blocker")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let status = if can_report {
        "ready_to_report_runtime_green"
    } else if blocker == "final_validation_result_missing"
        || blocker == "final_validation_result_not_runtime_green"
    {
        "final_result_required"
    } else {
        "runtime_proof_required"
    };
    let next_action = if can_report {
        "copy_agent_plugin_runtime_green_report_gate"
    } else if blocker == "final_validation_result_missing"
        || blocker == "final_validation_result_not_runtime_green"
    {
        "copy_agent_browser_final_validation_result_template"
    } else {
        report_gate
            .get("next_action")
            .and_then(Value::as_str)
            .unwrap_or("copy_agent_plugin_runtime_green_report_gate")
    };

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_GUIDE_SCHEMA,
        "status": status,
        "root_mode": root_mode,
        "badge": badge,
        "can_report_runtime_green": can_report,
        "blocker": blocker,
        "next_action": next_action,
        "report_gate_status": report_gate.get("status").and_then(Value::as_str),
        "manual_command": "just run",
        "ordered_steps": [
            {
                "id": "read_report_badge",
                "action": "copy_agent_plugin_runtime_green_report_gate",
                "purpose": "Start from the compact ready/blocked status row before opening larger proof packets.",
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "id": "copy_final_validation_bundle",
                "action": "copy_agent_browser_final_validation_bundle",
                "purpose": "Collect the final proof checklist, runbook, action manifest, and required result schema.",
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "id": "copy_final_result_template",
                "action": "copy_agent_browser_final_validation_result_template",
                "purpose": "Prepare the fillable final Windows proof result before the manual runtime pass.",
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "id": "manual_just_run",
                "manual_command": "just run",
                "purpose": "The operator runs this only when ready; this guide never invokes it.",
                "writes_files": "manual_runtime_only",
                "dispatches_input": "manual_validation_only"
            },
            {
                "id": "import_final_result",
                "action": "import_agent_browser_final_validation_result_from_clipboard",
                "purpose": "Persist the filled final result into the managed proof file after explicit user action.",
                "writes_files": true,
                "managed_proof_write": true,
                "dispatches_input": false
            },
            {
                "id": "copy_import_receipt",
                "action": "copy_agent_browser_final_validation_result_import_receipt",
                "purpose": "Capture durable proof paths and the post-import report-gate state before rechecking runtime status.",
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "id": "recheck_runtime_status",
                "tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                "payload": runtime_green_status_inspect_payload(root_mode),
                "purpose": "Confirm the durable final result and refreshed report gate before any runtime-green report.",
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "id": "copy_final_report_packet",
                "action": "copy_agent_plugin_runtime_green_final_report_packet",
                "purpose": "Read the compact final reporting packet that combines the report gate, import receipt, final-proof guide, and observability state.",
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "id": "copy_report_gate_again",
                "action": "copy_agent_plugin_runtime_green_report_gate",
                "purpose": "Use the refreshed ready/blocked gate as the only source for the final status claim.",
                "writes_files": false,
                "dispatches_input": false
            }
        ],
        "required_before_status_claim": true,
        "copy_action": "copy_agent_plugin_runtime_green_final_proof_guide",
        "send_action": "send_agent_plugin_runtime_green_final_proof_guide_to_agent",
        "template_action": "copy_agent_browser_final_validation_result_template",
        "import_action": "import_agent_browser_final_validation_result_from_clipboard",
        "import_receipt_action": "copy_agent_browser_final_validation_result_import_receipt",
        "final_report_packet_action": "copy_agent_plugin_runtime_green_final_report_packet",
        "final_bundle_action": "copy_agent_browser_final_validation_bundle",
        "read_only": true,
        "writes_files": false,
        "runs_node": false,
        "launches_browser": false,
        "dispatches_input": false,
    })
}

fn runtime_green_final_proof_guide_summary(guide: &Value) -> Value {
    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_GUIDE_SUMMARY_SCHEMA,
        "source_schema": guide.get("schema").and_then(Value::as_str),
        "status": guide.get("status").and_then(Value::as_str),
        "root_mode": guide.get("root_mode").and_then(Value::as_str),
        "can_report_runtime_green": guide
            .get("can_report_runtime_green")
            .and_then(Value::as_bool),
        "blocker": guide.get("blocker").and_then(Value::as_str),
        "next_action": guide.get("next_action").and_then(Value::as_str),
        "report_gate_status": guide
            .get("report_gate_status")
            .and_then(Value::as_str),
        "manual_command": guide.get("manual_command").and_then(Value::as_str),
        "ordered_step_count": guide
            .get("ordered_steps")
            .and_then(Value::as_array)
            .map(Vec::len),
        "first_step_id": guide
            .pointer("/ordered_steps/0/id")
            .and_then(Value::as_str),
        "manual_step_id": "manual_just_run",
        "required_before_status_claim": guide
            .get("required_before_status_claim")
            .and_then(Value::as_bool),
        "copy_action": guide.get("copy_action").and_then(Value::as_str),
        "send_action": guide.get("send_action").and_then(Value::as_str),
        "template_action": guide.get("template_action").and_then(Value::as_str),
        "import_action": guide.get("import_action").and_then(Value::as_str),
        "import_receipt_action": guide
            .get("import_receipt_action")
            .and_then(Value::as_str),
        "final_report_packet_action": guide
            .get("final_report_packet_action")
            .and_then(Value::as_str),
        "final_bundle_action": guide
            .get("final_bundle_action")
            .and_then(Value::as_str),
        "read_only": guide.get("read_only").and_then(Value::as_bool),
        "writes_files": guide.get("writes_files").and_then(Value::as_bool),
        "runs_node": guide.get("runs_node").and_then(Value::as_bool),
        "launches_browser": guide.get("launches_browser").and_then(Value::as_bool),
        "dispatches_input": guide.get("dispatches_input").and_then(Value::as_bool),
    })
}

fn runtime_green_final_report_packet(
    report_gate: &Value,
    final_proof_guide: &Value,
    root_mode: &str,
) -> Value {
    let can_report = report_gate
        .get("can_report_runtime_green")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let report_gate_status = report_gate
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let report_gate_blocker = report_gate
        .get("blocker")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let next_report_action = report_gate
        .get("next_action")
        .and_then(Value::as_str)
        .unwrap_or("copy_agent_plugin_runtime_green_report_gate");
    let status = if can_report {
        "ready_to_report_runtime_green_from_runtime_status"
    } else {
        "blocked_by_report_gate"
    };
    let next_action = if can_report {
        "copy_agent_plugin_runtime_green_report_gate"
    } else {
        next_report_action
    };
    let regression_watch = report_gate
        .get("regression_watch")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_REPORT_PACKET_SCHEMA,
        "status": status,
        "root_mode": root_mode,
        "may_report_runtime_green": can_report,
        "blocker": if can_report { "none" } else { report_gate_blocker },
        "next_action": next_action,
        "final_manual_command": "just run",
        "regression_watch": regression_watch,
        "report_gate": {
            "schema": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_GATE_SCHEMA,
            "status": report_gate_status,
            "can_report_runtime_green": can_report,
            "blocker": report_gate_blocker,
            "label": report_gate.get("label").and_then(Value::as_str),
            "severity": report_gate.get("severity").and_then(Value::as_str),
            "next_action": next_report_action,
            "badge": report_gate.get("badge").cloned(),
            "copy_action": "copy_agent_plugin_runtime_green_report_gate",
            "send_action": "send_agent_plugin_runtime_green_report_gate_to_agent"
        },
        "final_proof_guide": {
            "schema": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_GUIDE_SCHEMA,
            "status": final_proof_guide.get("status").and_then(Value::as_str),
            "blocker": final_proof_guide.get("blocker").and_then(Value::as_str),
            "next_action": final_proof_guide.get("next_action").and_then(Value::as_str),
            "copy_action": "copy_agent_plugin_runtime_green_final_proof_guide",
            "send_action": "send_agent_plugin_runtime_green_final_proof_guide_to_agent"
        },
        "webpreview_import_receipt": {
            "schema": AGENT_BROWSER_FINAL_VALIDATION_RESULT_IMPORT_RECEIPT_SCHEMA,
            "source_action": "import_agent_browser_final_validation_result_from_clipboard",
            "copy_action": "copy_agent_browser_final_validation_result_import_receipt",
            "send_action": "send_agent_browser_final_validation_result_import_receipt_to_agent",
            "recommended_for_handoff_audit": true
        },
        "required_evidence": [
            {
                "id": "report_gate_allows_runtime_green",
                "ready": can_report,
                "source": "runtime_green_report_gate.can_report_runtime_green",
                "required_for_final_status_claim": true
            },
            {
                "id": "durable_final_validation_result",
                "ready": report_gate
                    .get("final_result_runtime_green")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                "source": "runtime_green_report_gate.final_result_runtime_green",
                "required_for_final_status_claim": true
            },
            {
                "id": "post_import_receipt_handoff",
                "ready": false,
                "state": "unknown_to_runtime_status_tool",
                "source": "WebPreview final_validation_result_import_receipt",
                "required_for_handoff_audit": true
            }
        ],
        "reporting_policy": {
            "may_report_runtime_green": can_report,
            "source_of_truth": "runtime_green_report_gate",
            "requires_final_manual_command": "just run",
            "requires_imported_final_result": true,
            "import_receipt_recommended_for_handoff": true,
            "requires_regression_watch_review_before_manual_claim": true
        },
        "webpreview_copy_action": "copy_agent_plugin_runtime_green_final_report_packet",
        "webpreview_send_action": "send_agent_plugin_runtime_green_final_report_packet_to_agent",
        "read_only": true,
        "writes_files": false,
        "runs_node": false,
        "launches_browser": false,
        "dispatches_input": false,
    })
}

fn runtime_green_final_report_packet_summary(packet: &Value) -> Value {
    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_REPORT_PACKET_SUMMARY_SCHEMA,
        "source_schema": packet.get("schema").and_then(Value::as_str),
        "status": packet.get("status").and_then(Value::as_str),
        "root_mode": packet.get("root_mode").and_then(Value::as_str),
        "may_report_runtime_green": packet
            .get("may_report_runtime_green")
            .and_then(Value::as_bool),
        "blocker": packet.get("blocker").and_then(Value::as_str),
        "next_action": packet.get("next_action").and_then(Value::as_str),
        "final_manual_command": packet
            .get("final_manual_command")
            .and_then(Value::as_str),
        "report_gate_status": packet
            .pointer("/report_gate/status")
            .and_then(Value::as_str),
        "report_gate_can_report": packet
            .pointer("/report_gate/can_report_runtime_green")
            .and_then(Value::as_bool),
        "report_gate_next_action": packet
            .pointer("/report_gate/next_action")
            .and_then(Value::as_str),
        "final_proof_guide_status": packet
            .pointer("/final_proof_guide/status")
            .and_then(Value::as_str),
        "final_proof_guide_next_action": packet
            .pointer("/final_proof_guide/next_action")
            .and_then(Value::as_str),
        "regression_watch_status": packet
            .pointer("/regression_watch/status")
            .and_then(Value::as_str),
        "regression_watch_lane_count": packet
            .pointer("/regression_watch/watched_plugin_count")
            .and_then(Value::as_u64),
        "first_regression_watch_status": packet
            .pointer("/regression_watch/first_watch/status")
            .and_then(Value::as_str),
        "webpreview_copy_action": packet
            .get("webpreview_copy_action")
            .and_then(Value::as_str),
        "webpreview_send_action": packet
            .get("webpreview_send_action")
            .and_then(Value::as_str),
        "report_gate_copy_action": packet
            .pointer("/report_gate/copy_action")
            .and_then(Value::as_str),
        "final_proof_guide_copy_action": packet
            .pointer("/final_proof_guide/copy_action")
            .and_then(Value::as_str),
        "import_receipt_copy_action": packet
            .pointer("/webpreview_import_receipt/copy_action")
            .and_then(Value::as_str),
        "read_only": packet.get("read_only").and_then(Value::as_bool),
        "writes_files": packet.get("writes_files").and_then(Value::as_bool),
        "runs_node": packet.get("runs_node").and_then(Value::as_bool),
        "launches_browser": packet.get("launches_browser").and_then(Value::as_bool),
        "dispatches_input": packet.get("dispatches_input").and_then(Value::as_bool),
    })
}

fn runtime_green_report_readiness_card(
    claim_readiness: &Value,
    report_gate: &Value,
    final_report_packet: &Value,
    final_proof_audit: &Value,
) -> Value {
    let may_report_runtime_green = final_report_packet
        .get("may_report_runtime_green")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let report_gate_can_report = report_gate
        .get("can_report_runtime_green")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let audit_status = final_proof_audit
        .pointer("/audit/status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let status = if may_report_runtime_green && report_gate_can_report {
        "ready_to_report_runtime_green"
    } else if audit_status == "awaiting_final_runtime_result_import" {
        "final_runtime_result_import_required"
    } else if audit_status == "final_result_needs_evidence_or_blocker_fix" {
        "final_runtime_result_evidence_required"
    } else {
        "runtime_green_report_blocked"
    };
    let next_action = final_report_packet
        .get("next_action")
        .and_then(Value::as_str)
        .or_else(|| {
            final_proof_audit
                .pointer("/audit/next_action")
                .and_then(Value::as_str)
        })
        .or_else(|| report_gate.get("next_action").and_then(Value::as_str))
        .unwrap_or("copy_agent_plugin_runtime_green_report_gate");
    let blocker = final_report_packet
        .get("blocker")
        .and_then(Value::as_str)
        .or_else(|| report_gate.get("blocker").and_then(Value::as_str))
        .unwrap_or("unknown");
    let regression_watch = final_report_packet
        .get("regression_watch")
        .or_else(|| report_gate.get("regression_watch"))
        .unwrap_or(&Value::Null);

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_READINESS_CARD_SCHEMA,
        "status": status,
        "may_report_runtime_green": may_report_runtime_green,
        "blocker": blocker,
        "next_action": next_action,
        "final_manual_command": "just run",
        "claim_readiness": {
            "schema": AGENT_PLUGIN_RUNTIME_GREEN_CLAIM_READINESS_SCHEMA,
            "status": claim_readiness.get("status").and_then(Value::as_str),
            "can_report_runtime_green": claim_readiness
                .get("can_report_runtime_green")
                .and_then(Value::as_bool),
            "ready_lane_fraction": claim_readiness
                .get("ready_lane_fraction")
                .and_then(Value::as_str),
            "first_pending_lane_label": claim_readiness
                .get("first_pending_lane_label")
                .and_then(Value::as_str),
            "first_pending_lane_status": claim_readiness
                .get("first_pending_lane_status")
                .and_then(Value::as_str),
            "next_required_proof_id": claim_readiness
                .get("next_required_proof_id")
                .and_then(Value::as_str),
            "next_recommended_action": claim_readiness
                .get("next_recommended_action")
                .and_then(Value::as_str),
        },
        "report_gate": {
            "schema": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_GATE_SCHEMA,
            "status": report_gate.get("status").and_then(Value::as_str),
            "can_report_runtime_green": report_gate_can_report,
            "blocker": report_gate.get("blocker").and_then(Value::as_str),
            "severity": report_gate.get("severity").and_then(Value::as_str),
            "badge": report_gate.get("badge").cloned(),
            "next_action": report_gate.get("next_action").and_then(Value::as_str),
        },
        "final_report_packet": {
            "schema": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_REPORT_PACKET_SCHEMA,
            "status": final_report_packet.get("status").and_then(Value::as_str),
            "may_report_runtime_green": may_report_runtime_green,
            "blocker": final_report_packet.get("blocker").and_then(Value::as_str),
            "next_action": final_report_packet.get("next_action").and_then(Value::as_str),
        },
        "final_proof_audit": {
            "schema": AGENT_BROWSER_FINAL_PROOF_AUDIT_SCHEMA,
            "status": audit_status,
            "runtime_green_candidate": final_proof_audit
                .pointer("/audit/runtime_green_candidate")
                .and_then(Value::as_bool),
            "overall_runtime_green_candidate": final_proof_audit
                .pointer("/audit/overall_runtime_green_candidate")
                .and_then(Value::as_bool),
            "final_result_present": final_proof_audit
                .pointer("/audit/final_result_present")
                .and_then(Value::as_bool),
            "final_result_runtime_green": final_proof_audit
                .pointer("/audit/final_result_runtime_green")
                .and_then(Value::as_bool),
            "missing_required_check_count": final_proof_audit
                .pointer("/audit/missing_required_checks")
                .and_then(Value::as_array)
                .map(Vec::len),
            "missing_required_evidence_count": final_proof_audit
                .pointer("/audit/missing_required_evidence")
                .and_then(Value::as_array)
                .map(Vec::len),
            "required_check_blocker_count": final_proof_audit
                .pointer("/audit/required_check_blocker_count")
                .and_then(Value::as_u64),
            "has_overall_blocker": final_proof_audit
                .pointer("/audit/has_overall_blocker")
                .and_then(Value::as_bool),
            "next_action": final_proof_audit
                .pointer("/audit/next_action")
                .and_then(Value::as_str),
        },
        "regression_watch": {
            "schema": AGENT_PLUGIN_RUNTIME_OBSERVABILITY_WATCH_ROLLUP_SCHEMA,
            "status": regression_watch.get("status").and_then(Value::as_str),
            "watched_plugin_count": regression_watch
                .get("watched_plugin_count")
                .and_then(Value::as_u64),
            "total_must_recheck_count": regression_watch
                .get("total_must_recheck_count")
                .and_then(Value::as_u64),
            "total_must_not_regress_count": regression_watch
                .get("total_must_not_regress_count")
                .and_then(Value::as_u64),
            "first_watch_status": regression_watch
                .pointer("/first_watch/status")
                .and_then(Value::as_str),
            "review_before_runtime_green_claim": true,
        },
        "copy_action": "copy_agent_plugin_runtime_green_report_readiness_card",
        "send_action": "send_agent_plugin_runtime_green_report_readiness_card_to_agent",
        "final_report_packet_copy_action": "copy_agent_plugin_runtime_green_final_report_packet",
        "final_report_packet_send_action": "send_agent_plugin_runtime_green_final_report_packet_to_agent",
        "report_gate_copy_action": "copy_agent_plugin_runtime_green_report_gate",
        "report_gate_send_action": "send_agent_plugin_runtime_green_report_gate_to_agent",
        "audit_copy_action": "copy_agent_browser_final_proof_audit",
        "audit_send_action": "send_agent_browser_final_proof_audit_to_agent",
        "read_only": true,
        "writes_files": false,
        "runs_node": false,
        "launches_browser": false,
        "dispatches_input": false,
    })
}

fn runtime_green_report_readiness_card_summary(card: &Value) -> Value {
    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_READINESS_CARD_SUMMARY_SCHEMA,
        "source_schema": card.get("schema").and_then(Value::as_str),
        "status": card.get("status").and_then(Value::as_str),
        "may_report_runtime_green": card
            .get("may_report_runtime_green")
            .and_then(Value::as_bool),
        "blocker": card.get("blocker").and_then(Value::as_str),
        "next_action": card.get("next_action").and_then(Value::as_str),
        "final_manual_command": card
            .get("final_manual_command")
            .and_then(Value::as_str),
        "claim_readiness_status": card
            .pointer("/claim_readiness/status")
            .and_then(Value::as_str),
        "ready_lane_fraction": card
            .pointer("/claim_readiness/ready_lane_fraction")
            .and_then(Value::as_str),
        "first_pending_lane_label": card
            .pointer("/claim_readiness/first_pending_lane_label")
            .and_then(Value::as_str),
        "first_pending_lane_status": card
            .pointer("/claim_readiness/first_pending_lane_status")
            .and_then(Value::as_str),
        "next_required_proof_id": card
            .pointer("/claim_readiness/next_required_proof_id")
            .and_then(Value::as_str),
        "report_gate_status": card
            .pointer("/report_gate/status")
            .and_then(Value::as_str),
        "report_gate_can_report": card
            .pointer("/report_gate/can_report_runtime_green")
            .and_then(Value::as_bool),
        "final_report_packet_status": card
            .pointer("/final_report_packet/status")
            .and_then(Value::as_str),
        "final_report_packet_may_report": card
            .pointer("/final_report_packet/may_report_runtime_green")
            .and_then(Value::as_bool),
        "final_proof_audit_status": card
            .pointer("/final_proof_audit/status")
            .and_then(Value::as_str),
        "final_result_present": card
            .pointer("/final_proof_audit/final_result_present")
            .and_then(Value::as_bool),
        "final_result_runtime_green": card
            .pointer("/final_proof_audit/final_result_runtime_green")
            .and_then(Value::as_bool),
        "missing_required_check_count": card
            .pointer("/final_proof_audit/missing_required_check_count")
            .and_then(Value::as_u64),
        "missing_required_evidence_count": card
            .pointer("/final_proof_audit/missing_required_evidence_count")
            .and_then(Value::as_u64),
        "regression_watch_status": card
            .pointer("/regression_watch/status")
            .and_then(Value::as_str),
        "regression_watch_lane_count": card
            .pointer("/regression_watch/watched_plugin_count")
            .and_then(Value::as_u64),
        "first_regression_watch_status": card
            .pointer("/regression_watch/first_watch_status")
            .and_then(Value::as_str),
        "copy_action": card.get("copy_action").and_then(Value::as_str),
        "send_action": card.get("send_action").and_then(Value::as_str),
        "final_report_packet_copy_action": card
            .get("final_report_packet_copy_action")
            .and_then(Value::as_str),
        "report_gate_copy_action": card
            .get("report_gate_copy_action")
            .and_then(Value::as_str),
        "audit_copy_action": card.get("audit_copy_action").and_then(Value::as_str),
        "read_only": card.get("read_only").and_then(Value::as_bool),
        "writes_files": card.get("writes_files").and_then(Value::as_bool),
        "runs_node": card.get("runs_node").and_then(Value::as_bool),
        "launches_browser": card.get("launches_browser").and_then(Value::as_bool),
        "dispatches_input": card.get("dispatches_input").and_then(Value::as_bool),
    })
}

fn runtime_green_final_proof_audit(
    claim_readiness: &Value,
    report_gate: &Value,
    final_report_packet: &Value,
) -> Value {
    let final_result_summary = claim_readiness
        .get("final_validation_result_summary")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let final_result_present = claim_readiness
        .get("final_result_present")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let final_result_runtime_green = claim_readiness
        .get("final_result_runtime_green")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let overall_runtime_green_candidate = claim_readiness
        .get("runtime_green_candidate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let runtime_green_candidate = final_result_runtime_green;
    let may_report_runtime_green = report_gate
        .get("can_report_runtime_green")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let missing_required_checks = final_result_summary
        .get("missing_required_checks")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let missing_required_evidence = final_result_summary
        .get("missing_required_evidence")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let required_check_blocker_count = final_result_summary
        .get("required_check_blocker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let overall_blocker = final_result_summary.get("overall_blocker").cloned();
    let has_overall_blocker = overall_blocker
        .as_ref()
        .map(|blocker| !blocker.is_null())
        .unwrap_or(false);
    let status = if may_report_runtime_green {
        "ready_to_report_runtime_green"
    } else if !final_result_present {
        "awaiting_final_runtime_result_import"
    } else if final_result_runtime_green || runtime_green_candidate {
        "runtime_green_result_waiting_for_report_gate"
    } else {
        "final_result_needs_evidence_or_blocker_fix"
    };
    let next_action = if may_report_runtime_green {
        "copy_agent_plugin_runtime_green_final_report_packet"
    } else if !final_result_present {
        "copy_agent_browser_final_validation_result_template"
    } else if final_result_runtime_green || runtime_green_candidate {
        "copy_agent_plugin_runtime_green_report_gate"
    } else {
        "copy_agent_browser_final_validation_result"
    };

    serde_json::json!({
        "schema": AGENT_BROWSER_FINAL_PROOF_AUDIT_SCHEMA,
        "audit": {
            "generated_at_ms": current_epoch_millis(),
            "status": status,
            "runtime_green_candidate": runtime_green_candidate,
            "overall_runtime_green_candidate": overall_runtime_green_candidate,
            "final_result_present": final_result_present,
            "final_result_runtime_green": final_result_runtime_green,
            "may_report_runtime_green": may_report_runtime_green,
            "next_action": next_action,
            "missing_required_checks": missing_required_checks,
            "missing_required_evidence": missing_required_evidence,
            "required_check_blocker_count": required_check_blocker_count,
            "overall_blocker": overall_blocker,
            "has_overall_blocker": has_overall_blocker,
            "report_gate": {
                "schema": AGENT_PLUGIN_RUNTIME_GREEN_REPORT_GATE_SCHEMA,
                "status": report_gate.get("status").and_then(Value::as_str),
                "blocker": report_gate.get("blocker").and_then(Value::as_str),
                "can_report_runtime_green": may_report_runtime_green,
                "next_action": report_gate.get("next_action").and_then(Value::as_str),
                "badge": report_gate.get("badge").cloned(),
            },
            "regression_watch": {
                "schema": AGENT_PLUGIN_RUNTIME_OBSERVABILITY_WATCH_ROLLUP_SCHEMA,
                "status": report_gate.pointer("/regression_watch/status").and_then(Value::as_str),
                "watched_plugin_count": report_gate
                    .pointer("/regression_watch/watched_plugin_count")
                    .and_then(Value::as_u64),
                "first_watch_status": report_gate
                    .pointer("/regression_watch/first_watch/status")
                    .and_then(Value::as_str),
            }
        },
        "latest": {
            "final_validation_result": final_result_summary,
            "claim_readiness": {
                "schema": AGENT_PLUGIN_RUNTIME_GREEN_CLAIM_READINESS_SCHEMA,
                "status": claim_readiness.get("status").and_then(Value::as_str),
                "ready_lane_fraction": claim_readiness.get("ready_lane_fraction").and_then(Value::as_str),
                "next_required_proof_id": claim_readiness.get("next_required_proof_id").and_then(Value::as_str),
                "next_recommended_action": claim_readiness.get("next_recommended_action").and_then(Value::as_str),
            },
            "final_report_packet": {
                "schema": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_REPORT_PACKET_SCHEMA,
                "status": final_report_packet.get("status").and_then(Value::as_str),
                "may_report_runtime_green": final_report_packet
                    .get("may_report_runtime_green")
                    .and_then(Value::as_bool),
                "next_action": final_report_packet.get("next_action").and_then(Value::as_str),
                "regression_watch_status": final_report_packet
                    .pointer("/regression_watch/status")
                    .and_then(Value::as_str),
            }
        },
        "required_before_runtime_green_claim": [
            "final_validation_result.status == pass",
            "all required checks have status pass",
            "all required checks include non-empty evidence",
            "overall_blocker is null",
            "every required check blocker is null",
            "runtime_green_report_gate.can_report_runtime_green == true"
        ],
        "source_tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
        "webpreview_copy_action": "copy_agent_browser_final_proof_audit",
        "webpreview_send_action": "send_agent_browser_final_proof_audit_to_agent",
        "read_only": true,
        "writes_files": false,
        "runs_node": false,
        "launches_browser": false,
        "dispatches_input": false,
    })
}

fn runtime_green_final_proof_audit_summary(audit: &Value) -> Value {
    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_AUDIT_SUMMARY_SCHEMA,
        "source_schema": audit.get("schema").and_then(Value::as_str),
        "status": audit.pointer("/audit/status").and_then(Value::as_str),
        "runtime_green_candidate": audit
            .pointer("/audit/runtime_green_candidate")
            .and_then(Value::as_bool),
        "overall_runtime_green_candidate": audit
            .pointer("/audit/overall_runtime_green_candidate")
            .and_then(Value::as_bool),
        "may_report_runtime_green": audit
            .pointer("/audit/may_report_runtime_green")
            .and_then(Value::as_bool),
        "final_result_present": audit
            .pointer("/audit/final_result_present")
            .and_then(Value::as_bool),
        "final_result_runtime_green": audit
            .pointer("/audit/final_result_runtime_green")
            .and_then(Value::as_bool),
        "next_action": audit.pointer("/audit/next_action").and_then(Value::as_str),
        "missing_required_check_count": audit
            .pointer("/audit/missing_required_checks")
            .and_then(Value::as_array)
            .map(Vec::len),
        "missing_required_evidence_count": audit
            .pointer("/audit/missing_required_evidence")
            .and_then(Value::as_array)
            .map(Vec::len),
        "required_check_blocker_count": audit
            .pointer("/audit/required_check_blocker_count")
            .and_then(Value::as_u64),
        "has_overall_blocker": audit
            .pointer("/audit/has_overall_blocker")
            .and_then(Value::as_bool),
        "report_gate_status": audit
            .pointer("/audit/report_gate/status")
            .and_then(Value::as_str),
        "report_gate_blocker": audit
            .pointer("/audit/report_gate/blocker")
            .and_then(Value::as_str),
        "report_gate_can_report_runtime_green": audit
            .pointer("/audit/report_gate/can_report_runtime_green")
            .and_then(Value::as_bool),
        "report_gate_next_action": audit
            .pointer("/audit/report_gate/next_action")
            .and_then(Value::as_str),
        "regression_watch_status": audit
            .pointer("/audit/regression_watch/status")
            .and_then(Value::as_str),
        "regression_watch_lane_count": audit
            .pointer("/audit/regression_watch/watched_plugin_count")
            .and_then(Value::as_u64),
        "first_regression_watch_status": audit
            .pointer("/audit/regression_watch/first_watch_status")
            .and_then(Value::as_str),
        "final_report_packet_status": audit
            .pointer("/latest/final_report_packet/status")
            .and_then(Value::as_str),
        "final_report_packet_next_action": audit
            .pointer("/latest/final_report_packet/next_action")
            .and_then(Value::as_str),
        "webpreview_copy_action": audit
            .get("webpreview_copy_action")
            .and_then(Value::as_str),
        "webpreview_send_action": audit
            .get("webpreview_send_action")
            .and_then(Value::as_str),
        "read_only": audit.get("read_only").and_then(Value::as_bool),
        "writes_files": audit.get("writes_files").and_then(Value::as_bool),
        "runs_node": audit.get("runs_node").and_then(Value::as_bool),
        "launches_browser": audit.get("launches_browser").and_then(Value::as_bool),
        "dispatches_input": audit.get("dispatches_input").and_then(Value::as_bool),
    })
}

fn runtime_green_next_required_proof(
    current_best_next: &Value,
    runtime_green_candidate: bool,
) -> Value {
    let lane_id = current_best_next
        .get("lane_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let lane_label = current_best_next
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or(lane_id);
    let lane_status = current_best_next
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let primary_next_actions = current_best_next
        .get("primary_next_actions")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let operator_steps = current_best_next
        .get("operator_steps")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let recommended_tool = current_best_next
        .pointer("/operator_steps/0/tool")
        .and_then(Value::as_str);
    let recommended_action = current_best_next
        .pointer("/primary_next_actions/0")
        .and_then(Value::as_str);

    match lane_id {
        "browser_webpreview" => serde_json::json!({
            "lane_id": lane_id,
            "label": lane_label,
            "status": lane_status,
            "required_proof_id": "browser_final_validation_result_runtime_green",
            "required_proof_field": "runtime_green_blocker_summary.latest_evidence.browser_final_validation_result.summary.runtime_green_candidate",
            "recommended_action": recommended_action.unwrap_or("copy_agent_browser_final_validation_bundle"),
            "recommended_tool": recommended_tool,
            "primary_next_actions": primary_next_actions,
            "operator_steps": operator_steps,
            "recommended_sequence": [
                "copy_agent_browser_final_validation_bundle",
                "copy_agent_browser_final_validation_result_template",
                "run just run manually when ready for final runtime proof",
                "import_agent_browser_final_validation_result_from_clipboard",
                "copy_agent_browser_final_validation_result_import_receipt",
                "inspect_agent_plugin_runtime_status with include_runtime_green_claim_gate=true"
            ],
            "managed_proof_write": "only WebPreview final-result import writes managed final-proof JSON after explicit user action",
            "writes_files": false,
            "dispatches_input": false
        }),
        "managed_chrome" => serde_json::json!({
            "lane_id": lane_id,
            "label": lane_label,
            "status": lane_status,
            "required_proof_id": "managed_chrome_completed_execution_receipt",
            "required_proof_field": "runtime_green_blocker_summary.latest_evidence.managed_chrome_execution_receipt.read.outcome == completed",
            "recommended_action": recommended_action,
            "recommended_tool": recommended_tool.unwrap_or(AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME),
            "primary_next_actions": primary_next_actions,
            "operator_steps": operator_steps,
            "recommended_sequence": [
                "prepare_agent_plugin_runtime if managed roots are missing",
                "prepare_agent_plugin_managed_assets if the asset receipt is missing or stale",
                "prepare_managed_chrome_playwright_adapter",
                "inspect_managed_chrome_playwright_executions",
                "inspect_agent_plugin_runtime_status with include_runtime_green_claim_gate=true"
            ],
            "managed_proof_write": "permissioned Agent tools may write managed receipts only under managed plugin roots",
            "writes_files": false,
            "dispatches_input": false
        }),
        "pc_use" => serde_json::json!({
            "lane_id": lane_id,
            "label": lane_label,
            "status": lane_status,
            "required_proof_id": "pc_use_ready_runner_receipt",
            "required_proof_field": "runtime_green_blocker_summary.latest_evidence.pc_use_latest_runner_receipt.classification.outcome == ready_future_executor_pending",
            "summary_field": "runtime_green_blocker_summary.latest_evidence.pc_use_proof_summary",
            "recommended_action": recommended_action,
            "recommended_tool": recommended_tool.unwrap_or(AGENT_PC_USE_RUNNER_GATE_TOOL_NAME),
            "primary_next_actions": primary_next_actions,
            "operator_steps": operator_steps,
            "recommended_sequence": [
                "inspect_zed_pc_use_ui_snapshot",
                "inspect_zed_pc_use_payload_queue",
                "request_zed_pc_use_payload_run",
                "inspect_zed_pc_use_runner_receipts",
                "inspect_agent_plugin_runtime_status with include_runtime_green_claim_gate=true"
            ],
            "managed_proof_write": "request_zed_pc_use_payload_run may write a managed future-executor receipt only after explicit authorization",
            "takes_screenshot": false,
            "dispatches_input": false
        }),
        _ => serde_json::json!({
            "lane_id": lane_id,
            "label": lane_label,
            "status": lane_status,
            "required_proof_id": if runtime_green_candidate {
                "final_manual_windows_runtime_proof"
            } else {
                "runtime_green_claim_gate_recheck"
            },
            "required_proof_field": if runtime_green_candidate {
                "manual Windows just run result imported into WebPreview"
            } else {
                "runtime_green_claim_gate.status"
            },
            "recommended_action": recommended_action.unwrap_or("copy_agent_plugin_runtime_green_claim_gate"),
            "recommended_tool": recommended_tool.unwrap_or(AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME),
            "primary_next_actions": primary_next_actions,
            "operator_steps": operator_steps,
            "recommended_sequence": if runtime_green_candidate {
                serde_json::json!([
                    "run just run manually",
                    "copy_agent_browser_final_validation_result_template",
                    "import_agent_browser_final_validation_result_from_clipboard",
                    "copy_agent_browser_final_validation_result_import_receipt",
                    "inspect_agent_plugin_runtime_status with include_runtime_green_claim_gate=true"
                ])
            } else {
                serde_json::json!(["inspect_agent_plugin_runtime_status with include_runtime_green_claim_gate=true"])
            },
            "managed_proof_write": "only explicit permissioned tools or WebPreview imports write managed proof files",
            "writes_files": false,
            "dispatches_input": false
        }),
    }
}

fn runtime_green_final_operator_checklist(
    next_required_proof: &Value,
    runtime_green_candidate: bool,
    root_mode: &str,
) -> Value {
    let required_proof_id = next_required_proof
        .get("required_proof_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let recommended_action = next_required_proof
        .get("recommended_action")
        .and_then(Value::as_str)
        .unwrap_or("copy_agent_plugin_runtime_green_claim_gate");
    let recommended_tool = next_required_proof
        .get("recommended_tool")
        .and_then(Value::as_str)
        .unwrap_or(AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME);
    let recommended_sequence = next_required_proof
        .get("recommended_sequence")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let status = if runtime_green_candidate {
        "ready_for_final_windows_runtime_proof"
    } else {
        "blocked_by_required_proof"
    };

    serde_json::json!({
        "status": status,
        "can_run_final_manual_command": runtime_green_candidate,
        "final_manual_command": "just run",
        "first_required_proof_id": required_proof_id,
        "first_recommended_action": recommended_action,
        "first_recommended_tool": recommended_tool,
        "recommended_sequence": recommended_sequence,
        "ordered_checks": [
            {
                "id": "read_claim_gate",
                "label": "Read the compact runtime-green claim gate.",
                "status": "ready",
                "tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                "payload": runtime_green_status_inspect_payload(root_mode),
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "id": "resolve_first_required_proof",
                "label": "Resolve the first missing runtime proof before the final manual pass.",
                "status": if runtime_green_candidate { "ready" } else { "required" },
                "required_proof_id": required_proof_id,
                "recommended_action": recommended_action,
                "recommended_tool": recommended_tool,
                "writes_files": next_required_proof
                    .get("writes_files")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                "dispatches_input": next_required_proof
                    .get("dispatches_input")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            },
            {
                "id": "run_final_windows_runtime_proof",
                "label": "Run the final manual Windows runtime proof only after required proof is available.",
                "status": if runtime_green_candidate { "required" } else { "blocked" },
                "command": "just run",
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "id": "import_final_result",
                "label": "Import the filled final validation result after the manual proof.",
                "status": if runtime_green_candidate { "pending_manual_result" } else { "blocked" },
                "webpreview_action": "import_agent_browser_final_validation_result_from_clipboard",
                "managed_proof_write": "writes managed final-proof JSON only after explicit WebPreview import",
                "dispatches_input": false
            },
            {
                "id": "copy_import_receipt",
                "label": "Copy the final validation result import receipt.",
                "status": "required_after_import",
                "webpreview_action": "copy_agent_browser_final_validation_result_import_receipt",
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "id": "recheck_claim_gate",
                "label": "Re-read the claim gate and proof path before reporting runtime-green.",
                "status": "required_after_changes",
                "tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                "payload": runtime_green_status_inspect_payload(root_mode),
                "writes_files": false,
                "dispatches_input": false
            }
        ],
        "reporting_policy": {
            "may_report_runtime_green": false,
            "reason": "The code path can only prepare or summarize evidence; a final manual Windows runtime proof and imported result are still required before a runtime-green report.",
            "requires_imported_final_result": true
        },
        "read_only": true
    })
}

fn runtime_green_operator_handoff(
    root_mode: &str,
    runtime_status: &str,
    blocker_summary: &Value,
    scorecard: &Value,
) -> Value {
    let lanes = scorecard
        .get("lanes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let lane_handoffs = lanes
        .iter()
        .map(|lane| runtime_green_lane_operator_handoff(root_mode, lane))
        .collect::<Vec<_>>();
    let current_best_next = lane_handoffs
        .iter()
        .find(|lane| {
            lane.get("ready")
                .and_then(Value::as_bool)
                .map(|ready| !ready)
                .unwrap_or(true)
        })
        .cloned()
        .unwrap_or_else(|| runtime_green_final_operator_handoff(root_mode));
    let runtime_green_candidate = scorecard
        .get("runtime_green_candidate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let ready_lane_count = scorecard
        .pointer("/totals/ready_lane_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let lane_count = scorecard
        .pointer("/totals/lane_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let status = if runtime_green_candidate {
        "runtime_green_candidate"
    } else if lane_count > 0 && ready_lane_count == lane_count {
        "ready_for_final_runtime_validation"
    } else {
        "operator_action_required"
    };

    serde_json::json!({
        "schema": AGENT_PLUGIN_RUNTIME_GREEN_OPERATOR_HANDOFF_SCHEMA,
        "status": status,
        "runtime_status": runtime_status,
        "root_mode": root_mode,
        "runtime_green_candidate": runtime_green_candidate,
        "scorecard_status": scorecard.get("status").and_then(Value::as_str),
        "blocker_summary_status": blocker_summary.get("status").and_then(Value::as_str),
        "current_best_next": current_best_next,
        "lane_handoffs": lane_handoffs,
        "canonical_inspect_payload": runtime_green_status_inspect_payload(root_mode),
        "final_runtime_validation": runtime_green_final_operator_handoff(root_mode),
        "handoff_consumers": [
            "agent_panel",
            "subagents",
            "web_preview_status_packets",
            "future_plugin_operator_ui"
        ],
        "reads_from": [
            "runtime_green_blocker_summary",
            "runtime_green_readiness_scorecard"
        ],
        "safety": {
            "handoff_is_read_only_metadata": true,
            "writes_files": false,
            "runs_node": false,
            "launches_browser": false,
            "dispatches_input": false,
            "touches_real_browser_profiles": false,
            "permissioned_steps_require_user_visible_authorization": true
        }
    })
}

fn runtime_green_lane_operator_handoff(root_mode: &str, lane: &Value) -> Value {
    let lane_id = lane.get("id").and_then(Value::as_str).unwrap_or("unknown");
    let ready = lane.get("ready").and_then(Value::as_bool).unwrap_or(false);
    let status = lane
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("pending_runtime_evidence");
    let primary_next_actions = lane
        .get("primary_next_actions")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));

    serde_json::json!({
        "lane_id": lane_id,
        "label": lane.get("label").and_then(Value::as_str),
        "status": status,
        "ready": ready,
        "blocker_count": lane.get("blocker_count").and_then(Value::as_u64).unwrap_or(0),
        "blockers": lane
            .get("blockers")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "primary_next_actions": primary_next_actions,
        "operator_steps": runtime_green_lane_operator_steps(root_mode, lane_id),
        "post_step_verification": {
            "tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
            "payload": runtime_green_status_inspect_payload(root_mode),
            "expected_field": format!("runtime_green_readiness_scorecard.lanes.{lane_id}.ready"),
            "writes_files": false,
            "runs_node": false,
            "launches_browser": false,
            "dispatches_input": false
        }
    })
}

fn runtime_green_lane_operator_steps(root_mode: &str, lane_id: &str) -> Value {
    match lane_id {
        "browser_webpreview" => serde_json::json!([
            {
                "id": "inspect_browser_queue",
                "tool": AGENT_BROWSER_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                "purpose": "Confirm the latest managed Browser payload exists before WebPreview import validation.",
                "payload": {
                    "root_mode": root_mode,
                    "include_payload_packet": false
                },
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "id": "webpreview_final_result",
                "webpreview_actions": [
                    "copy_agent_browser_final_validation_bundle",
                    "copy_agent_browser_final_validation_result_template",
                    "import_agent_browser_final_validation_result_from_clipboard",
                    "send_agent_browser_final_validation_result_to_agent"
                ],
                "purpose": "Import the managed WebPreview final validation result only after manual runtime proof is available.",
                "writes_files": false,
                "managed_proof_write": "only after explicit WebPreview import action",
                "dispatches_input": false
            }
        ]),
        "managed_chrome" => serde_json::json!([
            {
                "id": "prepare_managed_roots",
                "tool": AgentPluginBootstrapTool::NAME,
                "payload": {
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
                "id": "write_asset_receipt",
                "tool": AGENT_PLUGIN_ASSET_PROVISIONER_TOOL_NAME,
                "payload": {
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
                "id": "prepare_playwright_adapter",
                "tool": AGENT_CHROME_PLAYWRIGHT_ADAPTER_TOOL_NAME,
                "payload": {
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
                "id": "inspect_execution_receipts",
                "tool": AGENT_CHROME_PLAYWRIGHT_EXECUTION_INSPECT_TOOL_NAME,
                "purpose": "Read the latest managed Chrome execution request and receipt before claiming this lane is green.",
                "writes_files": false,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            }
        ]),
        "pc_use" => serde_json::json!([
            {
                "id": "inspect_ui_snapshot",
                "tool": AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME,
                "purpose": "Read safe current Zed UI targets before composing any future PC-use payload.",
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "id": "inspect_pc_use_queue",
                "tool": AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
                "purpose": "Confirm the latest managed PC-use payload before the runner gate writes a receipt.",
                "payload": {
                    "root_mode": root_mode,
                    "include_payload_packet": false
                },
                "writes_files": false,
                "dispatches_input": false
            },
            {
                "id": "request_pc_use_runner_gate",
                "tool": AGENT_PC_USE_RUNNER_GATE_TOOL_NAME,
                "payload": {
                    "root_mode": root_mode,
                    "include_payload_packet": false
                },
                "purpose": "Write only a future-executor readiness receipt; this does not take screenshots or dispatch OS input.",
                "requires_authorization": true,
                "writes_files": true,
                "takes_screenshot": false,
                "dispatches_input": false
            }
        ]),
        _ => serde_json::json!([
            {
                "id": "inspect_runtime_status",
                "tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                "payload": runtime_green_status_inspect_payload(root_mode),
                "writes_files": false,
                "dispatches_input": false
            }
        ]),
    }
}

fn runtime_green_status_inspect_payload(root_mode: &str) -> Value {
    serde_json::json!({
        "root_mode": root_mode,
        "include_latest_handoffs": true,
        "include_host_checks": true,
        "include_next_actions": true,
        "include_workflows": true,
        "include_validation_matrix": true,
        "include_observability_profiles": true,
        "include_observability_digest": true,
        "include_runtime_green_proof_path": true,
        "include_runtime_green_claim_gate": true
    })
}

fn runtime_green_final_operator_handoff(root_mode: &str) -> Value {
    serde_json::json!({
        "lane_id": "final_runtime_validation",
        "label": "Final Windows runtime validation",
        "status": "manual_required",
        "ready": false,
        "operator_steps": [
            {
                "id": "inspect_runtime_status_before_final_pass",
                "tool": AGENT_PLUGIN_RUNTIME_STATUS_TOOL_NAME,
                "payload": runtime_green_status_inspect_payload(root_mode),
                "writes_files": false,
                "runs_node": false,
                "launches_browser": false,
                "dispatches_input": false
            },
            {
                "id": "manual_just_run",
                "manual_command": "just run",
                "when": "Only after Browser/WebPreview, managed Chrome, and PC-use lanes are ready and the user wants one final runtime pass.",
                "writes_files": false,
                "dispatches_input": "manual_validation_only"
            }
        ]
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
                    "final_result_import_receipt": "copy_agent_browser_final_validation_result_import_receipt",
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
                    "adapter execution remains limited to open_url, screenshot, inspect_element, dom_snapshot, runtime_events, set_viewport, and wait_for_selector",
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
            "browser_final_validation_result": browser_final_validation_result_probe(
                &roots.browser_final_validation_latest_result,
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
        "receipt_classifications": {
            "managed_chrome_runner_receipt": outcome_receipt_probe(
                &roots.chrome_latest_runner_receipt,
                Some(AGENT_CHROME_RUNNER_RECEIPT_SCHEMA),
                "/result/outcome",
                &[MANAGED_CHROME_RUNNER_READY_OUTCOME],
                generated_at_ms,
            ),
            "managed_chrome_execution_receipt": latest_prefixed_outcome_receipt_probe(
                &roots.chrome_execution_dir,
                MANAGED_CHROME_EXECUTION_RECEIPT_PREFIX,
                Some(AGENT_CHROME_PLAYWRIGHT_EXECUTION_RECEIPT_SCHEMA),
                "/outcome",
                &[MANAGED_CHROME_EXECUTION_READY_OUTCOME],
                generated_at_ms,
            ),
            "pc_use_runner_receipt": outcome_receipt_probe(
                &roots.pc_use_latest_receipt,
                Some(AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA),
                "/result/outcome",
                &[PC_USE_RUNNER_READY_OUTCOME],
                generated_at_ms,
            ),
        },
        "manual_session_state_required": [
            "WebPreview final validation bundle copy/send state",
            "WebPreview final result template copy/send state",
            "WebPreview imported final result state",
            "Managed WebPreview final validation result file"
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

fn pc_use_proof_summary(roots: &AgentPluginRuntimeRoots, generated_at_ms: u64) -> Value {
    let payload = pc_use_payload_proof_summary(&roots.pc_use_latest_payload, generated_at_ms);
    let runner_receipt =
        pc_use_runner_receipt_proof_summary(&roots.pc_use_latest_receipt, generated_at_ms);
    let payload_ready = payload
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let payload_exists = payload
        .pointer("/file/exists")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let runner_receipt_ready = runner_receipt
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let runner_receipt_exists = runner_receipt
        .pointer("/file/exists")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let status = if runner_receipt_ready {
        "ready_future_executor_receipt_present"
    } else if runner_receipt_exists {
        "runner_receipt_not_ready"
    } else if payload_ready {
        "payload_ready_runner_receipt_missing"
    } else if payload_exists {
        "payload_not_ready"
    } else {
        "payload_missing"
    };

    serde_json::json!({
        "schema": AGENT_PLUGIN_PC_USE_PROOF_SUMMARY_SCHEMA,
        "status": status,
        "generated_at_ms": generated_at_ms,
        "ready": runner_receipt_ready,
        "payload_ready": payload_ready,
        "runner_receipt_ready": runner_receipt_ready,
        "payload": payload,
        "runner_receipt": runner_receipt,
        "next_actions": pc_use_proof_next_actions(status),
        "reads_from": [
            "plugins.pc_use.managed_paths.latest_payload",
            "plugins.pc_use.managed_paths.latest_runner_receipt",
            "runtime_green_blocker_summary.latest_evidence.pc_use_latest_payload",
            "runtime_green_blocker_summary.latest_evidence.pc_use_latest_runner_receipt"
        ],
        "safety": {
            "summary_is_read_only": true,
            "writes_files": false,
            "takes_screenshot": false,
            "focuses_zed": false,
            "dispatches_input": false,
            "launches_processes": false,
            "os_wide_desktop_control": false,
        },
    })
}

fn pc_use_payload_proof_summary(path: &Path, generated_at_ms: u64) -> Value {
    let probe = proof_file_probe(
        path,
        Some(AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA),
        Some("/payload_packet/schema"),
        generated_at_ms,
    );
    let file = proof_file_metadata_summary(&probe, path);
    let value = match read_compact_json_file(path) {
        Ok(value) => value,
        Err(error) => {
            return serde_json::json!({
                "state": error.get("state").and_then(Value::as_str).unwrap_or("read_error"),
                "ready": false,
                "file": file,
                "read_error": error,
            });
        }
    };

    let queue_schema = value.get("schema").and_then(Value::as_str);
    let payload_schema = value
        .pointer("/payload_packet/schema")
        .and_then(Value::as_str);
    let queue_schema_matches = queue_schema == Some(AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA);
    let payload_schema_matches = payload_schema == Some(AGENT_PC_USE_PAYLOAD_SCHEMA);
    let dispatches_input = value
        .pointer("/payload_packet/safety/dispatches_input")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let ready = queue_schema_matches && payload_schema_matches && !dispatches_input;
    let state = if ready {
        "ready"
    } else if !queue_schema_matches {
        "queue_schema_mismatch"
    } else if !payload_schema_matches {
        "payload_schema_mismatch"
    } else {
        "unsafe_dispatching_payload"
    };

    serde_json::json!({
        "state": state,
        "ready": ready,
        "file": file,
        "schema": queue_schema,
        "payload_schema": payload_schema,
        "queue_schema_matches": queue_schema_matches,
        "payload_schema_matches": payload_schema_matches,
        "action": value.pointer("/payload_packet/payload/action").and_then(Value::as_str),
        "surface": value.pointer("/payload_packet/payload/surface").and_then(Value::as_str),
        "target_id_present": value.pointer("/payload_packet/payload/target_id")
            .and_then(Value::as_str)
            .is_some_and(|target_id| !target_id.trim().is_empty()),
        "target_snapshot_id": value.pointer("/payload_packet/payload/target_snapshot_id").and_then(Value::as_str),
        "target_snapshot_id_present": value.pointer("/payload_packet/payload/target_snapshot_id")
            .and_then(Value::as_str)
            .is_some_and(|snapshot_id| !snapshot_id.trim().is_empty()),
        "target_reference": value.pointer("/payload_packet/payload/target_reference").cloned(),
        "safe_for_handoff": !dispatches_input,
        "dispatches_input": dispatches_input,
        "takes_screenshot": value.pointer("/payload_packet/safety/takes_screenshot").and_then(Value::as_bool),
        "focuses_window": value.pointer("/payload_packet/safety/focuses_window").and_then(Value::as_bool),
        "launches_process": value.pointer("/payload_packet/safety/launches_process").and_then(Value::as_bool),
        "os_wide_control": value.pointer("/payload_packet/safety/os_wide_control").and_then(Value::as_bool),
        "source_tool": value.get("source_tool").and_then(Value::as_str)
            .or_else(|| value.pointer("/payload_packet/source_tool").and_then(Value::as_str)),
        "queued_at_ms": value.get("queued_at_ms").cloned(),
    })
}

fn pc_use_runner_receipt_proof_summary(path: &Path, generated_at_ms: u64) -> Value {
    let probe = outcome_receipt_probe(
        path,
        Some(AGENT_PC_USE_RUNNER_RECEIPT_SCHEMA),
        "/result/outcome",
        &[PC_USE_RUNNER_READY_OUTCOME],
        generated_at_ms,
    );
    let file = proof_file_metadata_summary(&probe, path);
    let classification = probe
        .get("classification")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let value = match read_compact_json_file(path) {
        Ok(value) => value,
        Err(error) => {
            return serde_json::json!({
                "state": error.get("state").and_then(Value::as_str).unwrap_or("read_error"),
                "ready": false,
                "file": file,
                "classification": classification,
                "read_error": error,
            });
        }
    };
    let ready = classification
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    serde_json::json!({
        "state": if ready { "ready" } else { "not_ready" },
        "ready": ready,
        "file": file,
        "classification": classification,
        "schema": value.get("schema").and_then(Value::as_str),
        "outcome": value.pointer("/result/outcome").and_then(Value::as_str),
        "root_mode": value.pointer("/result/root_mode").and_then(Value::as_str),
        "action": value.pointer("/queue/action").and_then(Value::as_str),
        "surface": value.pointer("/queue/surface").and_then(Value::as_str),
        "target_id_present": value.pointer("/queue/target_id_present").and_then(Value::as_bool),
        "target_snapshot_id": value.pointer("/queue/target_snapshot_id").and_then(Value::as_str),
        "target_snapshot_id_present": value.pointer("/queue/target_snapshot_id_present").and_then(Value::as_bool),
        "target_reference": value.pointer("/queue/target_reference").cloned(),
        "queue_blocker_count": value.pointer("/queue_blockers").and_then(Value::as_array).map(Vec::len),
        "executor_pending_count": value.pointer("/executor_pending").and_then(Value::as_array).map(Vec::len),
        "queue_blockers": value.get("queue_blockers").cloned().unwrap_or_else(|| serde_json::json!([])),
        "executor_pending": value.get("executor_pending").cloned().unwrap_or_else(|| serde_json::json!([])),
        "future_executor_enabled": value.pointer("/result/future_executor_enabled").and_then(Value::as_bool),
        "screenshot_taken": value.pointer("/result/screenshot_taken").and_then(Value::as_bool),
        "zed_focus_changed": value.pointer("/result/zed_focus_changed").and_then(Value::as_bool),
        "mouse_dispatched": value.pointer("/result/mouse_dispatched").and_then(Value::as_bool),
        "keyboard_dispatched": value.pointer("/result/keyboard_dispatched").and_then(Value::as_bool),
        "process_launched": value.pointer("/result/process_launched").and_then(Value::as_bool),
    })
}

fn proof_file_metadata_summary(probe: &Value, path: &Path) -> Value {
    serde_json::json!({
        "path": path_string(path),
        "exists": probe.get("exists").and_then(Value::as_bool).unwrap_or(false),
        "is_file": probe.get("is_file").and_then(Value::as_bool).unwrap_or(false),
        "byte_len": probe.get("byte_len").cloned(),
        "modified_at_ms": probe.get("modified_at_ms").cloned(),
        "age_seconds": probe.get("age_seconds").cloned(),
        "fresh_within_window": probe.get("fresh_within_window").cloned(),
    })
}

fn read_compact_json_file(path: &Path) -> Result<Value, Value> {
    let metadata = fs::metadata(path).map_err(|error| {
        serde_json::json!({
            "state": if error.kind() == std::io::ErrorKind::NotFound {
                "missing"
            } else {
                "metadata_error"
            },
            "path": path_string(path),
            "error": error.to_string(),
        })
    })?;
    if !metadata.is_file() {
        return Err(serde_json::json!({
            "state": "not_file",
            "path": path_string(path),
        }));
    }
    if metadata.len() > MAX_HANDOFF_PREVIEW_BYTES {
        return Err(serde_json::json!({
            "state": "skipped_too_large",
            "path": path_string(path),
            "byte_len": metadata.len(),
            "max_preview_bytes": MAX_HANDOFF_PREVIEW_BYTES,
        }));
    }
    let bytes = fs::read(path).map_err(|error| {
        serde_json::json!({
            "state": "read_error",
            "path": path_string(path),
            "error": error.to_string(),
        })
    })?;
    serde_json::from_slice::<Value>(&bytes).map_err(|error| {
        serde_json::json!({
            "state": "parse_error",
            "path": path_string(path),
            "byte_len": bytes.len(),
            "error": error.to_string(),
        })
    })
}

fn pc_use_proof_next_actions(status: &str) -> Vec<&'static str> {
    match status {
        "ready_future_executor_receipt_present" => vec![
            AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME,
            "send_pc_use_status_to_agent",
            "Keep PC-use screenshots, focus, click, type, and OS-wide control disabled until a future Zed-window executor consumes the receipt and emits an after-action receipt.",
        ],
        "runner_receipt_not_ready" => vec![
            AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME,
            AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
            AGENT_PC_USE_PAYLOAD_TOOL_NAME,
            AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME,
        ],
        "payload_ready_runner_receipt_missing" => vec![
            AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
            AGENT_PC_USE_RUNNER_GATE_TOOL_NAME,
            AGENT_PC_USE_RUNNER_RECEIPT_INSPECT_TOOL_NAME,
        ],
        "payload_not_ready" => vec![
            AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME,
            AGENT_PC_USE_PAYLOAD_TOOL_NAME,
            AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME,
            AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
        ],
        _ => vec![
            AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME,
            AGENT_PC_USE_PAYLOAD_TOOL_NAME,
            AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME,
            AGENT_PC_USE_PAYLOAD_QUEUE_INSPECT_TOOL_NAME,
        ],
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

fn browser_final_validation_result_probe(path: &Path, generated_at_ms: u64) -> Value {
    let mut probe = proof_file_probe(
        path,
        Some(AGENT_BROWSER_FINAL_VALIDATION_RESULT_SCHEMA),
        None,
        generated_at_ms,
    );
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            probe["summary"] = serde_json::json!({
                "state": "missing_result",
                "runtime_green_candidate": false,
            });
            return probe;
        }
        Err(error) => {
            probe["summary"] = serde_json::json!({
                "state": "read_error",
                "error": error.to_string(),
                "runtime_green_candidate": false,
            });
            return probe;
        }
    };
    if bytes.len() as u64 > MAX_HANDOFF_PREVIEW_BYTES {
        probe["summary"] = serde_json::json!({
            "state": "skipped_too_large",
            "max_preview_bytes": MAX_HANDOFF_PREVIEW_BYTES,
            "runtime_green_candidate": false,
        });
        return probe;
    }
    let result = match serde_json::from_slice::<Value>(&bytes) {
        Ok(result) => result,
        Err(error) => {
            probe["summary"] = serde_json::json!({
                "state": "parse_error",
                "error": error.to_string(),
                "runtime_green_candidate": false,
            });
            return probe;
        }
    };

    probe["summary"] = browser_final_validation_result_summary(&result);
    probe
}

fn browser_final_validation_result_summary(result: &Value) -> Value {
    let checks = result.pointer("/checks").and_then(Value::as_object);
    let required_check_ids = result
        .pointer("/required_check_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let required_check_count = required_check_ids.len();
    let pass_required_check_count = required_check_ids
        .iter()
        .filter_map(Value::as_str)
        .filter(|check_id| {
            checks
                .and_then(|checks| checks.get(*check_id))
                .and_then(|check| check.pointer("/status"))
                .and_then(Value::as_str)
                == Some("pass")
        })
        .count();
    let missing_required_checks = required_check_ids
        .iter()
        .filter_map(Value::as_str)
        .filter(|check_id| checks.and_then(|checks| checks.get(*check_id)).is_none())
        .collect::<Vec<_>>();
    let missing_required_evidence = required_check_ids
        .iter()
        .filter_map(Value::as_str)
        .filter(|check_id| {
            checks
                .and_then(|checks| checks.get(*check_id))
                .and_then(|check| check.pointer("/evidence"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|evidence| !evidence.is_empty())
                .is_none()
        })
        .collect::<Vec<_>>();
    let required_check_blocker_count = required_check_ids
        .iter()
        .filter_map(Value::as_str)
        .filter(|check_id| {
            checks
                .and_then(|checks| checks.get(*check_id))
                .and_then(|check| check.pointer("/blocker"))
                .map(|blocker| !blocker.is_null())
                .unwrap_or(false)
        })
        .count();
    let runtime_green_candidate = result.pointer("/schema").and_then(Value::as_str)
        == Some(AGENT_BROWSER_FINAL_VALIDATION_RESULT_SCHEMA)
        && result.pointer("/status").and_then(Value::as_str) == Some("pass")
        && required_check_count > 0
        && pass_required_check_count == required_check_count
        && result
            .pointer("/overall_blocker")
            .map(Value::is_null)
            .unwrap_or(true)
        && required_check_blocker_count == 0
        && missing_required_checks.is_empty()
        && missing_required_evidence.is_empty();

    serde_json::json!({
        "state": "parsed_result",
        "schema": result.pointer("/schema").and_then(Value::as_str),
        "status": result.pointer("/status").and_then(Value::as_str),
        "runtime_command": result.pointer("/runtime_command").and_then(Value::as_str),
        "branch": result.pointer("/branch").and_then(Value::as_str),
        "commit": result.pointer("/commit").and_then(Value::as_str),
        "started_at": result.pointer("/started_at").cloned(),
        "completed_at": result.pointer("/completed_at").cloned(),
        "required_check_count": required_check_count,
        "pass_required_check_count": pass_required_check_count,
        "required_check_blocker_count": required_check_blocker_count,
        "missing_required_checks": missing_required_checks,
        "missing_required_evidence": missing_required_evidence,
        "runtime_green_candidate": runtime_green_candidate,
        "overall_blocker": result.pointer("/overall_blocker").cloned(),
    })
}

fn outcome_receipt_probe(
    path: &Path,
    expected_schema: Option<&str>,
    outcome_pointer: &str,
    ready_outcomes: &[&str],
    generated_at_ms: u64,
) -> Value {
    let mut probe = proof_file_probe(path, expected_schema, None, generated_at_ms);
    probe["classification"] =
        receipt_outcome_classification(path, expected_schema, outcome_pointer, ready_outcomes);
    probe
}

fn latest_prefixed_outcome_receipt_probe(
    directory: &Path,
    file_name_prefix: &str,
    expected_schema: Option<&str>,
    outcome_pointer: &str,
    ready_outcomes: &[&str],
    generated_at_ms: u64,
) -> Value {
    let mut probe = latest_prefixed_json_file_probe(
        directory,
        file_name_prefix,
        expected_schema,
        generated_at_ms,
    );
    let Some(latest_file) = probe.get("latest_file").and_then(Value::as_str) else {
        probe["classification"] = serde_json::json!({
            "state": "missing_receipt",
            "ready": false,
            "expected_outcomes": ready_outcomes,
        });
        return probe;
    };
    let path = PathBuf::from(latest_file);
    let receipt_probe = outcome_receipt_probe(
        &path,
        expected_schema,
        outcome_pointer,
        ready_outcomes,
        generated_at_ms,
    );
    probe["classification"] = receipt_probe
        .get("classification")
        .cloned()
        .unwrap_or_else(|| {
            serde_json::json!({
                "state": "missing_classification",
                "ready": false,
                "expected_outcomes": ready_outcomes,
            })
        });
    probe["receipt"] = receipt_probe;
    probe
}

fn receipt_probe_ready(probe: &Value) -> bool {
    probe
        .pointer("/classification/ready")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn receipt_outcome_classification(
    path: &Path,
    expected_schema: Option<&str>,
    outcome_pointer: &str,
    ready_outcomes: &[&str],
) -> Value {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return serde_json::json!({
                "state": "missing_receipt",
                "ready": false,
                "expected_schema": expected_schema,
                "expected_outcomes": ready_outcomes,
            });
        }
        Err(error) => {
            return serde_json::json!({
                "state": "read_error",
                "ready": false,
                "expected_schema": expected_schema,
                "expected_outcomes": ready_outcomes,
                "error": error.to_string(),
            });
        }
    };

    if bytes.len() as u64 > MAX_HANDOFF_PREVIEW_BYTES {
        return serde_json::json!({
            "state": "skipped_too_large",
            "ready": false,
            "expected_schema": expected_schema,
            "expected_outcomes": ready_outcomes,
            "max_preview_bytes": MAX_HANDOFF_PREVIEW_BYTES,
        });
    }

    let parsed = match serde_json::from_slice::<Value>(&bytes) {
        Ok(parsed) => parsed,
        Err(error) => {
            return serde_json::json!({
                "state": "parse_error",
                "ready": false,
                "expected_schema": expected_schema,
                "expected_outcomes": ready_outcomes,
                "error": error.to_string(),
            });
        }
    };

    let schema = parsed.get("schema").and_then(Value::as_str);
    let schema_matches = expected_schema
        .map(|expected| schema == Some(expected))
        .unwrap_or(true);
    let outcome = parsed.pointer(outcome_pointer).and_then(Value::as_str);
    let outcome_ready = outcome.is_some_and(|outcome| ready_outcomes.contains(&outcome));
    let ready = schema_matches && outcome_ready;

    serde_json::json!({
        "state": if ready {
            "ready"
        } else if !schema_matches {
            "schema_mismatch"
        } else {
            "outcome_not_ready"
        },
        "ready": ready,
        "schema": schema,
        "expected_schema": expected_schema,
        "schema_matches": schema_matches,
        "outcome_pointer": outcome_pointer,
        "outcome": outcome,
        "expected_outcomes": ready_outcomes,
        "action": parsed.pointer("/action")
            .or_else(|| parsed.pointer("/queue/action"))
            .or_else(|| parsed.pointer("/payload_packet/payload/action"))
            .and_then(Value::as_str),
        "error": parsed.get("error").and_then(Value::as_str),
        "queue_blockers": parsed.get("queue_blockers").cloned(),
        "host_blockers": parsed.get("host_blockers").cloned(),
        "provision_required": parsed.get("provision_required").cloned(),
        "executor_pending": parsed.get("executor_pending").cloned(),
    })
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
            "safe_for": ["open_url", "screenshot", "inspect_element", "dom_snapshot", "runtime_events", "set_viewport", "wait_for_selector"],
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
                "compose and queue open_url, viewport/full-page/selector screenshot, inspect_element, dom_snapshot, runtime_events, set_viewport, and wait_for_selector payloads",
                "inspect the managed Chrome queue before any runner request",
                "write a runner-gate receipt without launching Chrome",
                "prepare the managed Playwright adapter files",
                "invoke the adapter only for allowlisted safe actions after a runner-gate receipt exists",
                "inspect execution request and receipt summaries after invocation, including navigation, screenshot, viewport, selector-wait, DOM, runtime-event, and element-inspection evidence"
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
