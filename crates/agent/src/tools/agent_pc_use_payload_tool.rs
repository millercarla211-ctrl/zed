use crate::{
    AgentTool, PC_USE_FUTURE_UI_TARGET_PREFIXES, ToolCallEventStream, ToolInput,
    ToolPermissionContext,
};
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

pub const AGENT_PC_USE_PAYLOAD_TOOL_NAME: &str = "compose_zed_pc_use_action_payload";
pub const AGENT_PC_USE_PAYLOAD_STAGE_TOOL_NAME: &str = "stage_zed_pc_use_action_payload";
pub const AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME: &str = "queue_zed_pc_use_action_payload";
pub const AGENT_PC_USE_PAYLOAD_SCHEMA: &str = "zed.agent_plugins.pc_use.action_payload.v1";
pub const AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA: &str =
    "zed.agent_plugins.pc_use.action_payload_queue_item.v1";
pub const AGENT_PC_USE_PAYLOAD_QUEUE_FILE_NAME: &str = "latest-zed-pc-use-payload.json";
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PcUseTargetIdKind {
    CurrentWorkspaceSnapshot,
    CurrentProjectPanelWorktreeSnapshot,
    FutureZedUiSnapshot,
    UnknownZedNamespace,
    Unknown,
}

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
    /// Optional snapshot receipt id that produced the target id.
    pub target_snapshot_id: Option<String>,
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
            target_snapshot_id: None,
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

/// Stages a validated Zed-window PC-use payload packet onto the clipboard.
///
/// This tool writes only the schema-versioned payload packet after explicit authorization. It does
/// not take screenshots, focus Zed, click, type, launch processes, or control the OS desktop.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPcUsePayloadStageToolInput {
    /// The PC-use payload to compose and stage.
    pub payload: AgentPcUsePayloadToolInput,
}

impl Default for AgentPcUsePayloadStageToolInput {
    fn default() -> Self {
        Self {
            payload: AgentPcUsePayloadToolInput {
                include_handoff_instructions: false,
                ..Default::default()
            },
        }
    }
}

pub struct AgentPcUsePayloadStageTool;

impl AgentTool for AgentPcUsePayloadStageTool {
    type Input = AgentPcUsePayloadStageToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_PC_USE_PAYLOAD_STAGE_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input.map(|input| input.payload.action) {
            Ok(AgentPcUsePayloadAction::Screenshot) => "Stage Zed screenshot payload".into(),
            Ok(AgentPcUsePayloadAction::Focus) => "Stage Zed focus payload".into(),
            Ok(AgentPcUsePayloadAction::Click) => "Stage Zed click payload".into(),
            Ok(AgentPcUsePayloadAction::TypeText) => "Stage Zed type payload".into(),
            Ok(AgentPcUsePayloadAction::InspectUi) => "Stage Zed UI inspect payload".into(),
            Err(_) => "Stage Zed PC-use payload".into(),
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
            let result = compose_pc_use_payload(&input.payload);
            let valid = result
                .pointer("/result/valid")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !valid {
                let output = serde_json::to_string_pretty(&result)
                    .map_err(|error| format!("Failed to serialize PC-use payload: {error}"))?;
                event_stream.update_fields(
                    acp::ToolCallUpdateFields::new().title("Zed PC-use payload needs fixes"),
                );
                return Err(output);
            }

            let packet = result
                .get("payload_packet")
                .cloned()
                .ok_or_else(|| "Missing payload_packet in composed PC-use payload".to_string())?;
            let packet_json = serde_json::to_string_pretty(&packet)
                .map_err(|error| format!("Failed to serialize PC-use payload packet: {error}"))?;

            let context = ToolPermissionContext::new(
                Self::NAME,
                vec![
                    pc_use_action_name(input.payload.action).to_string(),
                    format!("{} clipboard characters", packet_json.chars().count()),
                ],
            );
            let authorize = cx.update(|cx| {
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });
            authorize.await.map_err(|error| error.to_string())?;

            cx.update(|cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(packet_json.clone()));
            });

            let output = serde_json::json!({
                "schema": "zed.agent_plugins.pc_use.action_payload_stage_result.v1",
                "result": {
                    "generated_at_ms": current_epoch_millis(),
                    "status": "staged_to_clipboard",
                    "action": pc_use_action_name(input.payload.action),
                    "clipboard_characters": packet_json.chars().count(),
                    "next_step": "Keep this packet as an explicit handoff only. Future PC-use execution still requires Zed UI inspection, user-visible permission, a focused Zed target, and an execution receipt."
                },
                "payload_packet": packet,
                "safety": {
                    "tool_dispatches_input": false,
                    "tool_takes_screenshot": false,
                    "tool_focuses_zed": false,
                    "clipboard_written_after_authorization": true,
                    "requires_future_pc_use_permission_gate": true,
                    "os_wide_desktop_control": false
                }
            });
            let output = serde_json::to_string_pretty(&output)
                .map_err(|error| format!("Failed to serialize PC-use staging result: {error}"))?;

            event_stream
                .update_fields(acp::ToolCallUpdateFields::new().title("Staged Zed PC-use payload"));

            Ok(output)
        })
    }
}

