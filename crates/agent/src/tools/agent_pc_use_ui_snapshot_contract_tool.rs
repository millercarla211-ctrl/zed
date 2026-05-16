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

pub const AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_TOOL_NAME: &str =
    "inspect_zed_pc_use_ui_snapshot_contract";
pub const AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_SCHEMA: &str =
    "zed.agent_plugins.pc_use.ui_snapshot_contract.v1";

pub const PC_USE_FUTURE_UI_TARGET_PREFIXES: &[(&str, &str, &[&str])] = &[
    (
        "zed:editor:",
        "editor",
        &["inspect_ui", "screenshot", "focus", "click", "type_text"],
    ),
    (
        "zed:web_preview:",
        "web_preview",
        &["inspect_ui", "screenshot", "focus", "click"],
    ),
    (
        "zed:agent_panel:",
        "agent_panel",
        &["inspect_ui", "screenshot", "focus", "click", "type_text"],
    ),
    (
        "zed:right_panel:",
        "right_panel",
        &["inspect_ui", "screenshot", "focus", "click"],
    ),
    (
        "zed:project_panel:entry:",
        "project_panel",
        &["inspect_ui", "screenshot", "focus", "click"],
    ),
    (
        "zed:terminal:",
        "terminal",
        &["inspect_ui", "screenshot", "focus", "click", "type_text"],
    ),
    (
        "zed:title_bar:",
        "workspace",
        &["inspect_ui", "screenshot", "focus", "click"],
    ),
    (
        "zed:status_bar:",
        "workspace",
        &["inspect_ui", "screenshot", "focus", "click"],
    ),
];

/// Returns the read-only future Zed UI snapshot target contract.
///
/// This tool publishes the target-id namespaces and receipt requirements that future UI snapshot
/// tooling must produce before PC-use focus, click, or type payloads can be considered
/// input-ready. It does not inspect live UI, take screenshots, focus panes, click, type, launch
/// processes, or control the desktop.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPcUseUiSnapshotContractToolInput {
    /// Include target namespace rows.
    pub include_target_namespaces: bool,
    /// Include example target ids.
    pub include_examples: bool,
    /// Include future snapshot field requirements.
    pub include_snapshot_fields: bool,
}

impl Default for AgentPcUseUiSnapshotContractToolInput {
    fn default() -> Self {
        Self {
            include_target_namespaces: true,
            include_examples: true,
            include_snapshot_fields: true,
        }
    }
}

pub struct AgentPcUseUiSnapshotContractTool;

impl AgentTool for AgentPcUseUiSnapshotContractTool {
    type Input = AgentPcUseUiSnapshotContractToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Inspect Zed UI snapshot contract".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |_cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let result = ui_snapshot_contract(input);
            let output = serde_json::to_string_pretty(&result).map_err(|error| {
                format!("Failed to serialize PC-use UI snapshot contract: {error}")
            })?;

            event_stream.update_fields(
                acp::ToolCallUpdateFields::new().title("Inspected Zed UI snapshot contract"),
            );

            Ok(output)
        })
    }
}

