use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, Entity, SharedString, Task};
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const AGENT_PC_USE_TARGET_MANIFEST_TOOL_NAME: &str = "inspect_zed_pc_use_targets";
pub const AGENT_PC_USE_TARGET_MANIFEST_SCHEMA: &str = "zed.agent_plugins.pc_use.target_manifest.v1";

const PC_USE_SURFACES: &[(&str, &str)] = &[
    (
        "workspace",
        "The whole Zed workspace window; use only for read-only inspection or screenshots.",
    ),
    (
        "editor",
        "The active code editor surface; future typing must require focused editor target ids.",
    ),
    (
        "web_preview",
        "An in-editor WebPreview surface; browser actions should prefer the dedicated Browser plugin tools.",
    ),
    (
        "agent_panel",
        "The Agent Panel; future typing requires explicit user approval and a fresh target receipt.",
    ),
    (
        "right_panel",
        "The right asset/plugin panels; future clicks require a visible control target id.",
    ),
    (
        "project_panel",
        "The project/file panel; future clicks require a visible row or control target id.",
    ),
    (
        "terminal",
        "The integrated terminal; future typing is high-risk and must remain permission-gated.",
    ),
];

const PC_USE_ACTIONS: &[(&str, &[&str], &str)] = &[
    (
        "inspect_ui",
        &["surface"],
        "Read safe visible UI metadata after a future Zed UI snapshot is available.",
    ),
    (
        "screenshot",
        &["surface"],
        "Capture a Zed-window screenshot only after explicit permission and a screenshot receipt.",
    ),
    (
        "focus",
        &["surface", "target_id"],
        "Focus a Zed surface by a safe editor-native target id.",
    ),
    (
        "click",
        &["surface", "target_id", "button", "click_count"],
        "Click within Zed only after fresh target preflight, permission, and receipt gates.",
    ),
    (
        "type_text",
        &["surface", "target_id", "text"],
        "Type text only after focused-target validation, explicit text payload, and receipt gates.",
    ),
];

/// Returns the read-only Zed-window PC-use target contract for payload composition.
///
/// This tool does not inspect pixels, take screenshots, focus panes, click, type, launch
/// processes, or control the desktop. It tells agents which Zed surfaces and action fields are
/// allowed before they compose a PC-use payload or request a future executor.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPcUseTargetManifestToolInput {
    /// Include example payload hints for each supported action.
    pub include_examples: bool,
    /// Include the fixed surface catalog.
    pub include_surfaces: bool,
}

impl Default for AgentPcUseTargetManifestToolInput {
    fn default() -> Self {
        Self {
            include_examples: true,
            include_surfaces: true,
        }
    }
}

pub struct AgentPcUseTargetManifestTool {
    project: Entity<Project>,
}

impl AgentPcUseTargetManifestTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentPcUseTargetManifestTool {
    type Input = AgentPcUseTargetManifestToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_PC_USE_TARGET_MANIFEST_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Inspect Zed PC-use targets".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let result = cx.update(|cx| inspect_pc_use_targets(&self.project, input, cx));
            let output = serde_json::to_string_pretty(&result)
                .map_err(|error| format!("Failed to serialize PC-use target manifest: {error}"))?;

            event_stream.update_fields(
                acp::ToolCallUpdateFields::new().title("Inspected Zed PC-use targets"),
            );

            Ok(output)
        })
    }
}

