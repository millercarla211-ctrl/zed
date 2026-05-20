use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, Entity, SharedString, Task};
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

const DX_LAUNCH_DEMO_RECIPES_SCHEMA: &str = "zed.dx.launch_demo.recipes.v1";

/// List launch-ready DX demo recipes with their required Agent tools and receipt gates.
///
/// The recipes are read-only orchestration contracts. They do not run DX tools, start local
/// services, dispatch browser input, mutate sources, or write receipts.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DxLaunchDemoRecipesToolInput {
    /// Optional focus term such as metasearch, media, forge, source, or serializer.
    pub focus: Option<String>,
    /// Include optional backup/restore and media flows in addition to the primary source flow.
    pub include_optional_flows: bool,
}

impl Default for DxLaunchDemoRecipesToolInput {
    fn default() -> Self {
        Self {
            focus: None,
            include_optional_flows: true,
        }
    }
}

pub struct DxLaunchDemoRecipesTool {
    project: Entity<Project>,
}

impl DxLaunchDemoRecipesTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for DxLaunchDemoRecipesTool {
    type Input = DxLaunchDemoRecipesToolInput;
    type Output = String;

    const NAME: &'static str = "list_dx_launch_demo_recipes";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Fetch
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            if let Some(focus) = clean_optional_text(input.focus) {
                return format!("List DX launch demo recipes for {focus}").into();
            }
        }
        "List DX launch demo recipes".into()
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
            let workspace_root = cx.update(|cx| workspace_root_for_project(&project, cx));
            let permission_values = vec![
                clean_optional_text(input.focus.clone()).unwrap_or_else(|| "all".to_string()),
                workspace_root
                    .as_ref()
                    .map(path_string)
                    .unwrap_or_else(|| "no visible workspace".to_string()),
            ];
            let authorize = cx.update(|cx| {
                let context = crate::ToolPermissionContext::new(Self::NAME, permission_values);
                event_stream.authorize(self.initial_title(Ok(input.clone()), cx), context, cx)
            });

            authorize.await.map_err(|error| error.to_string())?;
            let response = build_launch_demo_recipes(input, workspace_root);
            event_stream.update_fields(acp::ToolCallUpdateFields::new().title(format!(
                "Listed {} DX launch demo recipe(s)",
                response.summary.recipe_count
            )));

            serde_json::to_string_pretty(&response)
                .map_err(|error| format!("Failed to serialize DX launch demo recipes: {error}"))
        })
    }
}

#[derive(Clone, Debug, Serialize)]
struct DxLaunchDemoRecipes {
    schema: &'static str,
    generated_at_ms: u64,
    request: DxLaunchDemoRecipeRequestSummary,
    workspace: DxLaunchDemoWorkspace,
    summary: DxLaunchDemoRecipeSummary,
    receipt_roots: Vec<DxLaunchDemoReceiptRoot>,
    recipes: Vec<DxLaunchDemoRecipe>,
    safety: DxLaunchDemoRecipeSafety,
    next_action: String,
}

#[derive(Clone, Debug, Serialize)]
struct DxLaunchDemoRecipeRequestSummary {
    focus: Option<String>,
    include_optional_flows: bool,
}

#[derive(Clone, Debug, Serialize)]
struct DxLaunchDemoWorkspace {
    root: Option<String>,
    root_available: bool,
}

