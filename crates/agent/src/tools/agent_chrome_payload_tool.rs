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
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_TYPE_TEXT_CHARS: usize = 4096;
const DEFAULT_WAIT_TIMEOUT_MS: u64 = 5_000;
const MAX_WAIT_TIMEOUT_MS: u64 = 30_000;
const MIN_VIEWPORT_WIDTH: u32 = 320;
const MAX_VIEWPORT_WIDTH: u32 = 3840;
const MIN_VIEWPORT_HEIGHT: u32 = 240;
const MAX_VIEWPORT_HEIGHT: u32 = 2160;
const MIN_DEVICE_SCALE_FACTOR: f32 = 0.5;
const MAX_DEVICE_SCALE_FACTOR: f32 = 4.0;

pub const AGENT_CHROME_PAYLOAD_TOOL_NAME: &str = "compose_managed_chrome_action_payload";
pub const AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME: &str = "queue_managed_chrome_action_payload";
pub const AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA: &str =
    "zed.agent_plugins.managed_chrome_executor_payload.v1";
pub const AGENT_CHROME_PAYLOAD_RESULT_SCHEMA: &str =
    "zed.agent_plugins.managed_chrome_payload_result.v1";
pub const AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA: &str =
    "zed.agent_plugins.managed_chrome_payload_queue_item.v1";
pub const AGENT_CHROME_PAYLOAD_QUEUE_RESULT_SCHEMA: &str =
    "zed.agent_plugins.managed_chrome_payload_queue_result.v1";
pub const AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME: &str = "latest-managed-chrome-payload.json";

const ALLOWED_KEYS: &[&str] = &[
    "Escape",
    "Enter",
    "Tab",
    "ArrowDown",
    "ArrowUp",
    "ArrowLeft",
    "ArrowRight",
    "Backspace",
    "Delete",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Space",
];
const ALLOWED_MODIFIERS: &[&str] = &["Alt", "Control", "Meta", "Shift"];
const ALLOWED_BUTTONS: &[&str] = &["left", "middle", "right"];

/// Composes a schema-versioned payload for a future managed Chrome/Playwright executor.
///
/// This tool is read-only. It does not launch Chrome, install Playwright, dispatch browser
/// input, run page scripts, or write to a real browser profile. Use it to generate a validated
/// packet that can be queued into the managed Chrome handoff path after explicit authorization.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentChromePayloadToolInput {
    /// Managed Chrome action to prepare.
    pub action: AgentChromePayloadAction,
    /// URL for `open_url`.
    pub url: Option<String>,
    /// CSS selector for element-scoped actions.
    pub selector: Option<String>,
    /// Text for `type_text`.
    pub text: Option<String>,
    /// Key for `press_key`. Defaults to Escape.
    pub key: Option<String>,
    /// Modifier keys for `press_key`.
    pub modifiers: Vec<String>,
    /// Mouse button for `click`.
    pub button: String,
    /// Click count for `click`; bounded to 1 through 3.
    pub click_count: u8,
    /// Horizontal wheel delta for `scroll`.
    pub delta_x: i32,
    /// Vertical wheel delta for `scroll`.
    pub delta_y: i32,
    /// Timeout for wait-style actions.
    pub timeout_ms: u64,
    /// Responsive viewport width for `set_viewport`.
    pub viewport_width: Option<u32>,
    /// Responsive viewport height for `set_viewport`.
    pub viewport_height: Option<u32>,
    /// Device scale factor for `set_viewport`.
    pub device_scale_factor: Option<f32>,
    /// Capture the full page for `screenshot`.
    pub full_page: bool,
    /// Include handoff and safety instructions in the returned JSON.
    pub include_handoff_instructions: bool,
}

