use super::agent_chrome_payload_tool::{
    AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA, AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME,
    AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA, AgentChromePayloadQueueRootMode,
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

pub const AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME: &str =
    "inspect_managed_chrome_payload_queue";

/// Inspects the managed Chrome payload queue and runner prerequisites without launching Chrome.
///
/// This read-only tool validates the latest queued payload, managed queue location, host
/// dependencies, Playwright package, DX extension manifest, and managed profile root. It never
/// writes files, installs packages, launches browsers, dispatches input, or touches real profiles.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentChromePayloadQueueInspectToolInput {
    /// Prefer workspace-local queue under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentChromePayloadQueueRootMode,
    /// Include the full queued payload packet in the returned JSON.
    pub include_payload_packet: bool,
}

impl Default for AgentChromePayloadQueueInspectToolInput {
    fn default() -> Self {
        Self {
            root_mode: AgentChromePayloadQueueRootMode::Workspace,
            include_payload_packet: false,
        }
    }
}

pub struct AgentChromePayloadQueueInspectTool {
    project: Entity<Project>,
}

impl AgentChromePayloadQueueInspectTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentChromePayloadQueueInspectTool {
    type Input = AgentChromePayloadQueueInspectToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_CHROME_PAYLOAD_QUEUE_INSPECT_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Inspect Chrome payload queue".into()
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
            let inspector = ManagedChromePayloadQueueInspector::new(project_root, input.root_mode);
            let result = inspector.inspect(input.include_payload_packet);
            let status = result
                .pointer("/result/status")
                .and_then(Value::as_str)
                .unwrap_or("inspected");
            let output = serde_json::to_string_pretty(&result)
                .map_err(|error| format!("Failed to serialize Chrome queue inspection: {error}"))?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(match status {
                "ready_for_permissioned_runner" => "Chrome queue ready for runner gate",
                "missing_queue" => "Chrome payload queue is empty",
                "invalid_queue" => "Chrome payload queue needs fixes",
                "blocked_missing_host_dependencies" => "Chrome runner missing host dependencies",
                "ready_to_provision" => "Chrome runner needs managed assets",
                _ => "Inspected Chrome payload queue",
            }));

            Ok(output)
        })
    }
}

struct ManagedChromePayloadQueueInspector {
    root_mode: AgentChromePayloadQueueRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    plugin_root: PathBuf,
    queue_dir: PathBuf,
    latest_path: PathBuf,
    playwright_root: PathBuf,
    dx_extension_root: PathBuf,
    managed_profile_root: PathBuf,
}

impl ManagedChromePayloadQueueInspector {
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
        let queue_dir = plugin_root.join("chrome-payloads");
        let latest_path = queue_dir.join(AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME);
        let dx_extension_root = plugin_root.join("dx-chrome-extension");

