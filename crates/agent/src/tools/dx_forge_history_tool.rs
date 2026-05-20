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

const DX_FORGE_HISTORY_SCHEMA: &str = "zed.dx.forge.history.v1";

/// Inspect managed DX Forge receipts for panel-facing history.
///
/// This read-only tool scans the workspace or Zed-data `dx-forge` receipt folders and returns a
/// compact timeline of safety policies, runner gates, backup executions, and restore executions.
/// It never writes files, runs commands, shells out, or mutates project data.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxForgeHistoryToolInput {
    /// Prefer workspace-local Forge receipts under `<workspace>/tools`; falls back to Zed data.
    pub artifact_root_mode: DxForgeHistoryArtifactRootMode,
    /// Maximum receipt entries to return after newest-first sorting.
    pub max_entries: usize,
}

impl Default for DxForgeHistoryToolInput {
    fn default() -> Self {
        Self {
            artifact_root_mode: DxForgeHistoryArtifactRootMode::Workspace,
            max_entries: 40,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxForgeHistoryArtifactRootMode {
    #[default]
    Workspace,
    ZedData,
}

pub struct DxForgeHistoryTool {
    project: Entity<Project>,
}

impl DxForgeHistoryTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxForgeHistoryTool {
    type Input = DxForgeHistoryToolInput;
    type Output = String;

    const NAME: &'static str = "inspect_dx_forge_history";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Inspect DX Forge history".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        let project = self.project.clone();

        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| error.to_string())?;
            let artifact_target = {
                let project_root = cx.update(|cx| workspace_root_for_project(&project, cx));
                DxForgeHistoryArtifactTarget::new(project_root, input.artifact_root_mode)
            };

            let authorize = cx.update(|cx| {
                let context = crate::ToolPermissionContext::new(
                    Self::NAME,
                    vec![path_string(&artifact_target.allowed_root)],
                );
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;
            let response = artifact_target.inspect(input.max_entries);
            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Inspected DX Forge history: {} entries",
                response.entries.len()
            )));

            serde_json::to_string_pretty(&response)
                .map_err(|error| format!("Failed to serialize DX Forge history: {error}"))
        })
    }
}

#[derive(Clone, Debug, Serialize)]
struct DxForgeHistoryResponse {
    schema: &'static str,
    generated_at_ms: u128,
    root_mode: String,
    project_root: Option<String>,
    artifact_root: String,
    status: String,
    max_entries: usize,
    scanned_receipts: usize,
    returned_entries: usize,
    counts: DxForgeHistoryCounts,
    entries: Vec<DxForgeHistoryEntry>,
    blockers: Vec<String>,
    next_action: String,
}

#[derive(Clone, Debug, Default, Serialize)]
struct DxForgeHistoryCounts {
    safety_policy: usize,
    runner_gate: usize,
    backup_execution: usize,
    restore_execution: usize,
    unknown: usize,
}

#[derive(Clone, Debug, Serialize)]
struct DxForgeHistoryEntry {
    kind: String,
    schema: String,
    source_tool: Option<String>,
    path: String,
    is_latest: bool,
    written_at_ms: Option<u128>,
    modified_at_ms: Option<u128>,
    status: Option<String>,
    operation: Option<String>,
    target_path: Option<String>,
    archive_path: Option<String>,
    manifest_path: Option<String>,
    restore_destination_root: Option<String>,
    next_action: Option<String>,
}

struct DxForgeHistoryArtifactTarget {
    root_mode: DxForgeHistoryArtifactRootMode,
    project_root: Option<PathBuf>,
    allowed_root: PathBuf,
}

impl DxForgeHistoryArtifactTarget {
    fn new(project_root: Option<PathBuf>, root_mode: DxForgeHistoryArtifactRootMode) -> Self {
        let use_workspace = matches!(root_mode, DxForgeHistoryArtifactRootMode::Workspace)
            && project_root.is_some();
        let allowed_root = if use_workspace {
            project_root
                .as_ref()
                .expect("workspace root checked above")
                .join("tools")
                .join("dx-forge")
        } else {
            data_dir().join("dx-forge")
        };

        Self {
            root_mode,
            project_root,
            allowed_root,
        }
    }