fn ui_snapshot_contract(input: AgentPcUseUiSnapshotContractToolInput) -> Value {
    serde_json::json!({
        "schema": AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_SCHEMA,
        "generated_at_ms": current_epoch_millis(),
        "status": "contract_available_snapshot_not_live",
        "tool": {
            "name": AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_TOOL_NAME,
            "kind": "read_only",
        },
        "target_namespaces": input
            .include_target_namespaces
            .then(|| target_namespaces(input.include_examples)),
        "snapshot_fields": input.include_snapshot_fields.then(snapshot_fields),
        "input_readiness": {
            "input_ready_target_prefixes": PC_USE_FUTURE_UI_TARGET_PREFIXES
                .iter()
                .map(|(prefix, _, _)| *prefix)
                .collect::<Vec<_>>(),
            "current_snapshot_prefixes_read_only_only": [
                "zed:workspace:active",
                "zed:project_panel:worktree:"
            ],
            "unknown_zed_prefixes_input_ready": false,
            "non_zed_target_ids_input_ready": false,
            "coordinates_accepted_from_agents": false,
            "requires_fresh_snapshot_receipt": true,
            "requires_visible_target": true,
            "requires_user_permission_for_focus_click_type": true,
        },
        "required_flow": [
            "inspect_zed_window_context",
            "inspect_zed_pc_use_targets",
            "inspect_zed_pc_use_target_snapshot",
            "inspect_zed_pc_use_ui_snapshot_contract",
            "future live Zed UI snapshot receipt",
            "compose_zed_pc_use_action_payload",
            "queue_zed_pc_use_action_payload or stage_zed_pc_use_action_payload",
            "request_zed_pc_use_payload_run",
            "inspect_zed_pc_use_runner_receipts or WebPreview Copy/Send Zed PC-use Status"
        ],
        "safety": {
            "read_only": true,
            "live_ui_inspected": false,
            "takes_screenshot": false,
            "focuses_zed": false,
            "dispatches_mouse": false,
            "dispatches_keyboard": false,
            "launches_process": false,
            "os_wide_desktop_control": false,
        },
        "next_actions": [
            "Use this contract to generate or validate future Zed UI snapshot target ids.",
            "Do not compose focus, click, or type_text payloads from current snapshot ids or arbitrary zed: strings.",
            "Keep future live UI snapshot implementation read-only until manual Windows QA confirms target stability."
        ],
    })
}

fn target_namespaces(include_examples: bool) -> Vec<Value> {
    PC_USE_FUTURE_UI_TARGET_PREFIXES
        .iter()
        .map(|(prefix, surface, actions)| {
            let mut namespace = serde_json::json!({
                "prefix": *prefix,
                "surface": *surface,
                "actions": *actions,
                "input_ready_after_fresh_snapshot": true,
                "requires_visible_bounds": true,
                "requires_focus_receipt_for_type": actions.contains(&"type_text"),
                "coordinates_not_agent_supplied": true,
            });
            if include_examples {
                namespace["example_target_id"] = example_target_id(*prefix);
            }
            namespace
        })
        .collect()
}

fn example_target_id(prefix: &str) -> Value {
    let suffix = match prefix {
        "zed:editor:" => "pane:active:item:buffer:selection",
        "zed:web_preview:" => "session:active:viewport",
        "zed:agent_panel:" => "thread:active:input",
        "zed:right_panel:" => "panel:assets:row:selected",
        "zed:project_panel:entry:" => "worktree:1:entry:src-main-rs",
        "zed:terminal:" => "pane:active:terminal:input",
        "zed:title_bar:" => "control:command-center",
        "zed:status_bar:" => "control:diagnostics",
        _ => "target",
    };
    serde_json::json!(format!("{prefix}{suffix}"))
}

fn snapshot_fields() -> Vec<Value> {
    vec![
        serde_json::json!({
            "field": "target_id",
            "required": true,
            "description": "Stable only for the current snapshot generation; must use a documented zed: prefix.",
        }),
        serde_json::json!({
            "field": "surface",
            "required": true,
            "description": "One of workspace, editor, web_preview, agent_panel, right_panel, project_panel, or terminal.",
        }),
        serde_json::json!({
            "field": "label",
            "required": true,
            "description": "Human-readable target label used in permission prompts and receipts.",
        }),
        serde_json::json!({
            "field": "visible",
            "required": true,
            "description": "Whether the target is visible in the current Zed window snapshot.",
        }),
        serde_json::json!({
            "field": "bounds",
            "required": true,
            "description": "Editor-native bounds for audit and preflight only; agents must not author coordinates.",
        }),
        serde_json::json!({
            "field": "enabled",
            "required": true,
            "description": "Whether the target can receive future focus or click actions.",
        }),
        serde_json::json!({
            "field": "focused",
            "required": false,
            "description": "Whether the target or its owning surface currently has focus.",
        }),
        serde_json::json!({
            "field": "risk",
            "required": true,
            "description": "Low, medium, or high risk; terminal and agent text input must remain high-risk.",
        }),
    ]
}

fn current_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}