        Self {
            root_mode,
            project_root,
            allowed_root,
            plugin_root,
            queue_dir,
            latest_path,
            playwright_root,
            dx_extension_root,
            managed_profile_root,
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
                    "details": "Latest queue path is outside the managed root."
                }),
            }
        };

        let node = find_executable(&["node", "node.exe"]);
        let npm = find_executable(&["npm", "npm.cmd", "npm.exe"]);
        let browser = find_browser_executable();
        let playwright_package = self
            .playwright_root
            .join("node_modules")
            .join("playwright")
            .join("package.json");
        let dx_extension_manifest = self.dx_extension_root.join("manifest.json");
        let runner_receipt_dir = self.plugin_root.join("chrome-receipts");

        let checks = vec![
            readiness_check(
                "queue.latest_payload",
                "Latest managed Chrome payload",
                matches!(queue.state, QueueState::Ready),
                Some(self.latest_path.clone()),
                if matches!(queue.state, QueueState::Missing) {
                    "queue_missing"
                } else {
                    "queue_invalid"
                },
                "Queue a payload with queue_managed_chrome_action_payload before runner execution.",
            ),
            readiness_check(
                "host.node",
                "Node.js runtime",
                node.is_some(),
                node.clone(),
                "host_blocker",
                "The managed Chrome runner needs Node.js to execute Playwright.",
            ),
            readiness_check(
                "host.npm",
                "npm package manager",
                npm.is_some(),
                npm.clone(),
                "host_blocker",
                "Provisioning Playwright needs npm or a compatible npm executable.",
            ),
            readiness_check(
                "host.chrome_or_edge",
                "Chrome or Edge executable",
                browser.is_some(),
                browser.clone(),
                "host_blocker",
                "External managed Chrome control needs Chrome, Edge, or Chromium on this OS.",
            ),
            readiness_check(
                "asset.playwright_package",
                "Managed Playwright package",
                playwright_package.is_file(),
                Some(playwright_package.clone()),
                "provision_required",
                "Install Playwright into the managed tools root before runner execution.",
            ),
            readiness_check(
                "asset.dx_chrome_extension",
                "DX Chrome extension manifest",
                dx_extension_manifest.is_file(),
                Some(dx_extension_manifest.clone()),
                "provision_required",
                "Download or unpack the DX Chrome extension before extension-backed Chrome control.",
            ),
            readiness_check(
                "profile.managed_chrome",
                "Managed Chrome profile root",
                self.managed_profile_root.is_dir(),
                Some(self.managed_profile_root.clone()),
                "provision_required",
                "Create the managed Chrome profile root; never use a user's real browser profile.",
            ),
        ];

        let queue_missing = readiness_issues(&checks, "queue_missing");
        let queue_invalid = readiness_issues(&checks, "queue_invalid");
        let host_blockers = readiness_issues(&checks, "host_blocker");
        let provision_required = readiness_issues(&checks, "provision_required");
        let status = if !queue_missing.is_empty() {
            "missing_queue"
        } else if !queue_invalid.is_empty() {
            "invalid_queue"
        } else if !host_blockers.is_empty() {
            "blocked_missing_host_dependencies"
        } else if !provision_required.is_empty() {
            "ready_to_provision"
        } else {
            "ready_for_permissioned_runner"
        };

        serde_json::json!({
            "schema": "zed.agent_plugins.managed_chrome_payload_queue_inspection.v1",
            "result": {
                "generated_at_ms": current_epoch_millis(),
                "status": status,
                "root_mode": self.root_mode_label(),
                "queue_path_valid": queue_path_valid,
                "ready_for_runner": status == "ready_for_permissioned_runner",
                "next_actions": next_actions(status),
            },
            "roots": {
                "project_root": self.project_root.as_ref().map(path_string),
                "allowed_root": path_string(&self.allowed_root),
                "plugin_root": path_string(&self.plugin_root),
                "queue_dir": path_string(&self.queue_dir),
                "latest_queue_path": path_string(&self.latest_path),
                "playwright_root": path_string(&self.playwright_root),
                "dx_chrome_extension_root": path_string(&self.dx_extension_root),
                "managed_chrome_profile_root": path_string(&self.managed_profile_root),
                "runner_receipt_dir": path_string(&runner_receipt_dir),
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
                "read_only": true,
                "writes_files": false,
                "launches_chrome": false,
                "installs_playwright": false,
                "dispatches_browser_input": false,
                "runs_page_scripts": false,
                "managed_profile_only": true,
                "real_browser_profiles_touched": false,
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
                        "details": "No managed Chrome payload has been queued yet."
                    }),
                };
            }
            Err(error) => {
                return QueueSummary {
                    state: QueueState::Invalid,
                    value: serde_json::json!({
                        "state": "read_error",
                        "path": path_string(&self.latest_path),
                        "details": error.to_string()
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
                        "path": path_string(&self.latest_path),
                        "bytes": bytes.len(),
                        "details": error.to_string()
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
            "path": path_string(&self.latest_path),
            "bytes": bytes.len(),
            "queue_schema_ok": queue_schema_ok,
            "payload_schema_ok": payload_schema_ok,
            "action_ok": action_ok,
            "action": action,
            "queued_at_ms": value.get("queued_at_ms").cloned(),
            "source_tool": value.get("source_tool").cloned(),
            "metadata": value.get("metadata").cloned(),
            "required_schema": {
                "queue_item": AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA,
                "executor_payload": AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA,
            }
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
    InvalidPath,
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

fn readiness_issues(checks: &[Value], state: &str) -> Vec<Value> {
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

fn next_actions(status: &str) -> Vec<&'static str> {
    match status {
        "missing_queue" => vec![
            "Use compose_managed_chrome_action_payload to create a valid payload.",
            "Use queue_managed_chrome_action_payload to write the payload into the managed queue.",
        ],
        "invalid_queue" => vec![
            "Regenerate the queue item with queue_managed_chrome_action_payload.",
            "Do not hand-edit managed Chrome queue files.",
        ],
        "blocked_missing_host_dependencies" => vec![
            "Install missing host dependencies first: Node.js, npm, and Chrome/Edge/Chromium.",
            "Re-run inspect_managed_chrome_payload_queue before runner execution.",
        ],
        "ready_to_provision" => vec![
            "Run prepare_agent_plugin_runtime with create_managed_roots=true and write_bootstrap_manifest=true.",
            "Install Playwright into the managed tools root.",
            "Download or unpack the DX Chrome extension into the managed agent plugin root.",
            "Keep Chrome profile data in the prepared managed profile root.",
        ],
        _ => vec![
            "The latest queue item and managed runner prerequisites are ready.",
            "Next slice can add a permission-gated managed Chrome runner that emits receipts.",
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