    fn inspect(&self, max_entries: usize) -> DxForgeHistoryResponse {
        let mut blockers = Vec::new();
        let max_entries = max_entries.clamp(1, 200);
        let mut entries = match collect_history_entries(&self.allowed_root) {
            Ok(entries) => entries,
            Err(error) => {
                blockers.push(error);
                Vec::new()
            }
        };

        let scanned_receipts = entries.len();
        entries.sort_by(|left, right| {
            right
                .written_at_ms
                .or(right.modified_at_ms)
                .cmp(&left.written_at_ms.or(left.modified_at_ms))
                .then_with(|| left.path.cmp(&right.path))
        });
        entries.truncate(max_entries);
        let counts = count_entries(&entries);
        let status = if blockers.is_empty() {
            "ready"
        } else {
            "blocked"
        };

        DxForgeHistoryResponse {
            schema: DX_FORGE_HISTORY_SCHEMA,
            generated_at_ms: current_epoch_millis(),
            root_mode: self.root_mode_label().to_string(),
            project_root: self.project_root.as_ref().map(path_string),
            artifact_root: path_string(&self.allowed_root),
            status: status.to_string(),
            max_entries,
            scanned_receipts,
            returned_entries: entries.len(),
            counts,
            entries,
            blockers,
            next_action:
                "Use this read-only history contract to populate the Forge panel timeline and restore audit UI."
                    .to_string(),
        }
    }

    fn root_mode_label(&self) -> &'static str {
        match self.root_mode {
            DxForgeHistoryArtifactRootMode::Workspace if self.project_root.is_some() => "workspace",
            DxForgeHistoryArtifactRootMode::Workspace => "zed_data_fallback",
            DxForgeHistoryArtifactRootMode::ZedData => "zed_data",
        }
    }
}

fn collect_history_entries(root: &Path) -> Result<Vec<DxForgeHistoryEntry>, String> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    if !root.is_dir() {
        return Err(format!(
            "DX Forge history root {} is not a directory.",
            root.display()
        ));
    }

    let mut entries = Vec::new();
    for folder in ["safety-policies", "runner-gates", "executions", "restores"] {
        let folder_path = root.join(folder);
        if !folder_path.exists() {
            continue;
        }
        collect_json_receipts(&folder_path, &mut entries)?;
    }

    Ok(entries)
}

fn collect_json_receipts(
    folder: &Path,
    entries: &mut Vec<DxForgeHistoryEntry>,
) -> Result<(), String> {
    let mut stack = vec![folder.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(&path).map_err(|error| {
            format!(
                "Failed to read DX Forge history {}: {error}",
                path.display()
            )
        })? {
            let entry = entry.map_err(|error| {
                format!(
                    "Failed to enumerate DX Forge history {}: {error}",
                    path.display()
                )
            })?;
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|file_name| file_name.to_str()) != Some("preview") {
                    stack.push(path);
                }
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
                if let Some(history_entry) = read_history_entry(&path) {
                    entries.push(history_entry);
                }
            }
        }
    }

    Ok(())
}

fn read_history_entry(path: &Path) -> Option<DxForgeHistoryEntry> {
    let bytes = fs::read(path).ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    let schema = string_field(&value, &["schema"]).unwrap_or_else(|| "unknown".to_string());
    let source_tool = string_field(&value, &["source_tool"]);
    let kind = history_kind(&schema, &value);
    if kind == "unknown" && !schema.starts_with("zed.dx.forge.") {
        return None;
    }
    let modified_at_ms = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(system_time_to_millis);

    Some(DxForgeHistoryEntry {
        kind,
        schema,
        source_tool,
        path: path_string(path),
        is_latest: path
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .is_some_and(|file_name| file_name.starts_with("latest-")),
        written_at_ms: value
            .get("written_at_ms")
            .and_then(Value::as_u64)
            .map(u128::from),
        modified_at_ms,
        status: history_status(&value),
        operation: history_operation(&value),
        target_path: history_target_path(&value),
        archive_path: history_archive_path(&value),
        manifest_path: history_manifest_path(&value),
        restore_destination_root: history_restore_destination_root(&value),
        next_action: string_field(&value, &["next_action"])
            .or_else(|| string_field(&value, &["restore_execution", "next_action"]))
            .or_else(|| string_field(&value, &["backup_execution", "next_action"]))
            .or_else(|| string_field(&value, &["runner_gate", "next_action"]))
            .or_else(|| string_field(&value, &["forge_safety_policy", "next_action"])),
    })
}

