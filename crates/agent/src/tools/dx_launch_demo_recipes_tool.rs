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
        serializer_rlm_external_execution_recipe(workspace_root.as_ref()),
        media_to_sources_recipe(workspace_root.as_ref()),
        forge_restore_to_sources_recipe(workspace_root.as_ref()),
        forge_restore_approval_recipe(workspace_root.as_ref()),
        forge_restore_target_plan_recipe(workspace_root.as_ref()),
        runtime_proof_import_recipe(workspace_root.as_ref()),
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
        next_action: "Run the primary recipe from the Agent tab, then pin the produced source/context/reduced-context receipts in the Sources rail for the demo."
            .to_string(),
    }
}

fn metasearch_to_context_recipe(workspace_root: Option<&PathBuf>) -> DxLaunchDemoRecipe {
    DxLaunchDemoRecipe {
        id: "metasearch-to-sources-to-context",
        title: "Metasearch Sources To Serializer/RLM Context",
        priority: "primary",
        status: recipe_status(workspace_root),
        intent: "Search DX metasearch, persist a cited source pack, attach the source pack through the Sources rail contract, compact it for serializer/RLM context, and produce approved execution, runner-gate, reduced-context, and dry-run execution-preview receipts.",
        required_tools: vec![
            "inspect_dx_metasearch",
            "search_dx_metasearch",
            "prepare_dx_source_attachment",
            "prepare_dx_metasearch_context",
            "plan_dx_serializer_rlm_execution",
            "gate_dx_serializer_rlm_runner",
            "write_dx_serializer_rlm_reduced_context",
            "preview_dx_serializer_rlm_reducer_execution",
        ],
        receipt_contracts: vec![
            "zed.dx.metasearch.source_pack_receipt.v1",
            "zed.dx.sources.attachment_receipt.v1",
            "zed.dx.serializer_rlm.context_receipt.v1",
            "zed.dx.serializer_rlm.execution_receipt.v1",
            "zed.dx.serializer_rlm.runner_gate_receipt.v1",
            "zed.dx.serializer_rlm.reduced_context_receipt.v1",
            "zed.dx.serializer_rlm.execution_preview_receipt.v1",
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
            step(
                6,
                "gate_dx_serializer_rlm_runner",
                "Validate the execution receipt and model-call policy before any reducer runner is wired.",
                Some("zed.dx.serializer_rlm.runner_gate_receipt.v1"),
                true,
                "Does not run serializer/RLM code, cargo, external processes, or model calls.",
            ),
            step(
                7,
                "write_dx_serializer_rlm_reduced_context",
                "Write a deterministic reduced-context receipt from the ready runner gate and context receipt.",
                Some("zed.dx.serializer_rlm.reduced_context_receipt.v1"),
                true,
                "Truncates existing context only; does not run serializer/RLM code, cargo, external processes, or model calls.",
            ),
            step(
                8,
                "preview_dx_serializer_rlm_reducer_execution",
                "Write a dry-run preview receipt describing the external reducer execution that would be allowed later.",
                Some("zed.dx.serializer_rlm.execution_preview_receipt.v1"),
                true,
                "Writes preview receipts only; does not run serializer/RLM code, cargo, external processes, network, or model calls.",
            ),
        ],
        proof_gates: vec![
            "Metasearch status is healthy or the missing-service state is reported.",
            "Latest source-pack receipt exists under tools/dx-metasearch/source-packs.",
            "Latest source attachment receipt exists under tools/dx-sources/attachments.",
            "Latest context receipt includes at least one cited item.",
            "Serializer/RLM execution plan remains dry-run unless explicitly approved.",
            "Runner gate receipt is ready only after explicit runner and model-call approvals.",
            "Reduced-context receipt includes cited source summaries and no external execution evidence.",
            "Execution preview receipt stays dry-run only and reports no external process or model-call execution.",
        ],
        blockers: recipe_blockers(workspace_root),
        next_action: "Use this as the flagship demo path before optionally running the separately approved external serializer/RLM executor.",
    }
}

