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
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const AGENT_PC_USE_INSPECT_TOOL_NAME: &str = "inspect_zed_window_context";

/// Reads safe Zed-window and workspace context for future PC-use workflows.
///
/// This tool is intentionally read-only. It does not take screenshots, focus panes, click, type,
/// launch desktop automation, or inspect OS-wide windows. Use it before any future permissioned
/// Zed-window or desktop-control tool so the agent can reason from the current workspace context.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPcUseInspectToolInput {
    /// Include visible project worktree roots.
    pub include_worktrees: bool,
    /// Include managed PC-use/plugin roots and whether they exist.
    pub include_managed_roots: bool,
    /// Maximum visible worktrees to include in the response.
    pub max_worktrees: usize,
}

impl Default for AgentPcUseInspectToolInput {
    fn default() -> Self {
        Self {
            include_worktrees: true,
            include_managed_roots: true,
            max_worktrees: 12,
        }
    }
}

pub struct AgentPcUseInspectTool {
    project: Entity<Project>,
}

impl AgentPcUseInspectTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for AgentPcUseInspectTool {
    type Input = AgentPcUseInspectToolInput;
    type Output = String;

    const NAME: &'static str = AGENT_PC_USE_INSPECT_TOOL_NAME;

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Inspect Zed window context".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let result = cx.update(|cx| inspect_zed_window_context(&self.project, input, cx));
            let output = serde_json::to_string_pretty(&result)
                .map_err(|error| format!("Failed to serialize Zed window context: {error}"))?;

            event_stream.update_fields(
                acp::ToolCallUpdateFields::new().title("Inspected Zed window context"),
            );

            Ok(output)
        })
    }
}

fn inspect_zed_window_context(
    project: &Entity<Project>,
    input: AgentPcUseInspectToolInput,
    cx: &App,
) -> Value {
    let max_worktrees = input.max_worktrees.clamp(1, 32);
    let visible_worktrees = project.read(cx).visible_worktrees(cx).collect::<Vec<_>>();
    let project_root = visible_worktrees
        .first()
        .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf());
    let visible_worktree_count = visible_worktrees.len();
    let worktrees = input.include_worktrees.then(|| {
        visible_worktrees
            .iter()
            .take(max_worktrees)
            .map(|worktree| {
                let worktree = worktree.read(cx);
                let abs_path = worktree.abs_path().as_ref().to_path_buf();
                serde_json::json!({
                    "id": format!("{:?}", worktree.id()),
                    "root_name": worktree.root_name_str(),
                    "abs_path": path_string(&abs_path),
                    "is_single_file": worktree.is_single_file(),
                    "scan_id": worktree.scan_id(),
                    "root": filesystem_summary(&abs_path),
                })
            })
            .collect::<Vec<_>>()
    });
    let managed_roots = input
        .include_managed_roots
        .then(|| managed_pc_use_roots(project_root.as_ref()));
    let status = if visible_worktree_count == 0 {
        "no_visible_worktree"
    } else {
        "ready_for_read_only_context"
    };

    serde_json::json!({
        "schema": "zed.agent_plugins.pc_use.zed_window_context.v1",
        "generated_at_ms": current_epoch_millis(),
        "status": status,
        "tool": {
            "name": AGENT_PC_USE_INSPECT_TOOL_NAME,
            "kind": "read_only",
        },
        "workspace": {
            "project_root": project_root.as_ref().map(path_string),
            "visible_worktree_count": visible_worktree_count,
            "returned_worktree_count": worktrees.as_ref().map(Vec::len),
            "worktrees_truncated": visible_worktree_count > max_worktrees,
            "worktrees": worktrees,
        },
        "managed_roots": managed_roots,
        "capabilities": {
            "safe_ui_metadata": true,
            "zed_window_screenshot": "planned_permission_gate",
            "zed_window_focus": "planned_permission_gate",
            "zed_window_click": "planned_permission_gate",
            "zed_window_type": "planned_permission_gate",
            "os_wide_desktop_automation": "blocked_by_default",
        },
        "safety": {
            "read_only": true,
            "takes_screenshot": false,
            "focuses_window": false,
            "dispatches_mouse": false,
            "dispatches_keyboard": false,
            "launches_process": false,
            "os_wide_control": false,
            "requires_permission_before_future_input": true,
        },
        "next_actions": pc_use_next_actions(status),
    })
}

fn managed_pc_use_roots(project_root: Option<&PathBuf>) -> Value {
    let zed_plugin_root = data_dir().join("agent-plugins");
    serde_json::json!({
        "workspace": project_root.map(|root| {
            let plugin_root = root.join("tools").join("agent-plugins");
            serde_json::json!({
                "agent_plugin_root": root_summary("workspace_agent_plugins", plugin_root.clone()),
                "pc_use_root": root_summary("workspace_pc_use", plugin_root.join("pc-use")),
            })
        }),
        "zed_data": {
            "agent_plugin_root": root_summary("zed_data_agent_plugins", zed_plugin_root.clone()),
            "pc_use_root": root_summary("zed_data_pc_use", zed_plugin_root.join("pc-use")),
        },
    })
}

fn filesystem_summary(path: &Path) -> Value {
    let metadata = fs::metadata(path).ok();
    serde_json::json!({
        "exists": metadata.is_some(),
        "is_dir": metadata.as_ref().is_some_and(|metadata| metadata.is_dir()),
        "is_file": metadata.as_ref().is_some_and(|metadata| metadata.is_file()),
        "readonly": metadata
            .as_ref()
            .map(|metadata| metadata.permissions().readonly()),
    })
}

fn root_summary(kind: &str, path: PathBuf) -> Value {
    let metadata = fs::metadata(&path).ok();
    serde_json::json!({
        "kind": kind,
        "path": path_string(&path),
        "exists": metadata.is_some(),
        "is_dir": metadata.as_ref().is_some_and(|metadata| metadata.is_dir()),
        "readonly": metadata
            .as_ref()
            .map(|metadata| metadata.permissions().readonly()),
    })
}

fn pc_use_next_actions(status: &str) -> Vec<&'static str> {
    match status {
        "ready_for_read_only_context" => vec![
            "Use this context before any future Zed-window screenshot, focus, click, or type request.",
            "Keep OS-wide desktop automation disabled unless the user explicitly grants a separate permission.",
        ],
        _ => vec![
            "Open a workspace before using future PC-use tools.",
            "Keep PC-use actions read-only until a visible target and explicit permission gate exist.",
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