/// Queues a validated Zed-window PC-use payload into a managed workspace or Zed-data file.
///
/// This creates a handoff artifact only. It does not import, execute, screenshot, focus, click,
/// type, launch processes, or control the OS desktop.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPcUsePayloadQueueToolInput {
    /// The PC-use payload to compose and queue.
    pub payload: AgentPcUsePayloadToolInput,
    /// Prefer workspace-local queue under `<workspace>/tools`; falls back to Zed data when no workspace exists.
    pub root_mode: AgentPcUsePayloadQueueRootMode,
}

impl Default for AgentPcUsePayloadQueueToolInput {
    fn default() -> Self {
        Self {
            payload: AgentPcUsePayloadToolInput {
                include_handoff_instructions: false,
                ..Default::default()
            },
            root_mode: AgentPcUsePayloadQueueRootMode::Workspace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentPcUsePayloadQueueRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct AgentPcUsePayloadQueueTool {
    project: Entity<Project>,
}

impl AgentPcUsePayloadQueueTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentPcUsePayloadQueueTool {
    type Input = AgentPcUsePayloadQueueToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Execute
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input.map(|input| input.payload.action) {
            Ok(AgentPcUsePayloadAction::Screenshot) => "Queue Zed screenshot payload".into(),
            Ok(AgentPcUsePayloadAction::Focus) => "Queue Zed focus payload".into(),
            Ok(AgentPcUsePayloadAction::Click) => "Queue Zed click payload".into(),
            Ok(AgentPcUsePayloadAction::TypeText) => "Queue Zed type payload".into(),
            Ok(AgentPcUsePayloadAction::InspectUi) => "Queue Zed UI inspect payload".into(),
            Err(_) => "Queue Zed PC-use payload".into(),
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
            let queue = AgentPcUsePayloadQueue::new(project_root, input.root_mode);

            let result = compose_pc_use_payload(&input.payload);
            let valid = result
                .pointer("/result/valid")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !valid {
                let output = serde_json::to_string_pretty(&result)
                    .map_err(|error| format!("Failed to serialize PC-use payload: {error}"))?;
                event_stream.update_fields(
                    acp::ToolCallUpdateFields::new().title("Zed PC-use payload needs fixes"),
                );
                return Err(output);
            }

            queue.validate()?;
            let packet = result
                .get("payload_packet")
                .cloned()
                .ok_or_else(|| "Missing payload_packet in composed PC-use payload".to_string())?;
            let queue_item = queue.queue_item(&input, packet);
            let queue_item_json = serde_json::to_vec_pretty(&queue_item)
                .map_err(|error| format!("Failed to serialize queued PC-use payload: {error}"))?;

            let context = ToolPermissionContext::new(
                Self::NAME,
                vec![
                    pc_use_action_name(input.payload.action).to_string(),
                    path_string(&queue.queue_dir),
                    path_string(&queue.latest_path),
                    format!("{} queued bytes", queue_item_json.len()),
                ],
            );
            let authorize = cx.update(|cx| {
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });
            authorize.await.map_err(|error| error.to_string())?;

            fs::create_dir_all(&queue.queue_dir).map_err(|error| {
                format!(
                    "Failed to prepare PC-use payload queue {}: {error}",
                    queue.queue_dir.display()
                )
            })?;
            fs::write(&queue.latest_path, &queue_item_json).map_err(|error| {
                format!(
                    "Failed to write PC-use payload queue {}: {error}",
                    queue.latest_path.display()
                )
            })?;
            fs::write(&queue.archive_path, &queue_item_json).map_err(|error| {
                format!(
                    "Failed to archive PC-use payload queue item {}: {error}",
                    queue.archive_path.display()
                )
            })?;

            let output = serde_json::json!({
                "schema": "zed.agent_plugins.pc_use.action_payload_queue_result.v1",
                "result": {
                    "generated_at_ms": current_epoch_millis(),
                    "status": "queued_to_managed_handoff",
                    "action": pc_use_action_name(input.payload.action),
                    "root_mode": queue.root_mode_label(),
                    "queue_path": path_string(&queue.queue_dir),
                    "latest_path": path_string(&queue.latest_path),
                    "archive_path": path_string(&queue.archive_path),
                    "next_step": "Use this managed handoff only after Zed UI inspection and explicit permission are available. Future PC-use executors must reread the packet, validate a fresh target receipt, and emit an execution receipt."
                },
                "queued_item": queue_item,
                "safety": {
                    "tool_dispatches_input": false,
                    "tool_takes_screenshot": false,
                    "tool_focuses_zed": false,
                    "managed_queue_written_after_authorization": true,
                    "requires_future_pc_use_import": true,
                    "requires_future_pc_use_permission_gate": true,
                    "os_wide_desktop_control": false
                }
            });
            let output = serde_json::to_string_pretty(&output)
                .map_err(|error| format!("Failed to serialize PC-use queue result: {error}"))?;

            event_stream
                .update_fields(acp::ToolCallUpdateFields::new().title("Queued Zed PC-use payload"));

            Ok(output)
        })
    }
}

struct AgentPcUsePayloadQueue {
    root_mode: AgentPcUsePayloadQueueRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
    queue_dir: PathBuf,
    latest_path: PathBuf,
    archive_path: PathBuf,
}

impl AgentPcUsePayloadQueue {
    fn new(project_root: Option<PathBuf>, root_mode: AgentPcUsePayloadQueueRootMode) -> Self {
        let use_workspace = matches!(root_mode, AgentPcUsePayloadQueueRootMode::Workspace)
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
            allowed_root
                .join("agent-plugins")
                .join("pc-use")
                .join("payloads")
        } else {
            allowed_root.join("pc-use").join("payloads")
        };
        let latest_path = queue_dir.join(AGENT_PC_USE_PAYLOAD_QUEUE_FILE_NAME);
        let archive_path = queue_dir.join(format!(
            "zed-pc-use-payload-{}.json",
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
                    "Refusing to queue PC-use payload at unmanaged path {} outside {}",
                    path.display(),
                    self.allowed_root.display()
                ));
            }
        }

        Ok(())
    }

    fn queue_item(&self, input: &AgentPcUsePayloadQueueToolInput, packet: Value) -> Value {
        serde_json::json!({
            "schema": AGENT_PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA,
            "queued_at_ms": current_epoch_millis(),
            "source_tool": AGENT_PC_USE_PAYLOAD_QUEUE_TOOL_NAME,
            "payload_packet": packet,
            "metadata": {
                "action": pc_use_action_name(input.payload.action),
                "surface": normalize_surface(&input.payload.surface),
                "root_mode": self.root_mode_label(),
                "project_root": self.project_root.as_ref().map(path_string),
                "queue_path": path_string(&self.queue_dir),
                "latest_path": path_string(&self.latest_path),
                "archive_path": path_string(&self.archive_path),
                "requires_zed_ui_inspection": true,
                "requires_explicit_permission": true,
                "requires_fresh_target_receipt": true,
                "requires_receipt_after_execution": true,
            },
            "safety": {
                "input_dispatched": false,
                "screenshot_taken": false,
                "zed_focus_changed": false,
                "process_launched": false,
                "os_wide_desktop_control": false,
            }
        })
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            AgentPcUsePayloadQueueRootMode::Workspace if self.project_root.is_some() => "workspace",
            AgentPcUsePayloadQueueRootMode::Workspace => "zed_data_fallback",
            AgentPcUsePayloadQueueRootMode::ZedData => "zed_data",
        }
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
    let target_snapshot_id = input
        .target_snapshot_id
        .as_deref()
        .map(str::trim)
        .filter(|snapshot_id| !snapshot_id.is_empty());
    let target_id_kind = target_id.map(classify_pc_use_target_id);

    if let Some((target_id, target_id_kind)) = target_id.zip(target_id_kind) {
        if let Some(required_surface) = target_id_kind.required_surface() {
            if surface != required_surface {
                blockers.push(blocker(
                    "target_surface_mismatch",
                    "The target id belongs to a different Zed surface than the requested payload surface.",
                    "Use the surface from inspect_zed_pc_use_target_snapshot or choose a matching target id.",
                ));
            }
        }

        if target_id_kind == PcUseTargetIdKind::Unknown {
            warnings.push(warning(
                "unknown_target_namespace",
                "Target ids should come from inspect_zed_pc_use_target_snapshot or a future Zed UI snapshot.",
            ));
        }

        if matches!(
            input.action,
            AgentPcUsePayloadAction::Focus
                | AgentPcUsePayloadAction::Click
                | AgentPcUsePayloadAction::TypeText
        ) && !target_id_kind.is_input_ready()
        {
            blockers.push(blocker(
                "target_not_input_ready",
                "Only future Zed UI snapshot ids are safe for focus, click, or type intent composition.",
                "Wait for a future Zed UI snapshot target id before composing focus, click, or type_text payloads.",
            ));
        }

        if target_id.len() > 512 {
            blockers.push(blocker(
                "target_id_too_long",
                "Target ids are bounded to keep PC-use handoff packets small and auditable.",
                "Use a target id returned by inspect_zed_pc_use_target_snapshot or a future Zed UI snapshot.",
            ));
        }

        if matches!(
            input.action,
            AgentPcUsePayloadAction::Focus
                | AgentPcUsePayloadAction::Click
                | AgentPcUsePayloadAction::TypeText
        ) && target_id_kind.is_input_ready()
            && target_snapshot_id.is_none()
        {
            blockers.push(blocker(
                "missing_target_snapshot_id",
                "Future focus, click, and type payloads require the Zed UI snapshot receipt id that produced the target.",
                "Run a live Zed UI snapshot first and pass its snapshot_id with the target id.",
            ));
        }
    }

    if let Some(snapshot_id) = target_snapshot_id {
        if !snapshot_id.starts_with("zed-ui-snapshot-") {
            blockers.push(blocker(
                "invalid_target_snapshot_id",
                "Target snapshot ids must come from a Zed UI snapshot receipt.",
                "Use the snapshot_id returned by inspect_zed_pc_use_ui_snapshot or a future live UI snapshot tool.",
            ));
        }

        if snapshot_id.len() > 128 {
            blockers.push(blocker(
                "target_snapshot_id_too_long",
                "Target snapshot ids are bounded to keep PC-use packets auditable.",
                "Use the exact snapshot_id returned by the Zed UI snapshot tool.",
            ));
        }

        if target_id.is_none() {
            warnings.push(warning(
                "unused_target_snapshot_id",
                "A target snapshot id is only meaningful when a target_id is also provided.",
            ));
        }
    }

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
            "target_snapshot_id": target_snapshot_id,
            "target_reference": target_id.map(|target_id| target_reference(target_id, target_snapshot_id)),
        }),
        AgentPcUsePayloadAction::Focus | AgentPcUsePayloadAction::InspectUi => serde_json::json!({
            "action": action,
            "surface": surface,
            "target_id": target_id,
            "target_label": target_label,
            "target_snapshot_id": target_snapshot_id,
            "target_reference": target_id.map(|target_id| target_reference(target_id, target_snapshot_id)),
        }),
        AgentPcUsePayloadAction::Click => serde_json::json!({
            "action": action,
            "surface": surface,
            "target_id": target_id,
            "target_label": target_label,
            "target_snapshot_id": target_snapshot_id,
            "target_reference": target_id.map(|target_id| target_reference(target_id, target_snapshot_id)),
            "button": button,
            "click_count": click_count,
        }),
        AgentPcUsePayloadAction::TypeText => serde_json::json!({
            "action": action,
            "surface": surface,
            "target_id": target_id,
            "target_label": target_label,
            "target_snapshot_id": target_snapshot_id,
            "target_reference": target_id.map(|target_id| target_reference(target_id, target_snapshot_id)),
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
            "target_manifest_tool_name": "inspect_zed_pc_use_targets",
            "target_snapshot_tool_name": "inspect_zed_pc_use_target_snapshot",
            "ui_snapshot_contract_tool_name": "inspect_zed_pc_use_ui_snapshot_contract",
            "ui_snapshot_tool_name": "inspect_zed_pc_use_ui_snapshot",
            "future_executor_status": "not_enabled",
            "next_steps": [
                "Use inspect_zed_window_context to collect safe workspace context.",
                "Use inspect_zed_pc_use_targets to read supported surfaces and action requirements.",
                "Use inspect_zed_pc_use_target_snapshot for current read-only workspace or project-panel target ids.",
                "Use inspect_zed_pc_use_ui_snapshot_contract before accepting future focus, click, or type target ids.",
                "Use inspect_zed_pc_use_ui_snapshot to see the current partial Zed UI snapshot and live-UI gaps.",
                "Include target_snapshot_id from the live Zed UI snapshot receipt before composing future focus, click, or type payloads.",
                "Wait for a future Zed UI inspection receipt before focus, click, or type execution.",
                "Require explicit user-visible permission before any future screenshot, focus, click, or type action.",
                "Keep OS-wide desktop automation blocked unless the user grants a separate permission."
            ]
        })),
    })
}

