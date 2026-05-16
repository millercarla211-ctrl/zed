use crate::{AgentTool, ToolCallEventStream, ToolInput, ToolPermissionContext};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, ClipboardItem, Entity, SharedString, Task};
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
pub const AGENT_BROWSER_PAYLOAD_TOOL_NAME: &str = "compose_agent_browser_action_payload";
pub const AGENT_BROWSER_PAYLOAD_STAGE_TOOL_NAME: &str = "stage_agent_browser_action_payload";
pub const AGENT_BROWSER_PAYLOAD_QUEUE_TOOL_NAME: &str = "queue_agent_browser_action_payload";
const AGENT_BROWSER_PAYLOAD_QUEUE_FILE_NAME: &str = "latest-agent-browser-payload.json";
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

/// Composes a schema-versioned payload for the in-app WebPreview Browser plugin.
///
/// This tool is read-only. It does not click, type, scroll, press keys, open Chrome,
/// or bypass WebPreview permission gates. Use it to generate a payload packet that can
/// be copied/imported into WebPreview's Agent Payload Bridge, then run the relevant
/// WebPreview executor only after the user has unlocked interactive browser actions.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentBrowserPayloadToolInput {
    /// Browser action to prepare.
    pub action: AgentBrowserPayloadAction,
    /// Optional CSS selector from the latest WebPreview preflight receipt.
    pub selector: Option<String>,
    /// Text for `type_text`. Required for type actions.
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
    /// Whether a future type executor should clear existing text before inserting.
    pub clear_existing: bool,
    /// Include WebPreview handoff and safety instructions in the returned JSON.
    pub include_handoff_instructions: bool,
}

impl Default for AgentBrowserPayloadToolInput {
    fn default() -> Self {
        Self {
            action: AgentBrowserPayloadAction::TypeText,
            selector: None,
            text: None,
            key: None,
            modifiers: Vec::new(),
            button: "left".to_string(),
            click_count: 1,
            delta_x: 0,
            delta_y: -480,
            clear_existing: false,
            include_handoff_instructions: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentBrowserPayloadAction {
    Click,
    #[default]
    TypeText,
    PressKey,
    Scroll,
}

pub struct AgentBrowserPayloadTool;

impl AgentTool for AgentBrowserPayloadTool {
    type Input = AgentBrowserPayloadToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_BROWSER_PAYLOAD_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input.map(|input| input.action) {
            Ok(AgentBrowserPayloadAction::Click) => "Compose browser click payload".into(),
            Ok(AgentBrowserPayloadAction::TypeText) => "Compose browser type payload".into(),
            Ok(AgentBrowserPayloadAction::PressKey) => "Compose browser key payload".into(),
            Ok(AgentBrowserPayloadAction::Scroll) => "Compose browser scroll payload".into(),
            Err(_) => "Compose browser action payload".into(),
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
            let result = compose_agent_browser_payload(&input);
            let valid = result
                .pointer("/result/valid")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let output = serde_json::to_string_pretty(&result)
                .map_err(|error| format!("Failed to serialize browser payload: {error}"))?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(if valid {
                "Composed browser action payload"
            } else {
                "Browser action payload needs fixes"
            }));

            if valid { Ok(output) } else { Err(output) }
        })
    }
}

/// Stages a validated WebPreview Browser payload packet onto the clipboard.
///
/// This tool does not import the payload into WebPreview or execute browser input. It only writes
/// the validated packet returned by `compose_agent_browser_action_payload` to the clipboard after
/// explicit authorization, so the user or Agent can invoke WebPreview's payload import action next.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentBrowserPayloadStageToolInput {
    /// The browser payload to compose and stage.
    pub payload: AgentBrowserPayloadToolInput,
}

impl Default for AgentBrowserPayloadStageToolInput {
    fn default() -> Self {
        Self {
            payload: AgentBrowserPayloadToolInput {
                include_handoff_instructions: false,
                ..Default::default()
            },
        }
    }
}

pub struct AgentBrowserPayloadStageTool;