impl Default for AgentChromePayloadToolInput {
    fn default() -> Self {
        Self {
            action: AgentChromePayloadAction::OpenUrl,
            url: None,
            selector: None,
            text: None,
            key: None,
            modifiers: Vec::new(),
            button: "left".to_string(),
            click_count: 1,
            delta_x: 0,
            delta_y: -480,
            timeout_ms: DEFAULT_WAIT_TIMEOUT_MS,
            viewport_width: Some(1440),
            viewport_height: Some(900),
            device_scale_factor: Some(1.0),
            full_page: false,
            include_handoff_instructions: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentChromePayloadAction {
    #[default]
    OpenUrl,
    Click,
    TypeText,
    PressKey,
    Scroll,
    Screenshot,
    InspectElement,
    DomSnapshot,
    RuntimeEvents,
    WaitForSelector,
    SetViewport,
}

pub struct AgentChromePayloadTool;

impl AgentTool for AgentChromePayloadTool {
    type Input = AgentChromePayloadToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_CHROME_PAYLOAD_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input.map(|input| input.action) {
            Ok(AgentChromePayloadAction::OpenUrl) => "Compose Chrome open URL payload".into(),
            Ok(AgentChromePayloadAction::Click) => "Compose Chrome click payload".into(),
            Ok(AgentChromePayloadAction::TypeText) => "Compose Chrome type payload".into(),
            Ok(AgentChromePayloadAction::PressKey) => "Compose Chrome key payload".into(),
            Ok(AgentChromePayloadAction::Scroll) => "Compose Chrome scroll payload".into(),
            Ok(AgentChromePayloadAction::Screenshot) => "Compose Chrome screenshot payload".into(),
            Ok(AgentChromePayloadAction::InspectElement) => {
                "Compose Chrome inspect element payload".into()
            }
            Ok(AgentChromePayloadAction::DomSnapshot) => {
                "Compose Chrome DOM snapshot payload".into()
            }
            Ok(AgentChromePayloadAction::RuntimeEvents) => {
                "Compose Chrome runtime events payload".into()
            }
            Ok(AgentChromePayloadAction::WaitForSelector) => "Compose Chrome wait payload".into(),
            Ok(AgentChromePayloadAction::SetViewport) => "Compose Chrome viewport payload".into(),
            Err(_) => "Compose Chrome action payload".into(),
        }
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |_cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let result = compose_managed_chrome_payload(&input);
            let valid = result
                .pointer("/result/valid")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let output = serde_json::to_string_pretty(&result)
                .map_err(|error| format!("Failed to serialize Chrome payload: {error}"))?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(if valid {
                "Composed Chrome action payload"
            } else {
                "Chrome action payload needs fixes"
            }));

            if valid { Ok(output) } else { Err(output) }
        })
    }
}

/// Queues a validated managed Chrome payload packet into a managed project or Zed-data file.
///
/// This is a safe handoff layer only. It writes a queue item after explicit authorization; a
/// future runner must still use managed profiles, permission gates, fresh preflight, and receipts
/// before launching or controlling Chrome.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentChromePayloadQueueToolInput {
    /// The managed Chrome payload to compose and queue.
    pub payload: AgentChromePayloadToolInput,
    /// Prefer workspace-local queue under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentChromePayloadQueueRootMode,
}

