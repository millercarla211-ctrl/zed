use super::agent_chrome_payload_tool::{
    AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA, AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME,
    AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA, AgentChromePayloadQueueRootMode,
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
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const AGENT_CHROME_RUNNER_GATE_TOOL_NAME: &str = "request_managed_chrome_payload_run";
pub const AGENT_CHROME_RUNNER_RECEIPT_SCHEMA: &str =
    "zed.agent_plugins.managed_chrome_runner_receipt.v1";
pub const AGENT_CHROME_RUNNER_RECEIPT_FILE_NAME: &str = "latest-managed-chrome-runner-receipt.json";

/// Opens the managed Chrome runner gate and writes an auditable receipt.
///
/// This is intentionally not the Playwright dispatcher yet. It validates the queued payload and
/// runner prerequisites, asks for permission, writes a blocked or ready receipt, and preserves the
/// invariant that no Chrome process is launched and no page input is dispatched by this slice.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentChromeRunnerGateToolInput {
    /// Prefer workspace-local queue under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentChromePayloadQueueRootMode,
    /// Include the full queued payload packet in the receipt.
    pub include_payload_packet: bool,
}

impl Default for AgentChromeRunnerGateToolInput {
    fn default() -> Self {
        Self {
            root_mode: AgentChromePayloadQueueRootMode::Workspace,
            include_payload_packet: false,
        }
    }
}

pub struct AgentChromeRunnerGateTool {
    project: Entity<Project>,
}

impl AgentChromeRunnerGateTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentChromeRunnerGateTool {
    type Input = AgentChromeRunnerGateToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_CHROME_RUNNER_GATE_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Request managed Chrome run".into()
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
            let gate = ManagedChromeRunnerGate::new(project_root, input.root_mode);
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
                .map_err(|error| format!("Failed to serialize Chrome runner receipt: {error}"))?;
            fs::create_dir_all(&gate.receipt_dir).map_err(|error| {
                format!(
                    "Failed to prepare Chrome runner receipt directory {}: {error}",
                    gate.receipt_dir.display()
                )
            })?;
            fs::write(&gate.latest_receipt_path, &receipt_json).map_err(|error| {
                format!(
                    "Failed to write Chrome runner receipt {}: {error}",
                    gate.latest_receipt_path.display()
                )
            })?;
            fs::write(&gate.archive_receipt_path, &receipt_json).map_err(|error| {
                format!(
                    "Failed to archive Chrome runner receipt {}: {error}",
                    gate.archive_receipt_path.display()
                )
            })?;

            let output = serde_json::json!({
                "schema": "zed.agent_plugins.managed_chrome_runner_gate_result.v1",
                "result": {
                    "generated_at_ms": current_epoch_millis(),
                    "status": "receipt_written",
                    "outcome": outcome.clone(),
                    "root_mode": gate.root_mode_label(),
                    "latest_receipt_path": path_string(&gate.latest_receipt_path),
                    "archive_receipt_path": path_string(&gate.archive_receipt_path),
                    "next_step": if outcome == "ready_runner_adapter_pending" {
                        "The managed queue and prerequisites are ready; next slice can enable a Playwright adapter that consumes this same receipt contract."
                    } else {
                        "Resolve the receipt blockers, then request the managed Chrome run again before any Playwright adapter can dispatch."
                    }
                },
                "receipt": receipt,
                "safety": {
                    "permission_prompted_before_receipt_write": true,
                    "launches_chrome": false,
                    "installs_playwright": false,
                    "dispatches_browser_input": false,
                    "runs_page_scripts": false,
                    "touches_real_browser_profiles": false,
                    "managed_profile_only": true,
                }
            });
            let output = serde_json::to_string_pretty(&output)
                .map_err(|error| format!("Failed to serialize Chrome run request: {error}"))?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(match outcome.as_str() {
                "ready_runner_adapter_pending" => "Chrome run gate ready",
                "blocked_missing_queue" => "Chrome run blocked: missing queue",
                "blocked_invalid_queue" => "Chrome run blocked: invalid queue",
                "blocked_missing_host_dependencies" => "Chrome run blocked: missing host deps",
                "blocked_needs_provisioning" => "Chrome run blocked: needs provisioning",
                _ => "Chrome run gate receipt written",
            }));

            Ok(output)
        })
    }
}

