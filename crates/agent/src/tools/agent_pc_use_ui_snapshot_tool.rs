use crate::{
    AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_TOOL_NAME, AgentTool, PC_USE_FUTURE_UI_TARGET_PREFIXES,
    ToolCallEventStream, ToolInput,
};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, Entity, SharedString, Task};
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    path::Path,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME: &str = "inspect_zed_pc_use_ui_snapshot";
pub const AGENT_PC_USE_UI_SNAPSHOT_SCHEMA: &str = "zed.agent_plugins.pc_use.ui_snapshot.v1";

/// Returns the safest current Zed UI snapshot available to the Agent today.
///
/// This is a partial UI snapshot built from project state, not a pixel/layout tree. It publishes
/// real read-only workspace and project-panel targets, plus explicit live-UI gaps for editor,
/// WebPreview, Agent Panel, right panel, title/status bars, and terminal targets. It never focuses
/// Zed, dispatches input, takes screenshots, launches processes, or controls the desktop.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPcUseUiSnapshotToolInput {
    /// Include the active workspace target.
    pub include_workspace: bool,
    /// Include visible project-panel worktree targets.
    pub include_project_panel: bool,
    /// Include unavailable live UI target families as explicit gaps.
    pub include_live_ui_gaps: bool,
    /// Maximum project-panel target rows to include.
    pub max_targets: usize,
}

impl Default for AgentPcUseUiSnapshotToolInput {
    fn default() -> Self {
        Self {
            include_workspace: true,
            include_project_panel: true,
            include_live_ui_gaps: true,
            max_targets: 24,
        }
    }
}

pub struct AgentPcUseUiSnapshotTool {
    project: Entity<Project>,
}

impl AgentPcUseUiSnapshotTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentPcUseUiSnapshotTool {
    type Input = AgentPcUseUiSnapshotToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Inspect Zed UI snapshot".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let result = cx.update(|cx| inspect_pc_use_ui_snapshot(&self.project, input, cx));
            let output = serde_json::to_string_pretty(&result)
                .map_err(|error| format!("Failed to serialize PC-use UI snapshot: {error}"))?;

            event_stream
                .update_fields(acp::ToolCallUpdateFields::new().title("Inspected Zed UI snapshot"));

            Ok(output)
        })
    }
}

fn inspect_pc_use_ui_snapshot(
    project: &Entity<Project>,
    input: AgentPcUseUiSnapshotToolInput,
    cx: &App,
) -> Value {
    let generated_at_ms = current_epoch_millis();
    let snapshot_id = format!("zed-ui-snapshot-{generated_at_ms}");
    let max_targets = input.max_targets.clamp(1, 96);
    let visible_worktrees = project.read(cx).visible_worktrees(cx).collect::<Vec<_>>();
    let project_root = visible_worktrees
        .first()
        .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf());
    let returned_project_panel_targets = if input.include_project_panel {
        visible_worktrees.len().min(max_targets)
    } else {
        0
    };
    let workspace_target_count = if input.include_workspace { 1 } else { 0 };
    let target_count = workspace_target_count + returned_project_panel_targets;
    let status = if visible_worktrees.is_empty() {
        "partial_workspace_snapshot"
    } else {
        "partial_project_state_snapshot"
    };

    serde_json::json!({
        "schema": AGENT_PC_USE_UI_SNAPSHOT_SCHEMA,
        "snapshot_id": snapshot_id,
        "generated_at_ms": generated_at_ms,
        "status": status,
        "tool": {
            "name": AGENT_PC_USE_UI_SNAPSHOT_TOOL_NAME,
            "kind": "read_only",
        },
        "snapshot_source": {
            "source": "project_visible_worktrees",
            "live_ui_tree_available": false,
            "pixel_bounds_available": false,
            "focus_state_available": false,
            "safe_current_targets_only": true,
        },
        "workspace": {
            "project_root": project_root.as_ref().map(path_string),
            "visible_worktree_count": visible_worktrees.len(),
            "returned_project_panel_target_count": returned_project_panel_targets,
            "project_panel_targets_truncated": input.include_project_panel
                && visible_worktrees.len() > max_targets,
        },
        "targets": ui_snapshot_targets(
            &snapshot_id,
            &visible_worktrees,
            input.include_workspace,
            input.include_project_panel,
            max_targets,
            cx
        ),
        "target_count": target_count,
        "live_ui_gaps": input.include_live_ui_gaps.then(live_ui_gaps),
        "input_readiness": {
            "snapshot_targets_input_ready": false,
            "snapshot_targets_safe_for": ["inspect_ui", "screenshot"],
            "snapshot_targets_not_safe_for": ["focus", "click", "type_text"],
            "current_snapshot_target_prefixes": [
                "zed:workspace:active",
                "zed:project_panel:worktree:"
            ],
            "future_input_ready_target_prefixes": PC_USE_FUTURE_UI_TARGET_PREFIXES
                .iter()
                .map(|(prefix, _, _)| *prefix)
                .collect::<Vec<_>>(),
            "future_input_requires_fresh_live_ui_snapshot": true,
            "future_input_payload_requires_target_snapshot_id": true,
            "coordinates_accepted_from_agents": false,
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
        "next_actions": pc_use_ui_snapshot_next_actions(status),
    })
}