impl Default for AgentChromePayloadQueueToolInput {
    fn default() -> Self {
        Self {
            payload: AgentChromePayloadToolInput {
                include_handoff_instructions: false,
                ..Default::default()
            },
            root_mode: AgentChromePayloadQueueRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentChromePayloadQueueRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct AgentChromePayloadQueueTool {
    project: Entity<Project>,
}

impl AgentChromePayloadQueueTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentChromePayloadQueueTool {
    type Input = AgentChromePayloadQueueToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input.map(|input| input.payload.action) {
            Ok(AgentChromePayloadAction::OpenUrl) => "Queue Chrome open URL payload".into(),
            Ok(AgentChromePayloadAction::Click) => "Queue Chrome click payload".into(),
            Ok(AgentChromePayloadAction::TypeText) => "Queue Chrome type payload".into(),
            Ok(AgentChromePayloadAction::PressKey) => "Queue Chrome key payload".into(),
            Ok(AgentChromePayloadAction::Scroll) => "Queue Chrome scroll payload".into(),
            Ok(AgentChromePayloadAction::Screenshot) => "Queue Chrome screenshot payload".into(),
            Ok(AgentChromePayloadAction::InspectElement) => {
                "Queue Chrome inspect element payload".into()
            }
            Ok(AgentChromePayloadAction::DomSnapshot) => "Queue Chrome DOM snapshot payload".into(),
            Ok(AgentChromePayloadAction::RuntimeEvents) => {
                "Queue Chrome runtime events payload".into()
            }
            Ok(AgentChromePayloadAction::WaitForSelector) => "Queue Chrome wait payload".into(),
            Ok(AgentChromePayloadAction::SetViewport) => "Queue Chrome viewport payload".into(),
            Err(_) => "Queue Chrome action payload".into(),
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
            let project_root = cx.update(|cx| workspace_root_for_project(&self.project, cx));
            let queue = AgentChromePayloadQueue::new(project_root, input.root_mode);

            let result = compose_managed_chrome_payload(&input.payload);
            let valid = result
                .pointer("/result/valid")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !valid {
                let output = serde_json::to_string_pretty(&result)
                    .map_err(|error| format!("Failed to serialize Chrome payload: {error}"))?;
                event_stream.update_fields(
                    acp::ToolCallUpdateFields::new().title("Chrome payload needs fixes"),
                );
                return Err(output);
            }

            queue.validate()?;
            let packet = result
                .get("payload_packet")
                .cloned()
                .ok_or_else(|| "Missing payload_packet in composed Chrome payload".to_string())?;
            let queue_item = queue.queue_item(&input, packet);
            let queue_item_json = serde_json::to_vec_pretty(&queue_item)
                .map_err(|error| format!("Failed to serialize queued Chrome payload: {error}"))?;

            let context = ToolPermissionContext::new(
                Self::NAME,
                vec![
                    action_name(input.payload.action).to_string(),
                    path_string(&queue.queue_dir),
                    path_string(&queue.latest_path),
                    format!("{} queued bytes", queue_item_json.len()),
                ],
            );
            let authorize = cx
                .update(|cx| {
                    event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
                })
                .map_err(|error| error.to_string())?;
            authorize.await.map_err(|error| error.to_string())?;

            fs::create_dir_all(&queue.queue_dir).map_err(|error| {
                format!(
                    "Failed to prepare Chrome payload queue {}: {error}",
                    queue.queue_dir.display()
                )
            })?;
            fs::write(&queue.latest_path, &queue_item_json).map_err(|error| {
                format!(
                    "Failed to write Chrome payload queue {}: {error}",
                    queue.latest_path.display()
                )
            })?;
            fs::write(&queue.archive_path, &queue_item_json).map_err(|error| {
                format!(
                    "Failed to archive Chrome payload queue item {}: {error}",
                    queue.archive_path.display()
                )
            })?;

            let output = serde_json::json!({
                "schema": AGENT_CHROME_PAYLOAD_QUEUE_RESULT_SCHEMA,
                "result": {
                    "generated_at_ms": current_epoch_millis(),
                    "status": "queued_to_managed_chrome_handoff",
                    "action": action_name(input.payload.action),
                    "root_mode": queue.root_mode_label(),
                    "queue_path": path_string(&queue.queue_dir),
                    "latest_path": path_string(&queue.latest_path),
                    "archive_path": path_string(&queue.archive_path),
                    "next_step": "Run the future managed Chrome/Playwright executor only after runtime bootstrap, managed-profile readiness, explicit permission, fresh preflight, and receipt gates pass."
                },
                "queued_item": queue_item,
                "safety": {
                    "tool_launches_chrome": false,
                    "tool_installs_playwright": false,
                    "tool_dispatches_browser_input": false,
                    "managed_queue_written_after_authorization": true,
                    "requires_managed_chrome_runner": true,
                    "requires_explicit_permission_for_execution": true,
                    "receipts_required_for_execution": true
                }
            });
            let output = serde_json::to_string_pretty(&output)
                .map_err(|error| format!("Failed to serialize Chrome queue result: {error}"))?;

            event_stream
                .update_fields(acp::ToolCallUpdateFields::new().title("Queued Chrome payload"));

            Ok(output)
        })
    }
}

struct AgentChromePayloadQueue {
    root_mode: AgentChromePayloadQueueRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    queue_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl AgentChromePayloadQueue {
    fn new(project_root: Option<PathBuf>, root_mode: AgentChromePayloadQueueRootMode) -> Self {
        let use_workspace = matches!(root_mode, AgentChromePayloadQueueRootMode::Workspace)
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
            allowed_root.join("agent-plugins").join("chrome-payloads")
        } else {
            allowed_root.join("chrome-payloads")
        };
        let latest_path = queue_dir.join(AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME);
        let archive_path = queue_dir.join(format!(
            "managed-chrome-payload-{}.json",
            current_epoch_millis()
        ));

        Self {
            root_mode,
            project_root,
            allowed_root,
            queue_dir,
            latest_path,
            archive_path,
        }
    }

