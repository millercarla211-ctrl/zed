use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, SharedString, Task};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const AGENT_PC_USE_PAYLOAD_TOOL_NAME: &str = "compose_zed_pc_use_action_payload";
pub const AGENT_PC_USE_PAYLOAD_SCHEMA: &str = "zed.agent_plugins.pc_use.action_payload.v1";
const MAX_PC_USE_TEXT_CHARS: usize = 4096;
const ALLOWED_BUTTONS: &[&str] = &["left", "middle", "right"];
const ALLOWED_SURFACES: &[&str] = &[
    "workspace",
    "editor",
    "web_preview",
    "agent_panel",
    "right_panel",
    "project_panel",
    "terminal",
];

/// Composes a schema-versioned payload for future Zed-window PC-use actions.
///
/// This tool is read-only. It does not take screenshots, focus panes, click, type, execute shell
/// commands, launch processes, or control the OS desktop. Use it to validate a future PC-use intent
/// before any permissioned Zed-window executor exists.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPcUsePayloadToolInput {
    /// PC-use action to prepare.
    pub action: AgentPcUsePayloadAction,
    /// Zed surface to target. OS-wide desktop targeting is intentionally unsupported.
    pub surface: String,
    /// Optional stable target id from a future UI inspection receipt.
    pub target_id: Option<String>,
    /// Optional human-readable target label from a future UI inspection receipt.
    pub target_label: Option<String>,
    /// Text for `type_text`.
    pub text: Option<String>,
    /// Mouse button for `click`.
    pub button: String,
    /// Click count for `click`; bounded to 1 through 3.
    pub click_count: u8,
    /// Include handoff and safety instructions in the returned JSON.
    pub include_handoff_instructions: bool,
}

impl Default for AgentPcUsePayloadToolInput {
    fn default() -> Self {
        Self {
            action: AgentPcUsePayloadAction::InspectUi,
            surface: "workspace".to_string(),
            target_id: None,
            target_label: None,
            text: None,
            button: "left".to_string(),
            click_count: 1,
            include_handoff_instructions: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentPcUsePayloadAction {
    Screenshot,
    Focus,
    Click,
    TypeText,
    #[default]
    InspectUi,
}

pub struct AgentPcUsePayloadTool;

impl AgentTool for AgentPcUsePayloadTool {
    type Input = AgentPcUsePayloadToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_PC_USE_PAYLOAD_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input.map(|input| input.action) {
            Ok(AgentPcUsePayloadAction::Screenshot) => "Compose Zed screenshot payload".into(),
            Ok(AgentPcUsePayloadAction::Focus) => "Compose Zed focus payload".into(),
            Ok(AgentPcUsePayloadAction::Click) => "Compose Zed click payload".into(),
            Ok(AgentPcUsePayloadAction::TypeText) => "Compose Zed type payload".into(),
            Ok(AgentPcUsePayloadAction::InspectUi) => "Compose Zed UI inspect payload".into(),
            Err(_) => "Compose Zed PC-use payload".into(),
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
            let result = compose_pc_use_payload(&input);
            let valid = result
                .pointer("/result/valid")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let output = serde_json::to_string_pretty(&result)
                .map_err(|error| format!("Failed to serialize PC-use payload: {error}"))?;

            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(if valid {
                "Composed Zed PC-use payload"
            } else {
                "Zed PC-use payload needs fixes"
            }));

            if valid { Ok(output) } else { Err(output) }
        })
    }
}