fn ui_snapshot_targets(
    snapshot_id: &str,
    visible_worktrees: &[Entity<project::Worktree>],
    include_workspace: bool,
    include_project_panel: bool,
    max_targets: usize,
    cx: &App,
) -> Vec<Value> {
    let workspace_target_count = if include_workspace { 1 } else { 0 };
    let project_panel_target_count = if include_project_panel {
        visible_worktrees.len().min(max_targets)
    } else {
        0
    };
    let mut targets = Vec::with_capacity(workspace_target_count + project_panel_target_count);

    if include_workspace {
        targets.push(serde_json::json!({
            "target_id": "zed:workspace:active",
            "snapshot_id": snapshot_id,
            "surface": "workspace",
            "label": "Active Zed workspace window",
            "kind": "workspace_window",
            "visible": true,
            "enabled": true,
            "focused": Value::Null,
            "bounds": Value::Null,
            "bounds_known": false,
            "known_from": "zed_project_context",
            "safe_actions": ["inspect_ui", "screenshot"],
            "input_ready": false,
            "risk": "low",
        }));
    }

    if include_project_panel {
        targets.extend(
            visible_worktrees
                .iter()
                .take(max_targets)
                .map(|worktree| project_panel_worktree_target(snapshot_id, worktree, cx)),
        );
    }

    targets
}

fn project_panel_worktree_target(
    snapshot_id: &str,
    worktree: &Entity<project::Worktree>,
    cx: &App,
) -> Value {
    let worktree = worktree.read(cx);
    let worktree_id = format!("{:?}", worktree.id());
    let abs_path = worktree.abs_path().as_ref().to_path_buf();

    serde_json::json!({
        "target_id": format!("zed:project_panel:worktree:{worktree_id}"),
        "snapshot_id": snapshot_id,
        "surface": "project_panel",
        "label": worktree.root_name_str(),
        "kind": if worktree.is_single_file() {
            "single_file_root"
        } else {
            "worktree_root"
        },
        "visible": true,
        "enabled": true,
        "focused": Value::Null,
        "bounds": Value::Null,
        "bounds_known": false,
        "known_from": "project_visible_worktrees",
        "worktree_id": worktree_id,
        "root_name": worktree.root_name_str(),
        "abs_path": path_string(&abs_path),
        "filesystem": filesystem_summary(&abs_path),
        "scan_id": worktree.scan_id(),
        "safe_actions": ["inspect_ui", "screenshot"],
        "input_ready": false,
        "risk": "low",
        "future_live_row_namespace": "zed:project_panel:entry:",
    })
}

fn live_ui_gaps() -> Vec<Value> {
    vec![
        live_ui_gap(
            "editor",
            "zed:editor:",
            &["inspect_ui", "screenshot", "focus", "click", "type_text"],
            "active editor buffers, selections, cursor state, and visible text bounds require a live workspace UI snapshot source.",
        ),
        live_ui_gap(
            "web_preview",
            "zed:web_preview:",
            &["inspect_ui", "screenshot", "focus", "click"],
            "WebPreview page control should use Browser plugin tools today; Zed-window PC-use still needs a visible preview item target.",
        ),
        live_ui_gap(
            "agent_panel",
            "zed:agent_panel:",
            &["inspect_ui", "screenshot", "focus", "click", "type_text"],
            "Agent input fields need user-selected panel handles and explicit permission before typing can be considered.",
        ),
        live_ui_gap(
            "right_panel",
            "zed:right_panel:",
            &["inspect_ui", "screenshot", "focus", "click"],
            "Right panel rows and controls need live row/control ids before click payloads can be input-ready.",
        ),
        live_ui_gap(
            "terminal",
            "zed:terminal:",
            &["inspect_ui", "screenshot", "focus", "click", "type_text"],
            "Terminal targets are high-risk and must wait for live focus receipts plus explicit permission.",
        ),
        live_ui_gap(
            "title_bar",
            "zed:title_bar:",
            &["inspect_ui", "screenshot", "focus", "click"],
            "Title-bar controls need live control ids and bounds from the window UI layer.",
        ),
        live_ui_gap(
            "status_bar",
            "zed:status_bar:",
            &["inspect_ui", "screenshot", "focus", "click"],
            "Status-bar controls need live control ids and bounds from the window UI layer.",
        ),
    ]
}

fn live_ui_gap(
    surface: &str,
    future_namespace: &str,
    future_actions: &[&str],
    reason: &str,
) -> Value {
    serde_json::json!({
        "surface": surface,
        "current_live_targets_available": false,
        "future_namespace": future_namespace,
        "future_actions": future_actions,
        "reason": reason,
        "requires_visible_bounds": true,
        "requires_fresh_snapshot_receipt": true,
        "contract_tool_name": AGENT_PC_USE_UI_SNAPSHOT_CONTRACT_TOOL_NAME,
    })
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

fn pc_use_ui_snapshot_next_actions(status: &str) -> Vec<&'static str> {
    match status {
        "partial_project_state_snapshot" => vec![
            "Use these snapshot targets only for read-only inspect_ui or screenshot payload composition.",
            "Use Browser plugin tools for WebPreview page control today.",
            "Wait for a future live Zed UI snapshot before focus, click, or type_text against editor, panel, terminal, or WebPreview controls.",
        ],
        _ => vec![
            "Open a workspace or project to expose project-panel targets.",
            "Keep focus, click, and type_text disabled until a live Zed UI snapshot target receipt exists.",
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