fn history_kind(schema: &str, value: &Value) -> String {
    if schema.contains(".restore_execution") || value.get("restore_execution").is_some() {
        "restore_execution"
    } else if schema.contains(".backup_execution") || value.get("backup_execution").is_some() {
        "backup_execution"
    } else if schema.contains(".backup_runner_gate") || value.get("runner_gate").is_some() {
        "runner_gate"
    } else if schema.contains(".safety_policy") || value.get("forge_safety_policy").is_some() {
        "safety_policy"
    } else {
        "unknown"
    }
    .to_string()
}

fn history_status(value: &Value) -> Option<String> {
    string_field(value, &["status"])
        .or_else(|| string_field(value, &["restore_execution", "restore", "status"]))
        .or_else(|| string_field(value, &["backup_execution", "execution", "status"]))
        .or_else(|| string_field(value, &["runner_gate", "validation", "status"]))
        .or_else(|| string_field(value, &["forge_safety_policy", "policy", "status"]))
}

fn history_operation(value: &Value) -> Option<String> {
    string_field(value, &["operation"])
        .or_else(|| string_field(value, &["restore_execution", "backup", "operation"]))
        .or_else(|| string_field(value, &["backup_execution", "gate", "operation"]))
        .or_else(|| string_field(value, &["runner_gate", "policy", "operation"]))
        .or_else(|| string_field(value, &["forge_safety_policy", "policy", "operation"]))
}

fn history_target_path(value: &Value) -> Option<String> {
    string_field(value, &["restore_execution", "backup", "target_path"])
        .or_else(|| string_field(value, &["backup_execution", "gate", "target_path"]))
        .or_else(|| string_field(value, &["runner_gate", "policy", "target_path"]))
        .or_else(|| string_field(value, &["forge_safety_policy", "policy", "target_path"]))
}

fn history_archive_path(value: &Value) -> Option<String> {
    string_field(value, &["archive_path_written"])
        .or_else(|| string_field(value, &["backup_archive_path"]))
        .or_else(|| string_field(value, &["restore_execution", "backup", "archive_path"]))
        .or_else(|| string_field(value, &["backup_execution", "execution", "archive_path"]))
        .or_else(|| string_field(value, &["runner_gate", "policy", "planned_archive_path"]))
        .or_else(|| {
            string_field(
                value,
                &["forge_safety_policy", "policy", "planned_archive_path"],
            )
        })
}

fn history_manifest_path(value: &Value) -> Option<String> {
    string_field(value, &["manifest_path_written"])
        .or_else(|| string_field(value, &["backup_manifest_path"]))
        .or_else(|| string_field(value, &["restore_execution", "backup", "manifest_path"]))
        .or_else(|| string_field(value, &["backup_execution", "execution", "manifest_path"]))
        .or_else(|| string_field(value, &["runner_gate", "policy", "planned_manifest_path"]))
        .or_else(|| {
            string_field(
                value,
                &["forge_safety_policy", "policy", "planned_manifest_path"],
            )
        })
}

fn history_restore_destination_root(value: &Value) -> Option<String> {
    string_field(value, &["restore_destination_root"]).or_else(|| {
        string_field(
            value,
            &["restore_execution", "restore", "restore_destination_root"],
        )
    })
}

fn count_entries(entries: &[DxForgeHistoryEntry]) -> DxForgeHistoryCounts {
    let mut counts = DxForgeHistoryCounts::default();
    for entry in entries {
        match entry.kind.as_str() {
            "safety_policy" => counts.safety_policy += 1,
            "runner_gate" => counts.runner_gate += 1,
            "backup_execution" => counts.backup_execution += 1,
            "restore_execution" => counts.restore_execution += 1,
            _ => counts.unknown += 1,
        }
    }
    counts
}

fn workspace_root_for_project(project: &Entity<Project>, cx: &App) -> Option<PathBuf> {
    project
        .read(cx)
        .visible_worktrees(cx)
        .next()
        .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf())
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

fn string_field(value: &Value, path: &[&str]) -> Option<String> {
    value_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

fn system_time_to_millis(time: SystemTime) -> Option<u128> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis())
}

fn current_epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