fn compose_pc_use_payload(input: &AgentPcUsePayloadToolInput) -> Value {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let action = pc_use_action_name(input.action);
    let surface = normalize_surface(&input.surface);
    let surface_allowed = ALLOWED_SURFACES.contains(&surface.as_str());
    if !surface_allowed {
        blockers.push(blocker(
            "unsupported_surface",
            "PC-use payloads are limited to Zed-owned surfaces.",
            "Use one of workspace, editor, web_preview, agent_panel, right_panel, project_panel, or terminal.",
        ));
    }

    let target_id = input
        .target_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty());
    let target_label = input
        .target_label
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty());

    if matches!(
        input.action,
        AgentPcUsePayloadAction::Focus
            | AgentPcUsePayloadAction::Click
            | AgentPcUsePayloadAction::TypeText
    ) && target_id.is_none()
    {
        blockers.push(blocker(
            "missing_target_id",
            "Future focus, click, and type actions require a target id from a Zed UI inspection receipt.",
            "Run inspect_zed_window_context now and wait for a future UI inspection receipt before execution.",
        ));
    }

    let text = input.text.as_deref().unwrap_or_default();
    let text_len = text.chars().count();
    if matches!(input.action, AgentPcUsePayloadAction::TypeText) {
        if text.is_empty() {
            blockers.push(blocker(
                "missing_text",
                "Type payloads require explicit text.",
                "Set text to the exact string the future Zed-window type executor should insert.",
            ));
        } else if text_len > MAX_PC_USE_TEXT_CHARS {
            blockers.push(blocker(
                "text_too_long",
                "Type payload text exceeds the PC-use executor limit.",
                "Split the text into smaller explicit payloads.",
            ));
        }
    } else if input.text.is_some() {
        warnings.push(warning(
            "unused_text",
            "Text is only used by type_text payloads and will be ignored for this action.",
        ));
    }

    let button = normalize_button(&input.button);
    if matches!(input.action, AgentPcUsePayloadAction::Click)
        && !ALLOWED_BUTTONS.contains(&button.as_str())
    {
        blockers.push(blocker(
            "unsupported_button",
            "Click payloads support only left, middle, or right buttons.",
            "Use a supported mouse button.",
        ));
    }
    let click_count = input.click_count.clamp(1, 3);
    if input.click_count != click_count {
        warnings.push(warning(
            "click_count_clamped",
            "Click count is clamped to the supported 1 through 3 range.",
        ));
    }

    let valid = blockers.is_empty();
    let payload = match input.action {
        AgentPcUsePayloadAction::Screenshot => serde_json::json!({
            "action": action,
            "surface": surface,
            "target_id": target_id,
        }),
        AgentPcUsePayloadAction::Focus | AgentPcUsePayloadAction::InspectUi => serde_json::json!({
            "action": action,
            "surface": surface,
            "target_id": target_id,
            "target_label": target_label,
        }),
        AgentPcUsePayloadAction::Click => serde_json::json!({
            "action": action,
            "surface": surface,
            "target_id": target_id,
            "target_label": target_label,
            "button": button,
            "click_count": click_count,
        }),
        AgentPcUsePayloadAction::TypeText => serde_json::json!({
            "action": action,
            "surface": surface,
            "target_id": target_id,
            "target_label": target_label,
            "text": if valid { Some(text) } else { None },
            "text_len": text_len,
        }),
    };

    serde_json::json!({
        "schema": "zed.agent_plugins.pc_use.action_payload_result.v1",
        "generated_at_ms": current_epoch_millis(),
        "result": {
            "valid": valid,
            "action": action,
            "blockers": blockers,
            "warnings": warnings,
        },
        "payload_packet": {
            "schema": AGENT_PC_USE_PAYLOAD_SCHEMA,
            "generated_at_ms": current_epoch_millis(),
            "source_tool": AGENT_PC_USE_PAYLOAD_TOOL_NAME,
            "payload": payload,
            "safety": {
                "read_only_payload_only": true,
                "dispatches_input": false,
                "takes_screenshot": false,
                "focuses_window": false,
                "launches_process": false,
                "os_wide_control": false,
                "requires_future_permission_gate": true,
                "requires_visible_target_receipt": matches!(
                    input.action,
                    AgentPcUsePayloadAction::Focus
                        | AgentPcUsePayloadAction::Click
                        | AgentPcUsePayloadAction::TypeText
                ),
            },
        },
        "handoff": input.include_handoff_instructions.then(|| serde_json::json!({
            "inspect_tool_name": "inspect_zed_window_context",
            "future_executor_status": "not_enabled",
            "next_steps": [
                "Use inspect_zed_window_context to collect safe workspace context.",
                "Wait for a future Zed UI inspection receipt before focus, click, or type execution.",
                "Require explicit user-visible permission before any future screenshot, focus, click, or type action.",
                "Keep OS-wide desktop automation blocked unless the user grants a separate permission."
            ]
        })),
    })
}

fn normalize_surface(surface: &str) -> String {
    surface.trim().to_ascii_lowercase().replace('-', "_")
}

fn normalize_button(button: &str) -> String {
    button.trim().to_ascii_lowercase()
}

fn pc_use_action_name(action: AgentPcUsePayloadAction) -> &'static str {
    match action {
        AgentPcUsePayloadAction::Screenshot => "screenshot",
        AgentPcUsePayloadAction::Focus => "focus",
        AgentPcUsePayloadAction::Click => "click",
        AgentPcUsePayloadAction::TypeText => "type_text",
        AgentPcUsePayloadAction::InspectUi => "inspect_ui",
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

fn current_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}