impl AgentTool for AgentBrowserPayloadStageTool {
    type Input = AgentBrowserPayloadStageToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_BROWSER_PAYLOAD_STAGE_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input.map(|input| input.payload.action) {
            Ok(AgentBrowserPayloadAction::Click) => "Stage browser click payload".into(),
            Ok(AgentBrowserPayloadAction::TypeText) => "Stage browser type payload".into(),
            Ok(AgentBrowserPayloadAction::PressKey) => "Stage browser key payload".into(),
            Ok(AgentBrowserPayloadAction::Scroll) => "Stage browser scroll payload".into(),
            Err(_) => "Stage browser action payload".into(),
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
            let result = compose_agent_browser_payload(&input.payload);
            let valid = result
                .pointer("/result/valid")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !valid {
                let output = serde_json::to_string_pretty(&result)
                    .map_err(|error| format!("Failed to serialize browser payload: {error}"))?;
                event_stream.update_fields(
                    acp::ToolCallUpdateFields::new().title("Browser payload needs fixes"),
                );
                return Err(output);
            }

            let packet = result
                .get("payload_packet")
                .cloned()
                .ok_or_else(|| "Missing payload_packet in composed browser payload".to_string())?;
            let packet_json = serde_json::to_string_pretty(&packet)
                .map_err(|error| format!("Failed to serialize browser payload packet: {error}"))?;

            let context = ToolPermissionContext::new(
                Self::NAME,
                vec![
                    action_name(input.payload.action).to_string(),
                    format!("{} clipboard characters", packet_json.chars().count()),
                ],
            );
            let authorize = cx
                .update(|cx| {
                    event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
                })
                .map_err(|error| error.to_string())?;
            authorize.await.map_err(|error| error.to_string())?;

            cx.update(|cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(packet_json.clone()));
            })
            .map_err(|error| error.to_string())?;

            let output = serde_json::json!({
                "schema": "zed.agent_plugins.browser_action_payload_stage_result.v1",
                "result": {
                    "generated_at_ms": current_epoch_millis(),
                    "status": "staged_to_clipboard",
                    "action": action_name(input.payload.action),
                    "clipboard_characters": packet_json.chars().count(),
                    "next_step": "Use WebPreview's Import Agent Payload from Clipboard action, then run the matching permissioned executor only after unlock, fresh preflight, focus, QA, and receipt gates pass."
                },
                "payload_packet": packet,
                "safety": {
                    "tool_dispatches_browser_input": false,
                    "tool_imports_into_webpreview": false,
                    "clipboard_written_after_authorization": true,
                    "requires_webpreview_permission_gate": true,
                    "receipts_required_for_execution": true
                }
            });
            let output = serde_json::to_string_pretty(&output)
                .map_err(|error| format!("Failed to serialize staging result: {error}"))?;

            event_stream.update_fields(
                acp::ToolCallUpdateFields::new().title("Staged browser payload"),
            );

            Ok(output)
        })
    }
}

/// Queues a validated WebPreview Browser payload packet into a managed project or Zed-data file.
///
/// This is the direct handoff path between Agent tools and WebPreview. It writes only a managed
/// queue item after explicit authorization; WebPreview must still import the queue item, require
/// user permission, rerun preflight/focus gates, and emit executor receipts before any input runs.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentBrowserPayloadQueueToolInput {
    /// The browser payload to compose and queue.
    pub payload: AgentBrowserPayloadToolInput,
    /// Prefer workspace-local queue under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentBrowserPayloadQueueRootMode,
}

