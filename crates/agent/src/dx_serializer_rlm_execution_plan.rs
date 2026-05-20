use serde::Serialize;
use serde_json::Value;
use std::{
    env,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const DX_SERIALIZER_ROOT_ENV: &str = "DX_SERIALIZER_ROOT";
const DX_RLM_ROOT_ENV: &str = "DX_RLM_ROOT";
const DEFAULT_SERIALIZER_ROOT: &str = r"G:\Workspaces\flow\serializer";
const DEFAULT_RLM_ROOT: &str = r"G:\Workspaces\flow\rlm";
const CONTEXT_BUNDLE_SCHEMA: &str = "zed.dx.serializer_rlm.context_bundle.v1";

pub(crate) const DX_SERIALIZER_RLM_EXECUTION_PLAN_SCHEMA: &str =
    "zed.dx.serializer_rlm.execution_plan.v1";
pub(crate) const DX_SERIALIZER_RLM_EXECUTION_RECEIPT_SCHEMA: &str =
    "zed.dx.serializer_rlm.execution_receipt.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxSerializerRlmExecutionPlanRequest {
    pub context_bundle: Value,
    pub task: Option<String>,
    pub reducer: Option<String>,
    pub approve_external_execution: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionPlan {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxSerializerRlmExecutionRequestSummary,
    pub context: DxSerializerRlmExecutionContextSummary,
    pub roots: DxSerializerRlmExecutionRoots,
    pub approval: DxSerializerRlmExecutionApproval,
    pub steps: Vec<DxSerializerRlmExecutionStep>,
    pub execution_receipt: Option<DxSerializerRlmExecutionReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionRequestSummary {
    pub task: Option<String>,
    pub reducer: String,
    pub approve_external_execution: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionContextSummary {
    pub schema: String,
    pub included_source_count: usize,
    pub estimated_tokens: usize,
    pub estimated_chars: usize,
    pub budget_exceeded: bool,
    pub context_text_chars: usize,
    pub source_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionRoots {
    pub serializer: DxSerializerRlmExecutionRoot,
    pub rlm: DxSerializerRlmExecutionRoot,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionRoot {
    pub name: &'static str,
    pub env_var: &'static str,
    pub root: String,
    pub root_exists: bool,
    pub cargo_toml_exists: bool,
    pub integration_ready: bool,
    pub key_files: Vec<DxSerializerRlmExecutionRootFile>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionRootFile {
    pub label: &'static str,
    pub path: String,
    pub exists: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionApproval {
    pub status: String,
    pub external_execution_requested: bool,
    pub external_execution_approved: bool,
    pub external_runner_ready: bool,
    pub tool_executed_external_process: bool,
    pub tool_ran_model_calls: bool,
    pub dry_run_only: bool,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionStep {
    pub step_id: &'static str,
    pub target: &'static str,
    pub status: String,
    pub external_process: bool,
    pub model_calls: bool,
    pub input_contract: &'static str,
    pub output_contract: &'static str,
    pub host_integration_preview: String,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub execution_plan_schema: &'static str,
    pub approval_status: String,
    pub step_count: usize,
    pub next_action: String,
}

pub(crate) fn build_serializer_rlm_execution_plan(
    request: DxSerializerRlmExecutionPlanRequest,
) -> Result<DxSerializerRlmExecutionPlan, String> {
    let reducer = normalize_reducer(request.reducer)?;
    let context_value = decode_json_string(&request.context_bundle)?;
    let context_bundle = locate_context_bundle(&context_value).ok_or_else(|| {
        "DX serializer/RLM execution plan needs a zed.dx.serializer_rlm.context_bundle.v1 object or receipt."
            .to_string()
    })?;
    let context = summarize_context_bundle(context_bundle);
    let roots = DxSerializerRlmExecutionRoots {
        serializer: execution_root(
            "serializer",
            DX_SERIALIZER_ROOT_ENV,
            DEFAULT_SERIALIZER_ROOT,
            &[
                ("cargo_manifest", "Cargo.toml"),
                ("llm_packed_contract", "src/llm/packed.rs"),
                ("llm_schema_contract", "src/llm/schema.rs"),
            ],
        ),
        rlm: execution_root(
            "rlm",
            DX_RLM_ROOT_ENV,
            DEFAULT_RLM_ROOT,
            &[
                ("cargo_manifest", "Cargo.toml"),
                ("document_contract", "src/rlm.rs"),
                ("library_entrypoint", "src/lib.rs"),
            ],
        ),
    };
    let mut blockers = root_blockers(&reducer, &roots);
    if !request.approve_external_execution {
        blockers.push(
            "External serializer/RLM execution has not been approved for this plan.".to_string(),
        );
    }

    let external_runner_ready = blockers.is_empty() && request.approve_external_execution;
    let approval_status = if external_runner_ready {
        "approved_plan_ready"
    } else if request.approve_external_execution {
        "blocked_after_approval"
    } else {
        "approval_required"
    };
    let steps = build_steps(&reducer, &roots, request.approve_external_execution);
    let next_action = if external_runner_ready {
        "Review the execution receipt, then wire the future runner to execute these in-process serializer/RLM calls under the same approval gate."
            .to_string()
    } else if request.approve_external_execution {
        "Resolve the listed root or contract blockers before enabling the external serializer/RLM runner."
            .to_string()
    } else {
        "Review this dry-run execution plan, then rerun with approve_external_execution=true when ready to authorize the future runner."
            .to_string()
    };

    Ok(DxSerializerRlmExecutionPlan {
        schema: DX_SERIALIZER_RLM_EXECUTION_PLAN_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxSerializerRlmExecutionRequestSummary {
            task: clean_optional_text(request.task, 240),
            reducer,
            approve_external_execution: request.approve_external_execution,
        },
        context,
        roots,
        approval: DxSerializerRlmExecutionApproval {
            status: approval_status.to_string(),
            external_execution_requested: request.approve_external_execution,
            external_execution_approved: request.approve_external_execution,
            external_runner_ready,
            tool_executed_external_process: false,
            tool_ran_model_calls: false,
            dry_run_only: true,
            blockers,
        },
        steps,
        execution_receipt: None,
        next_action,
    })
}

fn normalize_reducer(reducer: Option<String>) -> Result<String, String> {
    let reducer = reducer
        .unwrap_or_else(|| "hybrid".to_string())
        .trim()
        .to_ascii_lowercase()
        .replace('-', "_");

    match reducer.as_str() {
        "" | "hybrid" | "serializer_then_rlm" => Ok("hybrid".to_string()),
        "serializer" | "serializer_only" => Ok("serializer_only".to_string()),
        "rlm" | "rlm_only" => Ok("rlm_only".to_string()),
        _ => Err(format!(
            "Unsupported DX serializer/RLM reducer `{reducer}`. Use hybrid, serializer_only, or rlm_only."
        )),
    }
}

fn summarize_context_bundle(bundle: &Value) -> DxSerializerRlmExecutionContextSummary {
    let summary = bundle.get("summary").unwrap_or(&Value::Null);
    let items = bundle
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let source_ids = items
        .iter()
        .filter_map(|item| string_field(item, "source_id"))
        .collect::<Vec<_>>();
    let context_text_chars = string_field(bundle, "context_text")
        .map(|text| text.chars().count())
        .unwrap_or_default();

    DxSerializerRlmExecutionContextSummary {
        schema: string_field(bundle, "schema").unwrap_or_else(|| CONTEXT_BUNDLE_SCHEMA.to_string()),
        included_source_count: usize_field(summary, "included_source_count").unwrap_or(items.len()),
        estimated_tokens: usize_field(summary, "estimated_tokens").unwrap_or_default(),
        estimated_chars: usize_field(summary, "estimated_chars").unwrap_or_default(),
        budget_exceeded: bool_field(summary, "budget_exceeded").unwrap_or(false),
        context_text_chars,
        source_ids,
    }
}

fn build_steps(
    reducer: &str,
    roots: &DxSerializerRlmExecutionRoots,
    approved: bool,
) -> Vec<DxSerializerRlmExecutionStep> {
    let mut steps = Vec::new();

    if reducer == "hybrid" || reducer == "serializer_only" {
        steps.push(execution_step(
            "serializer_pack_context",
            "serializer",
            roots.serializer.integration_ready,
            approved,
            "zed.dx.serializer_rlm.context_bundle.v1",
            "dx_serializer llm packed context payload",
            "Call the serializer crate in-process to encode context_text and citation metadata through src/llm/packed.rs.",
        ));
    }

    if reducer == "hybrid" || reducer == "rlm_only" {
        steps.push(execution_step(
            "rlm_reduce_context",
            "rlm",
            roots.rlm.integration_ready,
            approved,
            "RLMDocument { id, title, content, source_path, mime_type, tags }",
            "RLMRecursiveResponse / reduced Agent context",
            "Create an RLMDocument from the compact context bundle and call RLM::complete_document through the approved local provider profile.",
        ));
    }

    steps
}

fn execution_step(
    step_id: &'static str,
    target: &'static str,
    root_ready: bool,
    approved: bool,
    input_contract: &'static str,
    output_contract: &'static str,
    host_integration_preview: &'static str,
) -> DxSerializerRlmExecutionStep {
    let mut blockers = Vec::new();
    if !root_ready {
        blockers.push(format!(
            "{target} root or key integration files are not ready."
        ));
    }
    if !approved {
        blockers
            .push("External execution approval is required before this step can run.".to_string());
    }
    let status = if blockers.is_empty() {
        "ready_for_future_runner"
    } else {
        "blocked"
    };

    DxSerializerRlmExecutionStep {
        step_id,
        target,
        status: status.to_string(),
        external_process: false,
        model_calls: target == "rlm",
        input_contract,
        output_contract,
        host_integration_preview: host_integration_preview.to_string(),
        blockers,
    }
}

fn root_blockers(reducer: &str, roots: &DxSerializerRlmExecutionRoots) -> Vec<String> {
    let mut blockers = Vec::new();
    if (reducer == "hybrid" || reducer == "serializer_only") && !roots.serializer.integration_ready
    {
        blockers.push(format!(
            "Serializer root is not integration-ready at {}.",
            roots.serializer.root
        ));
    }
    if (reducer == "hybrid" || reducer == "rlm_only") && !roots.rlm.integration_ready {
        blockers.push(format!(
            "RLM root is not integration-ready at {}.",
            roots.rlm.root
        ));
    }
    blockers
}

fn execution_root(
    name: &'static str,
    env_var: &'static str,
    default_root: &str,
    key_files: &[(&'static str, &'static str)],
) -> DxSerializerRlmExecutionRoot {
    let root = env::var_os(env_var)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_root));
    let root_exists = root.exists();
    let files = key_files
        .iter()
        .map(|(label, relative_path)| {
            let path = root.join(relative_path);
            DxSerializerRlmExecutionRootFile {
                label,
                path: path.display().to_string(),
                exists: path.is_file(),
            }
        })
        .collect::<Vec<_>>();
    let cargo_toml_exists = files
        .iter()
        .any(|file| file.label == "cargo_manifest" && file.exists);
    let integration_ready = root_exists && files.iter().all(|file| file.exists);

    DxSerializerRlmExecutionRoot {
        name,
        env_var,
        root: root.display().to_string(),
        root_exists,
        cargo_toml_exists,
        integration_ready,
        key_files: files,
    }
}

fn locate_context_bundle(value: &Value) -> Option<&Value> {
    if value.get("schema").and_then(Value::as_str) == Some(CONTEXT_BUNDLE_SCHEMA) {
        return Some(value);
    }

    value.get("context_bundle").filter(|bundle| {
        bundle.get("schema").and_then(Value::as_str) == Some(CONTEXT_BUNDLE_SCHEMA)
    })
}

fn decode_json_string(value: &Value) -> Result<Value, String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if text.starts_with('{') {
            return serde_json::from_str(text).map_err(|error| {
                format!("Failed to parse stringified DX serializer/RLM context JSON: {error}")
            });
        }
    }

    Ok(value.clone())
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn usize_field(value: &Value, field: &str) -> Option<usize> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn bool_field(value: &Value, field: &str) -> Option<bool> {
    value.get(field).and_then(Value::as_bool)
}

fn clean_optional_text(value: Option<String>, max_chars: usize) -> Option<String> {
    value
        .map(|value| truncate_for_char_budget(&compact_text(&value), max_chars))
        .filter(|value| !value.is_empty())
}

fn compact_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_for_char_budget(text: &str, max_chars: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let suffix = "...";
    if max_chars <= suffix.len() {
        return text.chars().take(max_chars).collect();
    }

    let mut truncated = text
        .chars()
        .take(max_chars - suffix.len())
        .collect::<String>();
    truncated.push_str(suffix);
    truncated
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