struct ManagedChromeRunnerGate {
    root_mode: AgentChromePayloadQueueRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    plugin_root: PathBuf,
    latest_queue_path: PathBuf,
    playwright_root: PathBuf,
    dx_extension_root: PathBuf,
    managed_profile_root: PathBuf,
    receipt_dir: PathBuf,
    latest_receipt_path: PathBuf,
    archive_receipt_path: PathBuf,
}

impl ManagedChromeRunnerGate {
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
        let latest_queue_path = plugin_root
            .join("chrome-payloads")
            .join(AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME);
        let dx_extension_root = plugin_root.join("dx-chrome-extension");
        let receipt_dir = plugin_root.join("chrome-receipts");
        let latest_receipt_path = receipt_dir.join(AGENT_CHROME_RUNNER_RECEIPT_FILE_NAME);
        let archive_receipt_path = receipt_dir.join(format!(
            "managed-chrome-runner-receipt-{}.json",
            current_epoch_millis()
        ));

        Self {
            root_mode,
            project_root,
            allowed_root,
            plugin_root,
            latest_queue_path,
            playwright_root,
            dx_extension_root,
            managed_profile_root,
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
                    "Refusing managed Chrome runner path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }
        Ok(())
    }

    fn receipt(&self, include_payload_packet: bool) -> Value {
        let queue = self.latest_queue_summary(include_payload_packet);
        let node = find_executable(&["node", "node.exe"]);
        let npm = find_executable(&["npm", "npm.cmd", "npm.exe"]);
        let browser = find_browser_executable();
        let playwright_package = self
            .playwright_root
            .join("node_modules")
            .join("playwright")
            .join("package.json");
        let dx_extension_manifest = self.dx_extension_root.join("manifest.json");

        let checks = vec![
            gate_check(
                "queue.latest_payload",
                "Latest managed Chrome payload",
                matches!(queue.state, QueueState::Ready),
                Some(self.latest_queue_path.clone()),
                if matches!(queue.state, QueueState::Missing) {
                    "queue_missing"
                } else {
                    "queue_invalid"
                },
                "Queue a valid managed Chrome payload before requesting a run.",
            ),
            gate_check(
                "host.node",
                "Node.js runtime",
                node.is_some(),
                node.clone(),
                "host_blocker",
                "A future Playwright adapter needs Node.js.",
            ),
            gate_check(
                "host.npm",
                "npm package manager",
                npm.is_some(),
                npm.clone(),
                "host_blocker",
                "Managed Playwright provisioning needs npm or a compatible npm executable.",
            ),
            gate_check(
                "host.chrome_or_edge",
                "Chrome or Edge executable",
                browser.is_some(),
                browser.clone(),
                "host_blocker",
                "Managed Chrome control needs Chrome, Edge, or Chromium on this OS.",
            ),
            gate_check(
                "asset.playwright_package",
                "Managed Playwright package",
                playwright_package.is_file(),
                Some(playwright_package.clone()),
                "provision_required",
                "Install Playwright into the managed tools root.",
            ),
            gate_check(
                "asset.dx_chrome_extension",
                "DX Chrome extension manifest",
                dx_extension_manifest.is_file(),
                Some(dx_extension_manifest.clone()),
                "provision_required",
                "Download or unpack the DX Chrome extension into the managed plugin root.",
            ),
            gate_check(
                "profile.managed_chrome",
                "Managed Chrome profile root",
                self.managed_profile_root.is_dir(),
                Some(self.managed_profile_root.clone()),
                "provision_required",
                "Create the managed Chrome profile root and never use real user browser profiles.",
            ),
        ];

        let queue_missing = gate_issues(&checks, "queue_missing");
        let queue_invalid = gate_issues(&checks, "queue_invalid");
        let host_blockers = gate_issues(&checks, "host_blocker");
        let provision_required = gate_issues(&checks, "provision_required");
        let outcome = if !queue_missing.is_empty() {
            "blocked_missing_queue"
        } else if !queue_invalid.is_empty() {
            "blocked_invalid_queue"
        } else if !host_blockers.is_empty() {
            "blocked_missing_host_dependencies"
        } else if !provision_required.is_empty() {
            "blocked_needs_provisioning"
        } else {
            "ready_runner_adapter_pending"
        };

        serde_json::json!({
            "schema": AGENT_CHROME_RUNNER_RECEIPT_SCHEMA,
            "result": {
                "generated_at_ms": current_epoch_millis(),
                "outcome": outcome,
                "root_mode": self.root_mode_label(),
                "runner_adapter_enabled": false,
                "chrome_launched": false,
                "browser_input_dispatched": false,
                "page_scripts_executed": false,
                "next_actions": runner_next_actions(outcome),
            },
            "roots": {
                "project_root": self.project_root.as_ref().map(path_string),
                "allowed_root": path_string(&self.allowed_root),
                "plugin_root": path_string(&self.plugin_root),
                "latest_queue_path": path_string(&self.latest_queue_path),
                "playwright_root": path_string(&self.playwright_root),
                "dx_chrome_extension_root": path_string(&self.dx_extension_root),
                "managed_chrome_profile_root": path_string(&self.managed_profile_root),
                "receipt_dir": path_string(&self.receipt_dir),
                "latest_receipt_path": path_string(&self.latest_receipt_path),
                "archive_receipt_path": path_string(&self.archive_receipt_path),
            },
            "queue": queue.value,
            "host": {
                "node": node.as_ref().map(path_string),
                "npm": npm.as_ref().map(path_string),
                "chrome_or_edge": browser.as_ref().map(path_string),
            },
            "checks": checks,
            "host_blockers": host_blockers,
            "provision_required": provision_required,
            "safety": {
                "permission_prompt_required": true,
                "receipt_only": true,
                "write_scope": "managed Zed data roots or workspace tools roots only",
                "launches_chrome": false,
                "installs_playwright": false,
                "dispatches_browser_input": false,
                "runs_page_scripts": false,
                "managed_profile_only": true,
                "real_browser_profiles_touched": false,
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
            .is_some_and(|schema| schema == AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA);
        let payload_schema_ok = value
            .pointer("/payload_packet/schema")
            .and_then(Value::as_str)
            .is_some_and(|schema| schema == AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA);
        let action = value
            .pointer("/payload_packet/payload/action")
            .and_then(Value::as_str);
        let action_ok = action.is_some_and(is_supported_action);
        let state = if queue_schema_ok && payload_schema_ok && action_ok {
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
            "action": action,
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
            AgentChromePayloadQueueRootMode::Workspace if self.project_root.is_some() => {
                "workspace"
            }
            AgentChromePayloadQueueRootMode::Workspace => "zed_data_fallback",
            AgentChromePayloadQueueRootMode::ZedData => "zed_data",
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

fn gate_issues(checks: &[Value], state: &str) -> Vec<Value> {
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

fn runner_next_actions(outcome: &str) -> Vec<&'static str> {
    match outcome {
        "blocked_missing_queue" => vec![
            "Compose a managed Chrome payload.",
            "Queue it with queue_managed_chrome_action_payload.",
            "Inspect the queue before requesting the run again.",
        ],
        "blocked_invalid_queue" => vec![
            "Regenerate the queue item with queue_managed_chrome_action_payload.",
            "Do not hand-edit managed Chrome queue files.",
        ],
        "blocked_missing_host_dependencies" => vec![
            "Install missing host dependencies: Node.js, npm, and Chrome/Edge/Chromium.",
            "Inspect the queue again after host dependencies are available.",
        ],
        "blocked_needs_provisioning" => vec![
            "Run prepare_agent_plugin_runtime with managed root creation enabled.",
            "Install Playwright into the managed tools root.",
            "Install or unpack the DX Chrome extension into the managed plugin root.",
            "Keep all profile data inside the managed Chrome profile root.",
        ],
        _ => vec![
            "The gate is ready for a future Playwright adapter.",
            "Next implementation should consume this receipt, launch only a managed profile, and write an execution receipt.",
        ],
    }
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