impl Default for AgentBrowserPayloadQueueToolInput {
    fn default() -> Self {
        Self {
            payload: AgentBrowserPayloadToolInput {
                include_handoff_instructions: false,
                ..Default::default()
            },
            root_mode: AgentBrowserPayloadQueueRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentBrowserPayloadQueueRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct AgentBrowserPayloadQueueTool {
    project: Entity<Project>,
}

impl AgentBrowserPayloadQueueTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentBrowserPayloadQueueTool {
    type Input = AgentBrowserPayloadQueueToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_BROWSER_PAYLOAD_QUEUE_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input.map(|input| input.payload.action) {
            Ok(AgentBrowserPayloadAction::Click) => "Queue browser click payload".into(),
            Ok(AgentBrowserPayloadAction::TypeText) => "Queue browser type payload".into(),
            Ok(AgentBrowserPayloadAction::PressKey) => "Queue browser key payload".into(),
            Ok(AgentBrowserPayloadAction::Scroll) => "Queue browser scroll payload".into(),
            Err(_) => "Queue browser action payload".into(),
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
            let queue = AgentBrowserPayloadQueue::new(project_root, input.root_mode);

            let result = compose_agent_browser_payload(&input.payload);
            let valid = result
                .pointer("/result/valid")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !valid {
                let output = serde_json::to_string_pretty(&result)
                    .map_err(|error| format!("Failed to serialize browser payload: {error}"))?;
                event_stream.update_fields(
                    acp::ToolCallUpdateFields::new().title("Browser payload needs fixes"),
                );
                return Err(output);
            }

            queue.validate()?;
            let packet = result
                .get("payload_packet")
                .cloned()
                .ok_or_else(|| "Missing payload_packet in composed browser payload".to_string())?;
            let queue_item = queue.queue_item(&input, packet);
            let queue_item_json = serde_json::to_vec_pretty(&queue_item)
                .map_err(|error| format!("Failed to serialize queued browser payload: {error}"))?;

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
                    "Failed to prepare browser payload queue {}: {error}",
                    queue.queue_dir.display()
                )
            })?;
            fs::write(&queue.latest_path, &queue_item_json).map_err(|error| {
                format!(
                    "Failed to write browser payload queue {}: {error}",
                    queue.latest_path.display()
                )
            })?;
            fs::write(&queue.archive_path, &queue_item_json).map_err(|error| {
                format!(
                    "Failed to archive browser payload queue item {}: {error}",
                    queue.archive_path.display()
                )
            })?;

            let output = serde_json::json!({
                "schema": "zed.agent_plugins.browser_action_payload_queue_result.v1",
                "result": {
                    "generated_at_ms": current_epoch_millis(),
                    "status": "queued_to_managed_handoff",
                    "action": action_name(input.payload.action),
                    "root_mode": queue.root_mode_label(),
                    "queue_path": path_string(&queue.queue_dir),
                    "latest_path": path_string(&queue.latest_path),
                    "archive_path": path_string(&queue.archive_path),
                    "next_step": "Use WebPreview's Import Agent Payload from Managed Queue action, then run the matching permissioned executor only after unlock, fresh preflight, focus, QA, and receipt gates pass."
                },
                "queued_item": queue_item,
                "safety": {
                    "tool_dispatches_browser_input": false,
                    "tool_imports_into_webpreview": false,
                    "managed_queue_written_after_authorization": true,
                    "requires_webpreview_import": true,
                    "requires_webpreview_permission_gate": true,
                    "receipts_required_for_execution": true
                }
            });
            let output = serde_json::to_string_pretty(&output)
                .map_err(|error| format!("Failed to serialize queue result: {error}"))?;

            event_stream.update_fields(
                acp::ToolCallUpdateFields::new().title("Queued browser payload"),
            );

            Ok(output)
        })
    }
}

