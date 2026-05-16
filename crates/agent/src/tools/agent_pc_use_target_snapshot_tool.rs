use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, Entity, SharedString, Task};
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const AGENT_PC_USE_TARGET_SNAPSHOT_TOOL_NAME: &str = "inspect_zed_pc_use_target_snapshot";
pub const AGENT_PC_USE_TARGET_SNAPSHOT_SCHEMA: &str = "zed.agent_plugins.pc_use.target_snapshot.v1";

/// Returns the current read-only Zed target snapshot available from project state.
///
/// This is intentionally narrower than a full UI tree. It only reports real targets the Agent
/// can know from safe project metadata today, plus explicit gaps for surfaces that still require
/// a future UI snapshot. It does not inspect pixels, take screenshots, focus panes, click, type,
/// launch processes, or control the desktop.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPcUseTargetSnapshotToolInput {
    /// Include visible project worktree targets.
    pub include_worktrees: bool,
    /// Include surfaces that are known but not currently target-addressable from project state.
    pub include_gaps: bool,
    /// Maximum worktree target rows to include.
    pub max_targets: usize,
}

impl Default for AgentPcUseTargetSnapshotToolInput {
    fn default() -> Self {
        Self {
            include_worktrees: true,
            include_gaps: true,
            max_targets: 16,
        }
    }
}

pub struct AgentPcUseTargetSnapshotTool {
    project: Entity<Project>,
}

impl AgentPcUseTargetSnapshotTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentPcUseTargetSnapshotTool {
    type Input = AgentPcUseTargetSnapshotToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_PC_USE_TARGET_SNAPSHOT_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Inspect Zed PC-use target snapshot".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let result = cx.update(|cx| inspect_pc_use_target_snapshot(&self.project, input, cx));
            let output = serde_json::to_string_pretty(&result)
                .map_err(|error| format!("Failed to serialize PC-use target snapshot: {error}"))?;

            event_stream.update_fields(
                acp::ToolCallUpdateFields::new().title("Inspected Zed PC-use target snapshot"),
            );

            Ok(output)
        })
    }
}

fn inspect_pc_use_target_snapshot(
    project: &Entity<Project>,
    input: AgentPcUseTargetSnapshotToolInput,
    cx: &App,
) -> Value {
    let max_targets = input.max_targets.clamp(1, 64);
    let visible_worktrees = project.read(cx).visible_worktrees(cx).collect::<Vec<_>>();
    let project_root = visible_worktrees
        .first()
        .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf());
    let visible_worktree_count = visible_worktrees.len();
    let returned_worktree_count = if input.include_worktrees {
        visible_worktree_count.min(max_targets)
    } else {
        0
    };
    let target_count = 1 + returned_worktree_count;
    let status = if visible_worktree_count == 0 {
        "workspace_only"
    } else {
        "ready_for_safe_target_reference"
    };

    serde_json::json!({
        "schema": AGENT_PC_USE_TARGET_SNAPSHOT_SCHEMA,
        "generated_at_ms": current_epoch_millis(),
        "status": status,
        "tool": {
            "name": AGENT_PC_USE_TARGET_SNAPSHOT_TOOL_NAME,
            "kind": "read_only",
        },
        "workspace": {
            "project_root": project_root.as_ref().map(path_string),
            "visible_worktree_count": visible_worktree_count,
            "returned_worktree_count": returned_worktree_count,
            "worktree_targets_truncated": input.include_worktrees && visible_worktree_count > max_targets,
        },
        "targets": target_snapshot_targets(&visible_worktrees, input.include_worktrees, max_targets, cx),
        "target_count": target_count,
        "unaddressable_surfaces": input.include_gaps.then(unaddressable_surfaces),
        "target_contract": {
            "stable_for_current_snapshot": true,
            "use_with_actions": ["inspect_ui", "screenshot"],
            "do_not_use_for_direct_input": true,
            "input_actions_still_require_future_ui_snapshot": ["focus", "click", "type_text"],
            "coordinates_not_accepted_from_agents": true,
            "os_wide_targets_supported": false,
        },
        "required_flow": [
            "inspect_zed_window_context",
            "inspect_zed_pc_use_targets",
            "inspect_zed_pc_use_target_snapshot",
            "inspect_zed_pc_use_ui_snapshot_contract",
            "inspect_zed_pc_use_ui_snapshot",
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
            "future_input_requires_permission": true,
            "future_input_requires_fresh_target_receipt": true,
        },
        "next_actions": pc_use_snapshot_next_actions(status),
    })
}