fn classify_pc_use_target_id(target_id: &str) -> PcUseTargetIdKind {
    if target_id == "zed:workspace:active" {
        PcUseTargetIdKind::CurrentWorkspaceSnapshot
    } else if target_id.starts_with("zed:project_panel:worktree:") {
        PcUseTargetIdKind::CurrentProjectPanelWorktreeSnapshot
    } else if PC_USE_FUTURE_UI_TARGET_PREFIXES
        .iter()
        .any(|(prefix, _, _)| target_id.starts_with(prefix))
    {
        PcUseTargetIdKind::FutureZedUiSnapshot
    } else if target_id.starts_with("zed:") {
        PcUseTargetIdKind::UnknownZedNamespace
    } else {
        PcUseTargetIdKind::Unknown
    }
}

impl PcUseTargetIdKind {
    fn label(self) -> &'static str {
        match self {
            Self::CurrentWorkspaceSnapshot => "current_workspace_snapshot",
            Self::CurrentProjectPanelWorktreeSnapshot => "current_project_panel_worktree_snapshot",
            Self::FutureZedUiSnapshot => "future_zed_ui_snapshot",
            Self::UnknownZedNamespace => "unknown_zed_namespace",
            Self::Unknown => "unknown",
        }
    }

    fn required_surface(self) -> Option<&'static str> {
        match self {
            Self::CurrentWorkspaceSnapshot => Some("workspace"),
            Self::CurrentProjectPanelWorktreeSnapshot => Some("project_panel"),
            Self::FutureZedUiSnapshot | Self::UnknownZedNamespace | Self::Unknown => None,
        }
    }

    fn is_current_snapshot(self) -> bool {
        matches!(
            self,
            Self::CurrentWorkspaceSnapshot | Self::CurrentProjectPanelWorktreeSnapshot
        )
    }

    fn is_input_ready(self) -> bool {
        matches!(self, Self::FutureZedUiSnapshot)
    }
}

fn target_reference(target_id: &str, target_snapshot_id: Option<&str>) -> Value {
    let kind = classify_pc_use_target_id(target_id);
    serde_json::json!({
        "source": kind.label(),
        "target_snapshot_id": target_snapshot_id,
        "current_snapshot_target": kind.is_current_snapshot(),
        "input_ready": kind.is_input_ready(),
        "required_surface": kind.required_surface(),
        "safe_for_read_only_actions": true,
        "safe_for_input_actions": kind.is_input_ready(),
        "requires_live_ui_snapshot": kind.is_input_ready(),
        "has_target_snapshot_receipt": target_snapshot_id.is_some(),
        "requires_fresh_target_receipt_for_execution": true,
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