    fn validate(&self) -> Result<(), String> {
        for path in [&self.queue_dir, &self.latest_path, &self.archive_path] {
            if !path.starts_with(&self.allowed_root) {
                return Err(format!(
                    "Refusing to queue Chrome payload at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn queue_item(&self, input: &AgentChromePayloadQueueToolInput, packet: Value) -> Value {
        serde_json::json!({
            "schema": AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA,
            "queued_at_ms": current_epoch_millis(),
            "source_tool": AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME,
            "payload_packet": packet,
            "metadata": {
                "action": action_name(input.payload.action),
                "root_mode": self.root_mode_label(),
                "project_root": self.project_root.as_ref().map(path_string),
                "queue_path": path_string(&self.queue_dir),
                "latest_path": path_string(&self.latest_path),
                "archive_path": path_string(&self.archive_path),
                "requires_runtime_bootstrap": true,
                "requires_managed_chrome_profile": true,
                "requires_explicit_permission": true,
                "requires_fresh_preflight": true,
                "requires_receipt_after_execution": true,
            },
            "safety": {
                "chrome_launched": false,
                "playwright_installed": false,
                "browser_input_dispatched": false,
                "page_scripts_executed": false,
                "real_browser_profiles_touched": false,
                "managed_profile_only": true,
            }
        })
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

fn compose_managed_chrome_payload(input: &AgentChromePayloadToolInput) -> Value {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let payload = match input.action {
        AgentChromePayloadAction::OpenUrl => open_url_payload(input, &mut blockers),
        AgentChromePayloadAction::Click => click_payload(input, &mut blockers, &mut warnings),
        AgentChromePayloadAction::TypeText => type_text_payload(input, &mut blockers),
        AgentChromePayloadAction::PressKey => press_key_payload(input, &mut blockers),
        AgentChromePayloadAction::Scroll => scroll_payload(input, &mut blockers, &mut warnings),
        AgentChromePayloadAction::Screenshot => screenshot_payload(input, &mut warnings),
        AgentChromePayloadAction::InspectElement => inspect_element_payload(input, &mut blockers),
        AgentChromePayloadAction::DomSnapshot => {
            dom_snapshot_payload(input, &mut blockers, &mut warnings)
        }
        AgentChromePayloadAction::RuntimeEvents => runtime_events_payload(input, &mut blockers),
        AgentChromePayloadAction::WaitForSelector => {
            wait_for_selector_payload(input, &mut blockers)
        }
        AgentChromePayloadAction::SetViewport => set_viewport_payload(input, &mut blockers),
    };
    let valid = blockers.is_empty();

    serde_json::json!({
        "schema": AGENT_CHROME_PAYLOAD_RESULT_SCHEMA,
        "result": {
            "generated_at_ms": current_epoch_millis(),
            "valid": valid,
            "status": if valid { "payload_ready" } else { "payload_blocked" },
            "action": action_name(input.action),
            "blockers": blockers,
            "warnings": warnings,
            "next_step": if valid {
                "Queue this packet through queue_managed_chrome_action_payload, then execute it only through a managed Chrome/Playwright runner after bootstrap, permission, preflight, and receipt gates pass."
            } else {
                "Fix the blockers, compose the payload again, and queue it only when the user intends to control the managed browser."
            }
        },
        "payload_packet": {
            "schema": AGENT_CHROME_EXECUTOR_PAYLOAD_SCHEMA,
            "payload": payload,
        },
        "handoff": input.include_handoff_instructions.then(|| serde_json::json!({
            "target_plugin": "zed.chrome",
            "queue_tool_name": AGENT_CHROME_PAYLOAD_QUEUE_TOOL_NAME,
            "queue_item_schema": AGENT_CHROME_PAYLOAD_QUEUE_ITEM_SCHEMA,
            "latest_queue_file": AGENT_CHROME_PAYLOAD_QUEUE_FILE_NAME,
            "managed_queue_roots": {
                "workspace": "<workspace>/tools/agent-plugins/chrome-payloads",
                "zed_data": "<zed-data>/agent-plugins/chrome-payloads"
            },
            "required_before_execution": [
                "prepare_agent_plugin_runtime has verified or created managed roots",
                "Chrome/Playwright runner uses only managed browser profiles",
                "user has explicitly authorized the queued action",
                "fresh selector, URL, viewport, screenshot, or inspection preflight exists when applicable",
                "runner emits a schema-versioned receipt after success, block, or failure"
            ]
        })),
        "safety": {
            "tool_dispatches_browser_input": false,
            "tool_launches_chrome": false,
            "tool_installs_playwright": false,
            "tool_runs_page_scripts": false,
            "real_browser_profiles_touched": false,
            "managed_profile_required": true,
            "receipts_required_for_execution": true
        }
    })
}

fn open_url_payload(input: &AgentChromePayloadToolInput, blockers: &mut Vec<Value>) -> Value {
    let url = input.url.clone().unwrap_or_default();
    if url.trim().is_empty() {
        blockers.push(blocker(
            "url_missing",
            "Chrome open_url payloads require a non-empty URL.",
            "Set url to the exact http, https, file, or local development URL to open.",
        ));
    } else if !is_supported_url(&url) {
        blockers.push(blocker(
            "unsupported_url_scheme",
            "Chrome open_url payloads only support http, https, file, localhost, 127.0.0.1, and ::1 targets.",
            "Use a supported URL scheme or local development host.",
        ));
    }

    serde_json::json!({
        "action": "open_url",
        "url": url,
    })
}

fn click_payload(
    input: &AgentChromePayloadToolInput,
    blockers: &mut Vec<Value>,
    warnings: &mut Vec<Value>,
) -> Value {
    require_selector(
        input,
        blockers,
        "click_selector_missing",
        "Click payloads require a selector for managed Chrome.",
    );
    if !ALLOWED_BUTTONS.contains(&input.button.as_str()) {
        blockers.push(blocker(
            "unsupported_click_button",
            "Click payloads only support left, middle, or right buttons.",
            "Use button=\"left\", \"middle\", or \"right\".",
        ));
    }
    if !(1..=3).contains(&input.click_count) {
        blockers.push(blocker(
            "unsupported_click_count",
            "Click payloads require a click_count from 1 through 3.",
            "Set click_count to 1 for normal clicks, 2 for double-click, or 3 only when explicitly needed.",
        ));
    }
    warnings.push(warning(
        "locator_preflight_required",
        "The runner must re-resolve the selector in the managed Chrome page before dispatching input.",
    ));

    serde_json::json!({
        "action": "click",
        "selector": input.selector.clone(),
        "button": input.button.clone(),
        "click_count": input.click_count,
    })
}

fn type_text_payload(input: &AgentChromePayloadToolInput, blockers: &mut Vec<Value>) -> Value {
    require_selector(
        input,
        blockers,
        "type_selector_missing",
        "Type payloads require a selector for managed Chrome.",
    );
    let text = input.text.clone().unwrap_or_default();
    if text.trim().is_empty() {
        blockers.push(blocker(
            "type_text_missing",
            "Type payloads require non-empty text.",
            "Set text to the exact string the managed Chrome runner should insert.",
        ));
    }
    if text.chars().count() > MAX_TYPE_TEXT_CHARS {
        blockers.push(blocker(
            "type_text_too_large",
            "Type payload text exceeds the managed Chrome executor limit.",
            "Split text into chunks of 4096 characters or fewer.",
        ));
    }

    serde_json::json!({
        "action": "type_text",
        "selector": input.selector.clone(),
        "text": text,
    })
}

fn press_key_payload(input: &AgentChromePayloadToolInput, blockers: &mut Vec<Value>) -> Value {
    let key = input.key.clone().unwrap_or_else(|| "Escape".to_string());
    if !ALLOWED_KEYS.contains(&key.as_str()) {
        blockers.push(blocker(
            "unsupported_key",
            "Key payloads are restricted to the managed Chrome key allowlist.",
            "Use one of Escape, Enter, Tab, ArrowDown, ArrowUp, ArrowLeft, ArrowRight, Backspace, Delete, Home, End, PageUp, PageDown, or Space.",
        ));
    }
    for modifier in &input.modifiers {
        if !ALLOWED_MODIFIERS.contains(&modifier.as_str()) {
            blockers.push(serde_json::json!({
                "code": "unsupported_key_modifier",
                "message": "Key modifier is not in the supported modifier allowlist.",
                "modifier": modifier,
                "required_action": "Use only Alt, Control, Meta, or Shift modifiers.",
            }));
        }
    }

    serde_json::json!({
        "action": "press_key",
        "key": key,
        "modifiers": input.modifiers.clone(),
    })
}

fn scroll_payload(
    input: &AgentChromePayloadToolInput,
    blockers: &mut Vec<Value>,
    warnings: &mut Vec<Value>,
) -> Value {
    if input.delta_x == 0 && input.delta_y == 0 {
        blockers.push(blocker(
            "scroll_delta_missing",
            "Scroll payloads require a non-zero delta_x or delta_y.",
            "Use a bounded wheel delta, for example delta_y=-480 to scroll down.",
        ));
    }
    if input.selector.as_deref().is_none_or(str::is_empty) {
        warnings.push(warning(
            "selector_missing",
            "The managed Chrome runner may scroll the page-level viewport when no selector is supplied.",
        ));
    }

    serde_json::json!({
        "action": "scroll",
        "selector": input.selector.clone(),
        "delta_x": input.delta_x,
        "delta_y": input.delta_y,
    })
}

fn screenshot_payload(input: &AgentChromePayloadToolInput, warnings: &mut Vec<Value>) -> Value {
    if input.selector.as_deref().is_some_and(str::is_empty) {
        warnings.push(warning(
            "empty_selector_ignored",
            "An empty screenshot selector will be ignored and the runner should capture the viewport or full page.",
        ));
    }

    serde_json::json!({
        "action": "screenshot",
        "selector": input
            .selector
            .as_ref()
            .filter(|selector| !selector.is_empty())
            .cloned(),
        "full_page": input.full_page,
    })
}

fn inspect_element_payload(
    input: &AgentChromePayloadToolInput,
    blockers: &mut Vec<Value>,
) -> Value {
    require_selector(
        input,
        blockers,
        "inspect_selector_missing",
        "Inspect element payloads require a selector for managed Chrome.",
    );
    if input.timeout_ms == 0 || input.timeout_ms > MAX_WAIT_TIMEOUT_MS {
        blockers.push(blocker(
            "unsupported_timeout",
            "Inspect element timeouts must be between 1 and 30000 milliseconds.",
            "Use a bounded timeout_ms value, usually 5000 to 10000.",
        ));
    }

    serde_json::json!({
        "action": "inspect_element",
        "selector": input.selector.clone(),
        "timeout_ms": input.timeout_ms,
    })
}

fn dom_snapshot_payload(
    input: &AgentChromePayloadToolInput,
    blockers: &mut Vec<Value>,
    warnings: &mut Vec<Value>,
) -> Value {
    if input.selector.as_deref().is_some_and(str::is_empty) {
        warnings.push(warning(
            "empty_selector_ignored",
            "An empty DOM snapshot selector will be ignored and the runner should inspect the document root.",
        ));
    }
    if input.timeout_ms == 0 || input.timeout_ms > MAX_WAIT_TIMEOUT_MS {
        blockers.push(blocker(
            "unsupported_timeout",
            "DOM snapshot timeouts must be between 1 and 30000 milliseconds.",
            "Use a bounded timeout_ms value, usually 5000 to 10000.",
        ));
    }

    serde_json::json!({
        "action": "dom_snapshot",
        "selector": input
            .selector
            .as_ref()
            .filter(|selector| !selector.is_empty())
            .cloned(),
        "timeout_ms": input.timeout_ms,
    })
}

fn runtime_events_payload(input: &AgentChromePayloadToolInput, blockers: &mut Vec<Value>) -> Value {
    if input.timeout_ms == 0 || input.timeout_ms > MAX_WAIT_TIMEOUT_MS {
        blockers.push(blocker(
            "unsupported_timeout",
            "Runtime event observation timeouts must be between 1 and 30000 milliseconds.",
            "Use a bounded timeout_ms value, usually 1000 to 5000.",
        ));
    }

    serde_json::json!({
        "action": "runtime_events",
        "timeout_ms": input.timeout_ms,
    })
}

fn wait_for_selector_payload(
    input: &AgentChromePayloadToolInput,
    blockers: &mut Vec<Value>,
) -> Value {
    require_selector(
        input,
        blockers,
        "wait_selector_missing",
        "Wait payloads require a selector for managed Chrome.",
    );
    if input.timeout_ms == 0 || input.timeout_ms > MAX_WAIT_TIMEOUT_MS {
        blockers.push(blocker(
            "unsupported_timeout",
            "Wait timeouts must be between 1 and 30000 milliseconds.",
            "Use a bounded timeout_ms value, usually 5000 to 10000.",
        ));
    }

    serde_json::json!({
        "action": "wait_for_selector",
        "selector": input.selector.clone(),
        "timeout_ms": input.timeout_ms,
    })
}

fn set_viewport_payload(input: &AgentChromePayloadToolInput, blockers: &mut Vec<Value>) -> Value {
    let width = input.viewport_width.unwrap_or(1440);
    let height = input.viewport_height.unwrap_or(900);
    let device_scale_factor = input.device_scale_factor.unwrap_or(1.0);
    if !(MIN_VIEWPORT_WIDTH..=MAX_VIEWPORT_WIDTH).contains(&width) {
        blockers.push(blocker(
            "viewport_width_out_of_range",
            "Viewport width is outside the managed Chrome responsive mode range.",
            "Use a width from 320 through 3840.",
        ));
    }
    if !(MIN_VIEWPORT_HEIGHT..=MAX_VIEWPORT_HEIGHT).contains(&height) {
        blockers.push(blocker(
            "viewport_height_out_of_range",
            "Viewport height is outside the managed Chrome responsive mode range.",
            "Use a height from 240 through 2160.",
        ));
    }
    if !(MIN_DEVICE_SCALE_FACTOR..=MAX_DEVICE_SCALE_FACTOR).contains(&device_scale_factor) {
        blockers.push(blocker(
            "device_scale_factor_out_of_range",
            "Device scale factor is outside the managed Chrome responsive mode range.",
            "Use a device_scale_factor from 0.5 through 4.0.",
        ));
    }

    serde_json::json!({
        "action": "set_viewport",
        "width": width,
        "height": height,
        "device_scale_factor": device_scale_factor,
    })
}

fn require_selector(
    input: &AgentChromePayloadToolInput,
    blockers: &mut Vec<Value>,
    code: &str,
    message: &str,
) {
    if input.selector.as_deref().is_none_or(str::is_empty) {
        blockers.push(blocker(
            code,
            message,
            "Supply a selector from a recent DOM snapshot, inspect result, or wait contract.",
        ));
    }
}

fn is_supported_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("file://")
        || lower.starts_with("localhost:")
        || lower.starts_with("127.0.0.1:")
        || lower.starts_with("[::1]:")
}

fn action_name(action: AgentChromePayloadAction) -> &'static str {
    match action {
        AgentChromePayloadAction::OpenUrl => "open_url",
        AgentChromePayloadAction::Click => "click",
        AgentChromePayloadAction::TypeText => "type_text",
        AgentChromePayloadAction::PressKey => "press_key",
        AgentChromePayloadAction::Scroll => "scroll",
        AgentChromePayloadAction::Screenshot => "screenshot",
        AgentChromePayloadAction::InspectElement => "inspect_element",
        AgentChromePayloadAction::DomSnapshot => "dom_snapshot",
        AgentChromePayloadAction::RuntimeEvents => "runtime_events",
        AgentChromePayloadAction::WaitForSelector => "wait_for_selector",
        AgentChromePayloadAction::SetViewport => "set_viewport",
    }
}

fn blocker(code: &str, message: &str, required_action: &str) -> Value {
    serde_json::json!({
        "code": code,
        "message": message,
        "required_action": required_action,
    })
}

fn warning(code: &str, message: &str) -> Value {
    serde_json::json!({
        "code": code,
        "message": message,
    })
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

fn current_epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