fn inspect_pc_use_targets(
    project: &Entity<Project>,
    input: AgentPcUseTargetManifestToolInput,
    cx: &App,
) -> Value {
    let visible_worktrees = project.read(cx).visible_worktrees(cx).collect::<Vec<_>>();
    let project_root = visible_worktrees
        .first()
        .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf());
    let visible_worktree_count = visible_worktrees.len();
    let status = if visible_worktree_count == 0 {
        "no_visible_worktree"
    } else {
        "ready_for_payload_composition"
    };

    serde_json::json!({
        "schema": AGENT_PC_USE_TARGET_MANIFEST_SCHEMA,
        "generated_at_ms": current_epoch_millis(),
        "status": status,
        "tool": {
            "name": AGENT_PC_USE_TARGET_MANIFEST_TOOL_NAME,
            "kind": "read_only",
        },
        "workspace": {
            "project_root": project_root.as_ref().map(path_string),
            "visible_worktree_count": visible_worktree_count,
        },
        "surfaces": input.include_surfaces.then(surface_manifest),
        "actions": action_manifest(input.include_examples),
        "target_contract": {
            "target_id_source": "future zed UI snapshot or explicit user-selected target",
            "target_id_required_for": ["focus", "click", "type_text"],
            "target_label_optional": true,
            "coordinates_not_accepted_from_agents": true,
            "os_wide_targets_supported": false,
            "fresh_target_receipt_required": true,
        },
        "required_flow": [
            "inspect_zed_window_context",
            "inspect_zed_pc_use_targets",
            "compose_zed_pc_use_action_payload",
            "queue_zed_pc_use_action_payload or stage_zed_pc_use_action_payload",
            "inspect_zed_pc_use_payload_queue",
            "request_zed_pc_use_payload_run",
            "inspect_zed_pc_use_runner_receipts or WebPreview Copy/Send Zed PC-use Status"
        ],
        "safety": {
            "read_only": true,
            "takes_screenshot": false,
            "focuses_zed": false,
            "dispatches_mouse": false,
            "dispatches_keyboard": false,
            "launches_process": false,
            "os_wide_desktop_control": false,
            "future_execution_requires_permission": true,
            "future_execution_requires_receipt": true,
        },
        "next_actions": pc_use_target_next_actions(status),
    })
}

fn surface_manifest() -> Vec<Value> {
    PC_USE_SURFACES
        .iter()
        .map(|(id, description)| {
            serde_json::json!({
                "id": id,
                "description": description,
                "available_for_payloads": true,
                "future_executor_state": if *id == "terminal" {
                    "planned_high_risk_permission_gate"
                } else {
                    "planned_permission_gate"
                },
            })
        })
        .collect()
}

fn action_manifest(include_examples: bool) -> Vec<Value> {
    PC_USE_ACTIONS
        .iter()
        .map(|(action, required_fields, description)| {
            let mut entry = serde_json::json!({
                "action": action,
                "description": description,
                "required_payload_fields": required_fields,
                "executor_state": "planned_permission_gate",
                "requires_user_permission": *action != "inspect_ui",
                "requires_fresh_target_receipt": matches!(*action, "focus" | "click" | "type_text"),
                "dispatches_now": false,
            });
            if include_examples {
                entry["example_payload"] = example_payload(action);
            }
            entry
        })
        .collect()
}

fn example_payload(action: &str) -> Value {
    match action {
        "inspect_ui" => serde_json::json!({
            "action": "inspect_ui",
            "surface": "workspace"
        }),
        "screenshot" => serde_json::json!({
            "action": "screenshot",
            "surface": "workspace"
        }),
        "focus" => serde_json::json!({
            "action": "focus",
            "surface": "editor",
            "target_id": "from-future-ui-snapshot"
        }),
        "click" => serde_json::json!({
            "action": "click",
            "surface": "right_panel",
            "target_id": "from-future-ui-snapshot",
            "button": "left",
            "click_count": 1
        }),
        "type_text" => serde_json::json!({
            "action": "type_text",
            "surface": "editor",
            "target_id": "from-future-ui-snapshot",
            "text": "explicit user-approved text"
        }),
        _ => Value::Null,
    }
}

fn pc_use_target_next_actions(status: &str) -> Vec<&'static str> {
    match status {
        "ready_for_payload_composition" => vec![
            "Use this manifest before composing any Zed PC-use payload.",
            "Do not use coordinates or OS-wide window ids; wait for future Zed UI snapshot target ids.",
        ],
        _ => vec![
            "Open a workspace before composing Zed PC-use payloads.",
            "Keep PC-use execution disabled until a visible workspace and target receipt exist.",
        ],
    }
}

fn path_string(path: impl AsRef<std::path::Path>) -> String {
    path.as_ref().display().to_string()
}

fn current_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}