fn serializer_rlm_external_execution_recipe(
    workspace_root: Option<&PathBuf>,
) -> DxLaunchDemoRecipe {
    DxLaunchDemoRecipe {
        id: "serializer-rlm-approved-external-execution",
        title: "Serializer/RLM Approved External Execution",
        priority: "optional",
        status: recipe_status(workspace_root),
        intent: "Run an already reviewed serializer/RLM execution preview through an explicit no-shell command vector, then persist stdout/stderr previews and hashes as managed execution receipts.",
        required_tools: vec![
            "preview_dx_serializer_rlm_reducer_execution",
            "execute_dx_serializer_rlm_reducer",
        ],
        receipt_contracts: vec![
            "zed.dx.serializer_rlm.execution_preview_receipt.v1",
            "zed.dx.serializer_rlm.reduced_context_receipt.v1",
            "zed.dx.serializer_rlm.external_execution_receipt.v1",
        ],
        steps: vec![
            step(
                1,
                "preview_dx_serializer_rlm_reducer_execution",
                "Confirm the dry-run execution preview is ready and reports no prior external execution.",
                Some("zed.dx.serializer_rlm.execution_preview_receipt.v1"),
                true,
                "Preview receipts only; no external process, shell, Cargo, network, or model call execution.",
            ),
            step(
                2,
                "execute_dx_serializer_rlm_reducer",
                "Run the approved absolute command vector and feed reduced_context_text to stdin when requested.",
                Some("zed.dx.serializer_rlm.external_execution_receipt.v1"),
                true,
                "Executes only explicit no-shell commands under approved DX serializer/RLM roots and writes managed receipts.",
            ),
        ],
        proof_gates: vec![
            "Execution preview receipt is ready, dry_run_only, and reports no prior external process.",
            "Reduced-context receipt is deterministic and contains bounded reduced_context_text.",
            "Operator supplies an absolute command vector under approved DX serializer/RLM roots.",
            "Execution receipt records stdout/stderr previews, hashes, exit code, and no shell execution.",
        ],
        blockers: recipe_blockers(workspace_root),
        next_action: "Use only when a reviewed reducer binary exists; otherwise keep the demo on the dry-run preview receipt.",
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

fn forge_restore_approval_recipe(workspace_root: Option<&PathBuf>) -> DxLaunchDemoRecipe {
    DxLaunchDemoRecipe {
        id: "forge-restore-approval-capture",
        title: "Forge Restore Approval Capture",
        priority: "optional",
        status: recipe_status(workspace_root),
        intent: "Capture operator restore-to-target approval evidence as a managed receipt after a restore preview has been reviewed, without mutating target files.",
        required_tools: vec![
            "inspect_dx_forge_history",
            "capture_dx_forge_restore_approval",
        ],
        receipt_contracts: vec![
            "zed.dx.forge.history.v1",
            "zed.dx.forge.restore_approval_receipt.v1",
        ],
        steps: vec![
            step(
                1,
                "inspect_dx_forge_history",
                "Review managed Forge backup and restore-preview receipts before approval capture.",
                Some("zed.dx.forge.history.v1"),
                false,
                "Read-only receipt scan; does not touch target paths.",
            ),
            step(
                2,
                "capture_dx_forge_restore_approval",
                "Persist operator approval, rollback evidence, overwrite posture, target path, and blockers as a managed approval receipt.",
                Some("zed.dx.forge.restore_approval_receipt.v1"),
                true,
                "Writes approval receipts only; no restore-to-target mutation, overwrite, delete, shell, or external process execution.",
            ),
        ],
        proof_gates: vec![
            "Restore execution receipt reports a managed preview with verified hashes.",
            "Operator approval and rollback evidence are captured explicitly.",
            "Any overwrite posture is recorded as evidence only and does not perform writes.",
            "Approval receipts live under tools/dx-forge/restore-approvals.",
            "inspect_dx_forge_history reports restore_approval entries after capture.",
        ],
        blockers: recipe_blockers(workspace_root),
        next_action: "Use this before any future restore-to-target executor applies changes; it records approval evidence and keeps it visible in Forge history without mutating targets.",
    }
}

fn forge_restore_target_plan_recipe(workspace_root: Option<&PathBuf>) -> DxLaunchDemoRecipe {
    DxLaunchDemoRecipe {
        id: "forge-restore-target-dry-run-plan",
        title: "Forge Restore Target Dry-Run Plan",
        priority: "optional",
        status: recipe_status(workspace_root),
        intent: "Turn restore-preview and approval receipts into a governed dry-run plan for a future restore-to-target executor without mutating live files.",
        required_tools: vec!["inspect_dx_forge_history", "plan_dx_forge_restore_target"],
        receipt_contracts: vec![
            "zed.dx.forge.history.v1",
            "zed.dx.forge.restore_target_plan_receipt.v1",
        ],
        steps: vec![
            step(
                1,
                "inspect_dx_forge_history",
                "Find the latest restore approval, restore preview, backup, and manifest receipts.",
                Some("zed.dx.forge.history.v1"),
                false,
                "Read-only receipt scan; does not inspect or mutate live target contents.",
            ),
            step(
                2,
                "plan_dx_forge_restore_target",
                "Write a dry-run restore-to-target plan with target existence, overwrite posture, rollback, preview, and approval gates.",
                Some("zed.dx.forge.restore_target_plan_receipt.v1"),
                true,
                "Writes plan receipts only; no target mutation, overwrite, delete, shell, external process, Forge, or zstd execution.",
            ),
        ],
        proof_gates: vec![
            "Restore approval receipt is approval-ready and rollback-verified.",
            "Managed restore preview still exists and has verified hash evidence.",
            "Target path and overwrite posture are explicit.",
            "Plan receipt stays non-mutating and visible through inspect_dx_forge_history.",
        ],
        blockers: recipe_blockers(workspace_root),
        next_action: "Use this after restore approval capture to prove the future restore-to-target path is governed before any mutation tool exists.",
    }
}

fn runtime_proof_import_recipe(workspace_root: Option<&PathBuf>) -> DxLaunchDemoRecipe {
    DxLaunchDemoRecipe {
        id: "runtime-proof-import-to-status",
        title: "Runtime Proof Plan And Import To Status Rail",
        priority: "optional",
        status: recipe_status(workspace_root),
        intent: "Prepare the governed manual validation checklist, then capture operator-supplied proof from that window into managed runtime proof import and status receipts for the Check and Proof Freshness rails.",
        required_tools: vec!["plan_dx_runtime_proof", "import_dx_runtime_proof"],
        receipt_contracts: vec![
            "zed.dx.runtime_proof.plan_receipt.v1",
            "zed.dx.runtime_proof.import.v1",
            "zed.dx.runtime_proof.status_copy.v1",
        ],
        steps: vec![
            step(
                1,
                "plan_dx_runtime_proof",
                "Prepare the governed manual runtime validation checklist and managed receipt target contract.",
                Some("zed.dx.runtime_proof.plan_receipt.v1"),
                true,
                "Writes only managed runtime-proof plan receipts; does not run just run, Cargo, browser automation, deploys, reducers, or restore-to-target actions.",
            ),
            step(
                2,
                "import_dx_runtime_proof",
                "Persist operator-provided runtime proof summary, evidence, and blockers as managed import/status receipts after evidence exists.",
                Some("zed.dx.runtime_proof.import.v1"),
                true,
                "Writes only managed runtime proof receipts; does not run just run, Cargo, browser automation, deploys, reducers, or restore-to-target actions.",
            ),
        ],
        proof_gates: vec![
            "Plan receipt records required manual evidence and keeps runtime_green_claim_ready=false before import.",
            "Operator evidence comes from an explicitly governed validation window.",
            "Passed imports include at least one evidence line.",
            "Status copy remains not claim-ready when blockers or missing evidence exist.",
            "Runtime proof receipts live under tools/dx-runtime-proof.",
        ],
        blockers: recipe_blockers(workspace_root),
        next_action: "Run the plan first; import only after manual runtime evidence exists.",
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
            "serializer/RLM runner gates",
            root.join("tools")
                .join("dx-serializer-rlm")
                .join("runner-gates"),
        ),
        (
            "serializer/RLM reduced context",
            root.join("tools")
                .join("dx-serializer-rlm")
                .join("reduced-context"),
        ),
        (
            "serializer/RLM execution previews",
            root.join("tools")
                .join("dx-serializer-rlm")
                .join("execution-previews"),
        ),
        (
            "serializer/RLM external executions",
            root.join("tools")
                .join("dx-serializer-rlm")
                .join("external-executions"),
        ),
        (
            "media executions",
            root.join("tools").join("dx-media").join("executions"),
        ),
        (
            "Forge restores",
            root.join("tools").join("dx-forge").join("restores"),
        ),
        (
            "Forge restore approvals",
            root.join("tools")
                .join("dx-forge")
                .join("restore-approvals"),
        ),
        (
            "runtime proof plans",
            root.join("tools").join("dx-runtime-proof").join("plans"),
        ),
        (
            "runtime proof imports",
            root.join("tools").join("dx-runtime-proof").join("imports"),
        ),
        (
            "runtime proof status",
            root.join("tools").join("dx-runtime-proof").join("status"),
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