fn target_snapshot_targets(
    visible_worktrees: &[Entity<project::Worktree>],
    include_worktrees: bool,
    max_targets: usize,
    cx: &App,
) -> Vec<Value> {
    let mut targets = Vec::with_capacity(1 + visible_worktrees.len().min(max_targets));
    targets.push(serde_json::json!({
        "target_id": "zed:workspace:active",
        "surface": "workspace",
        "label": "Active Zed workspace window",
        "kind": "window",
        "known_from": "zed_workspace_state",
        "safe_actions": ["inspect_ui", "screenshot"],
        "requires_permission_for_input": true,
        "input_ready": false,
    }));

    if include_worktrees {
        targets.extend(
            visible_worktrees
                .iter()
                .take(max_targets)
                .map(|worktree| worktree_target(worktree, cx)),
        );
    }

    targets
}

fn worktree_target(worktree: &Entity<project::Worktree>, cx: &App) -> Value {
    let worktree = worktree.read(cx);
    let worktree_id = format!("{:?}", worktree.id());
    let abs_path = worktree.abs_path().as_ref().to_path_buf();
    serde_json::json!({
        "target_id": format!("zed:project_panel:worktree:{worktree_id}"),
        "surface": "project_panel",
        "label": worktree.root_name_str(),
        "kind": if worktree.is_single_file() {
            "single_file_root"
        } else {
            "worktree_root"
        },
        "known_from": "project_visible_worktrees",
        "worktree_id": worktree_id,
        "root_name": worktree.root_name_str(),
        "abs_path": path_string(&abs_path),
        "filesystem": filesystem_summary(&abs_path),
        "scan_id": worktree.scan_id(),
        "safe_actions": ["inspect_ui", "screenshot"],
        "requires_permission_for_input": true,
        "input_ready": false,
    })
}

fn unaddressable_surfaces() -> Vec<Value> {
    vec![
        serde_json::json!({
            "surface": "editor",
            "reason": "active editor, cursor, selection, and buffer item ids require a future UI snapshot.",
            "future_target_source": "zed_ui_snapshot",
        }),
        serde_json::json!({
            "surface": "web_preview",
            "reason": "browser-page control should use WebPreview Browser tools; Zed-window PC-use requires a future preview item target id.",
            "future_target_source": "webpreview_session_snapshot",
        }),
        serde_json::json!({
            "surface": "agent_panel",
            "reason": "agent input targets require explicit user-selected panel handles before any typing can be considered.",
            "future_target_source": "zed_ui_snapshot",
        }),
        serde_json::json!({
            "surface": "right_panel",
            "reason": "asset-panel rows and controls require visible row/control ids from a future UI snapshot.",
            "future_target_source": "zed_ui_snapshot",
        }),
        serde_json::json!({
            "surface": "terminal",
            "reason": "terminal input is high-risk and must wait for a fresh focused terminal receipt plus explicit permission.",
            "future_target_source": "zed_ui_snapshot",
        }),
    ]
}

fn filesystem_summary(path: &Path) -> Value {
    let metadata = std::fs::metadata(path).ok();
    serde_json::json!({
        "exists": metadata.is_some(),
        "is_dir": metadata.as_ref().is_some_and(|metadata| metadata.is_dir()),
        "is_file": metadata.as_ref().is_some_and(|metadata| metadata.is_file()),
        "readonly": metadata
            .as_ref()
            .map(|metadata| metadata.permissions().readonly()),
    })
}

fn pc_use_snapshot_next_actions(status: &str) -> Vec<&'static str> {
    match status {
        "ready_for_safe_target_reference" => vec![
            "Use zed:workspace:active or project-panel worktree target ids only for read-only inspect/screenshot intent composition.",
            "Wait for a future Zed UI snapshot before composing focus, click, or type_text payloads against editor, panel, terminal, or WebPreview controls.",
        ],
        _ => vec![
            "Use only the active workspace target until a project is open.",
            "Keep direct input actions disabled until a visible UI target snapshot and permission receipt exist.",
        ],
    }
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