#[derive(Clone, Debug, Serialize)]
struct DxLaunchDemoRecipeSummary {
    status: &'static str,
    recipe_count: usize,
    primary_recipe_count: usize,
    optional_recipe_count: usize,
    blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct DxLaunchDemoReceiptRoot {
    label: &'static str,
    path: String,
    exists: bool,
}

#[derive(Clone, Debug, Serialize)]
struct DxLaunchDemoRecipe {
    id: &'static str,
    title: &'static str,
    priority: &'static str,
    status: &'static str,
    intent: &'static str,
    required_tools: Vec<&'static str>,
    receipt_contracts: Vec<&'static str>,
    steps: Vec<DxLaunchDemoRecipeStep>,
    proof_gates: Vec<&'static str>,
    blockers: Vec<String>,
    next_action: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct DxLaunchDemoRecipeStep {
    order: usize,
    tool: &'static str,
    action: &'static str,
    receipt_schema: Option<&'static str>,
    writes_receipt: bool,
    safety: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct DxLaunchDemoRecipeSafety {
    runs_external_processes: bool,
    starts_local_servers: bool,
    dispatches_browser_input: bool,
    writes_files: bool,
    mutates_sources: bool,
}

fn build_launch_demo_recipes(
    input: DxLaunchDemoRecipesToolInput,
    workspace_root: Option<PathBuf>,
) -> DxLaunchDemoRecipes {
    let focus = clean_optional_text(input.focus);
    let mut recipes = vec![
        metasearch_to_context_recipe(workspace_root.as_ref()),
        media_to_sources_recipe(workspace_root.as_ref()),
        forge_restore_to_sources_recipe(workspace_root.as_ref()),
    ];

    if !input.include_optional_flows {
        recipes.retain(|recipe| recipe.priority == "primary");
    }

    if let Some(focus) = focus.as_deref() {
        let focus = focus.to_ascii_lowercase();
        recipes.retain(|recipe| {
            recipe.id.contains(&focus)
                || recipe.title.to_ascii_lowercase().contains(&focus)
                || recipe.intent.to_ascii_lowercase().contains(&focus)
                || recipe
                    .required_tools
                    .iter()
                    .any(|tool| tool.contains(&focus))
        });
    }

    let mut blockers = Vec::new();
    if workspace_root.is_none() {
        blockers.push(
            "Open a workspace to write and inspect managed launch-demo receipts under tools/."
                .to_string(),
        );
    }
    if recipes.is_empty() {
        blockers.push("No DX launch demo recipe matched the requested focus.".to_string());
    }

    let primary_recipe_count = recipes
        .iter()
        .filter(|recipe| recipe.priority == "primary")
        .count();
    let optional_recipe_count = recipes.len().saturating_sub(primary_recipe_count);
    let status = if recipes.is_empty() {
        "empty"
    } else if workspace_root.is_some() {
        "ready"
    } else {
        "needs_workspace"
    };

    DxLaunchDemoRecipes {
        schema: DX_LAUNCH_DEMO_RECIPES_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxLaunchDemoRecipeRequestSummary {
            focus,
            include_optional_flows: input.include_optional_flows,
        },
        workspace: DxLaunchDemoWorkspace {
            root: workspace_root.as_ref().map(path_string),
            root_available: workspace_root.is_some(),
        },
        summary: DxLaunchDemoRecipeSummary {
            status,
            recipe_count: recipes.len(),
            primary_recipe_count,
            optional_recipe_count,
            blockers,
        },
        receipt_roots: receipt_roots(workspace_root.as_deref()),
        recipes,
        safety: DxLaunchDemoRecipeSafety {
            runs_external_processes: false,
            starts_local_servers: false,
            dispatches_browser_input: false,
            writes_files: false,
            mutates_sources: false,
        },
        next_action: "Run the primary recipe from the Agent tab, then pin the produced source/context receipts in the Sources rail for the demo."
            .to_string(),
    }
}

fn metasearch_to_context_recipe(workspace_root: Option<&PathBuf>) -> DxLaunchDemoRecipe {
    DxLaunchDemoRecipe {
        id: "metasearch-to-sources-to-context",
        title: "Metasearch Sources To Serializer/RLM Context",
        priority: "primary",
        status: recipe_status(workspace_root),
        intent: "Search DX metasearch, persist a cited source pack, attach the source pack through the Sources rail contract, compact it for serializer/RLM context, and produce an approved dry-run execution receipt.",
        required_tools: vec![
            "inspect_dx_metasearch",
            "search_dx_metasearch",
            "prepare_dx_source_attachment",
            "prepare_dx_metasearch_context",
            "plan_dx_serializer_rlm_execution",
        ],
        receipt_contracts: vec![
            "zed.dx.metasearch.source_pack_receipt.v1",
            "zed.dx.sources.attachment_receipt.v1",
            "zed.dx.serializer_rlm.context_receipt.v1",
            "zed.dx.serializer_rlm.execution_receipt.v1",
        ],
        steps: vec![
            step(
                1,
                "inspect_dx_metasearch",
                "Confirm service and engine readiness before routing demo searches.",
                None,
                false,
                "Read-only HTTP status request; does not start metasearch.",
            ),
            step(
                2,
                "search_dx_metasearch",
                "Run the demo query with write_source_pack_receipt=true.",
                Some("zed.dx.metasearch.source_pack_receipt.v1"),
                true,
                "Permissioned fetch; writes only managed source-pack receipts when requested.",
            ),
            step(
                3,
                "prepare_dx_source_attachment",
                "Package the latest metasearch source-pack receipt as a selected source manifest.",
                Some("zed.dx.sources.attachment_receipt.v1"),
                true,
                "Reads managed receipts and writes a manifest; embeds no binary payloads.",
            ),
            step(
                4,
                "prepare_dx_metasearch_context",
                "Build compact cited context from the source attachment receipt.",
                Some("zed.dx.serializer_rlm.context_receipt.v1"),
                true,
                "Reads managed source-pack receipt JSON and refuses unmanaged receipt paths.",
            ),
            step(
                5,
                "plan_dx_serializer_rlm_execution",
                "Create the approved dry-run reducer plan from the context receipt.",
                Some("zed.dx.serializer_rlm.execution_receipt.v1"),
                true,
                "Does not run external serializer/RLM crates or model calls.",
            ),
        ],
        proof_gates: vec![
            "Metasearch status is healthy or the missing-service state is reported.",
            "Latest source-pack receipt exists under tools/dx-metasearch/source-packs.",
            "Latest source attachment receipt exists under tools/dx-sources/attachments.",
            "Latest context receipt includes at least one cited item.",
            "Serializer/RLM execution plan remains dry-run unless explicitly approved.",
        ],
        blockers: recipe_blockers(workspace_root),
        next_action: "Use this as the flagship demo path before adding the external reducer runner gate.",
    }
}

fn media_to_sources_recipe(workspace_root: Option<&PathBuf>) -> DxLaunchDemoRecipe {
    DxLaunchDemoRecipe {
        id: "media-output-to-sources",
        title: "Media Tool Output To Sources Rail",
        priority: "optional",
        status: recipe_status(workspace_root),
        intent: "Plan, gate, and execute a safe media inspection or extraction, then expose produced media outputs as Sources rail attachments by path.",
        required_tools: vec![
            "plan_dx_media_tool",
            "gate_dx_media_tool_runner",
            "execute_dx_media_tool",
            "prepare_dx_source_attachment",
        ],
        receipt_contracts: vec![
            "zed.dx.media_tool.plan_receipt.v1",
            "zed.dx.media_tool.runner_gate_receipt.v1",
            "zed.dx.media_tool.execution_receipt.v1",
            "zed.dx.sources.attachment_receipt.v1",
        ],
        steps: vec![
            step(
                1,
                "plan_dx_media_tool",
                "Create a no-overwrite ffprobe/ffmpeg argument-vector plan for a local media file.",
                Some("zed.dx.media_tool.plan_receipt.v1"),
                true,
                "Plans only; no shell and no media execution.",
            ),
            step(
                2,
                "gate_dx_media_tool_runner",
                "Validate the approved media plan before execution.",
                Some("zed.dx.media_tool.runner_gate_receipt.v1"),
                true,
                "Checks runner readiness and managed output constraints.",
            ),
            step(
                3,
                "execute_dx_media_tool",
                "Run the approved no-shell media plan and hash produced outputs.",
                Some("zed.dx.media_tool.execution_receipt.v1"),
                true,
                "Executes only approved argument vectors and refuses overwrites/path traversal.",
            ),
            step(
                4,
                "prepare_dx_source_attachment",
                "Expose produced media files as path-only source attachments.",
                Some("zed.dx.sources.attachment_receipt.v1"),
                true,
                "References binary media by path only; no binary payloads enter model context.",
            ),
        ],
        proof_gates: vec![
            "Media plan receipt shows no shell interpolation and managed outputs.",
            "Runner gate receipt is approved before execution.",
            "Execution receipt contains produced file hashes or a graceful missing-tool error.",
            "Source attachment manifest marks media outputs as path-only attachments.",
        ],
        blockers: recipe_blockers(workspace_root),
        next_action: "Use this after the primary metasearch flow when the demo needs visible produced-file receipts.",
    }
}

fn forge_restore_to_sources_recipe(workspace_root: Option<&PathBuf>) -> DxLaunchDemoRecipe {
    DxLaunchDemoRecipe {
        id: "forge-restore-preview-to-sources",
        title: "Forge Restore Preview To Sources Rail",
        priority: "optional",
        status: recipe_status(workspace_root),
        intent: "Inspect Forge history, restore a backup into a managed preview, and attach the preview directory as a path-only source for review.",
        required_tools: vec![
            "inspect_dx_forge_history",
            "execute_dx_forge_restore",
            "prepare_dx_source_attachment",
        ],
        receipt_contracts: vec![
            "zed.dx.forge.history.v1",
            "zed.dx.forge.restore_execution_receipt.v1",
            "zed.dx.sources.attachment_receipt.v1",
        ],
        steps: vec![
            step(
                1,
                "inspect_dx_forge_history",
                "List managed Forge backup and restore receipts for the workspace.",
                Some("zed.dx.forge.history.v1"),
                false,
                "Read-only receipt scan; does not touch preview contents.",
            ),
            step(
                2,
                "execute_dx_forge_restore",
                "Restore an approved backup execution receipt into a managed preview directory.",
                Some("zed.dx.forge.restore_execution_receipt.v1"),
                true,
                "Verifies hashes and writes preview files only; no target mutation or permanent delete.",
            ),
            step(
                3,
                "prepare_dx_source_attachment",
                "Attach Forge restore previews as directory references in the Sources rail contract.",
                Some("zed.dx.sources.attachment_receipt.v1"),
                true,
                "Keeps restored preview directories as path-only references.",
            ),
        ],
        proof_gates: vec![
            "Forge history shows an available backup execution receipt.",
            "Restore execution receipt reports verified hashes.",
            "Restore preview lives under tools/dx-forge/restores.",
            "Source attachment manifest marks restore previews as directory references.",
        ],
        blockers: recipe_blockers(workspace_root),
        next_action: "Use this when the demo needs no-permanent-delete recovery proof before restore-to-target approvals exist.",
    }
}

fn step(
    order: usize,
    tool: &'static str,
    action: &'static str,
    receipt_schema: Option<&'static str>,
    writes_receipt: bool,
    safety: &'static str,
) -> DxLaunchDemoRecipeStep {
    DxLaunchDemoRecipeStep {
        order,
        tool,
        action,
        receipt_schema,
        writes_receipt,
        safety,
    }
}

fn receipt_roots(workspace_root: Option<&Path>) -> Vec<DxLaunchDemoReceiptRoot> {
    let Some(root) = workspace_root else {
        return Vec::new();
    };

    [
        (
            "metasearch source packs",
            root.join("tools")
                .join("dx-metasearch")
                .join("source-packs"),
        ),
        (
            "source attachments",
            root.join("tools").join("dx-sources").join("attachments"),
        ),
        (
            "metasearch context",
            root.join("tools").join("dx-metasearch").join("context"),
        ),
        (
            "serializer/RLM execution",
            root.join("tools")
                .join("dx-serializer-rlm")
                .join("execution"),
        ),
        (
            "media executions",
            root.join("tools").join("dx-media").join("executions"),
        ),
        (
            "Forge restores",
            root.join("tools").join("dx-forge").join("restores"),
        ),
    ]
    .into_iter()
    .map(|(label, path)| DxLaunchDemoReceiptRoot {
        label,
        exists: path.exists(),
        path: path_string(&path),
    })
    .collect()
}

fn recipe_status(workspace_root: Option<&PathBuf>) -> &'static str {
    if workspace_root.is_some() {
        "ready"
    } else {
        "needs_workspace"
    }
}

fn recipe_blockers(workspace_root: Option<&PathBuf>) -> Vec<String> {
    if workspace_root.is_some() {
        Vec::new()
    } else {
        vec!["No visible workspace root is available for managed demo receipts.".to_string()]
    }
}

fn workspace_root_for_project(project: &Entity<Project>, cx: &App) -> Option<PathBuf> {
    project
        .read(cx)
        .visible_worktrees(cx)
        .next()
        .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf())
}

fn clean_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