struct AgentBrowserPayloadQueue {
    root_mode: AgentBrowserPayloadQueueRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    queue_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl AgentBrowserPayloadQueue {
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
        let archive_path = queue_dir.join(format!(
            "agent-browser-payload-{}.json",
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
                    "Refusing to queue browser payload at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn queue_item(&self, input: &AgentBrowserPayloadQueueToolInput, packet: Value) -> Value {
        serde_json::json!({
            "schema": "zed.agent_plugins.browser_action_payload_queue_item.v1",
            "queued_at_ms": current_epoch_millis(),
            "source_tool": AGENT_BROWSER_PAYLOAD_QUEUE_TOOL_NAME,
            "payload_packet": packet,
            "metadata": {
                "action": action_name(input.payload.action),
                "root_mode": self.root_mode_label(),
                "project_root": self.project_root.as_ref().map(path_string),
                "queue_path": path_string(&self.queue_dir),
                "latest_path": path_string(&self.latest_path),
                "archive_path": path_string(&self.archive_path),
                "requires_webpreview_import": true,
                "requires_interactive_unlock": true,
                "requires_fresh_preflight": true,
                "requires_receipt_after_execution": true,
            },
            "safety": {
                "browser_input_dispatched": false,
                "webpreview_imported": false,
                "external_chrome_touched": false,
                "real_browser_profiles_touched": false,
            }
        })
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

fn compose_agent_browser_payload(input: &AgentBrowserPayloadToolInput) -> Value {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let payload = match input.action {
        AgentBrowserPayloadAction::Click => click_payload(input, &mut blockers, &mut warnings),
        AgentBrowserPayloadAction::TypeText => {
            type_text_payload(input, &mut blockers, &mut warnings)
        }
        AgentBrowserPayloadAction::PressKey => {
            press_key_payload(input, &mut blockers, &mut warnings)
        }
        AgentBrowserPayloadAction::Scroll => scroll_payload(input, &mut blockers, &mut warnings),
    };
    let valid = blockers.is_empty();

    serde_json::json!({
        "schema": "zed.agent_plugins.browser_action_payload_result.v1",
        "result": {
            "generated_at_ms": current_epoch_millis(),
            "valid": valid,
            "status": if valid { "payload_ready" } else { "payload_blocked" },
            "action": action_name(input.action),
            "blockers": blockers,
            "warnings": warnings,
            "next_step": if valid {
                "Import this packet through WebPreview's Agent Payload Bridge, then run the matching permissioned executor after unlock, fresh preflight, focus, QA, and receipt gates pass."
            } else {
                "Fix the blockers, compose the payload again, then import it into WebPreview only after the user intends to control the browser."
            }
        },
        "payload_packet": {
            "schema": "zed.web_preview.agent_browser_executor_payload.v1",
            "payload": payload,
        },
        "handoff": input.include_handoff_instructions.then(|| serde_json::json!({
            "target_plugin": "zed.browser",
            "webpreview_bridge_schema": "zed.web_preview.agent_browser_action_payload_bridge.v1",
            "webpreview_import_action": "import_agent_browser_action_payload_from_clipboard",
            "webpreview_executor_actions": {
                "click": "run_permissioned_native_click_executor",
                "type_text": "run_permissioned_native_type_executor",
                "press_key": "run_permissioned_native_key_executor",
                "scroll": "run_permissioned_native_scroll_executor"
            },
            "required_before_execution": [
                "user explicitly unlocks interactive WebPreview browser actions",
                "fresh page diagnostics, DOM snapshot, action targets, and readiness probe exist",
                "fresh action-specific preflight and native trace receipt exist",
                "native dispatch QA checklist exists",
                "type_text and press_key also satisfy the WebPreview keyboard-focus gate"
            ]
        })),
        "safety": {
            "tool_dispatches_browser_input": false,
            "requires_webpreview_import": true,
            "requires_webpreview_permission_gate": true,
            "real_browser_profiles_touched": false,
            "external_chrome_controlled": false,
            "receipts_required_for_execution": true
        }
    })
}

fn click_payload(
    input: &AgentBrowserPayloadToolInput,
    blockers: &mut Vec<Value>,
    warnings: &mut Vec<Value>,
) -> Value {
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
    if input.selector.as_deref().is_none_or(str::is_empty) {
        warnings.push(warning(
            "selector_missing",
            "WebPreview will fall back to the latest click preflight target if no selector is supplied.",
        ));
    }

    serde_json::json!({
        "action": "click",
        "selector": input.selector,
        "button": input.button,
        "click_count": input.click_count,
    })
}

fn type_text_payload(
    input: &AgentBrowserPayloadToolInput,
    blockers: &mut Vec<Value>,
    warnings: &mut Vec<Value>,
) -> Value {
    let text = input.text.clone().unwrap_or_default();
    if text.trim().is_empty() {
        blockers.push(blocker(
            "type_text_missing",
            "Type payloads require non-empty text.",
            "Set text to the exact string the agent should insert.",
        ));
    }
    if text.chars().count() > MAX_TYPE_TEXT_CHARS {
        blockers.push(blocker(
            "type_text_too_large",
            "Type payload text exceeds the bounded WebPreview native type executor limit.",
            "Split text into chunks of 4096 characters or fewer.",
        ));
    }
    if input.clear_existing {
        blockers.push(blocker(
            "clear_existing_not_supported",
            "The current WebPreview native type executor inserts explicit text but does not clear existing input first.",
            "Set clear_existing=false, or clear the field with a separate explicitly validated browser action before typing.",
        ));
    }
    if input.selector.as_deref().is_none_or(str::is_empty) {
        warnings.push(warning(
            "selector_missing",
            "WebPreview will require the current focused type target; supplying a selector helps detect stale focus.",
        ));
    }

    serde_json::json!({
        "action": "type_text",
        "selector": input.selector,
        "text": text,
        "clear_existing": input.clear_existing,
    })
}

fn press_key_payload(
    input: &AgentBrowserPayloadToolInput,
    blockers: &mut Vec<Value>,
    _warnings: &mut Vec<Value>,
) -> Value {
    let key = input.key.clone().unwrap_or_else(|| "Escape".to_string());
    if !ALLOWED_KEYS.contains(&key.as_str()) {
        blockers.push(blocker(
            "unsupported_key",
            "Key payloads are restricted to the WebPreview native key executor allowlist.",
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
        "modifiers": input.modifiers,
    })
}

fn scroll_payload(
    input: &AgentBrowserPayloadToolInput,
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
            "WebPreview will use the latest scroll preflight target or page-level scroll plan when no selector is supplied.",
        ));
    }

    serde_json::json!({
        "action": "scroll",
        "selector": input.selector,
        "delta_x": input.delta_x,
        "delta_y": input.delta_y,
    })
}

fn action_name(action: AgentBrowserPayloadAction) -> &'static str {
    match action {
        AgentBrowserPayloadAction::Click => "click",
        AgentBrowserPayloadAction::TypeText => "type_text",
        AgentBrowserPayloadAction::PressKey => "press_key",
        AgentBrowserPayloadAction::Scroll => "scroll",
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
