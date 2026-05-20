use crate::{
    dx_metasearch_context_adapter::{
        DX_METASEARCH_CONTEXT_BUNDLE_SCHEMA, DX_METASEARCH_CONTEXT_RECEIPT_SCHEMA,
    },
    dx_serializer_rlm_runner_gate::{
        DX_SERIALIZER_RLM_RUNNER_GATE_RECEIPT_SCHEMA, DX_SERIALIZER_RLM_RUNNER_GATE_SCHEMA,
    },
};
use serde::Serialize;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_OUTPUT_TOKEN_BUDGET: usize = 900;
const MIN_OUTPUT_TOKEN_BUDGET: usize = 100;
const MAX_OUTPUT_TOKEN_BUDGET: usize = 4_000;
const APPROX_CHARS_PER_TOKEN: usize = 4;
const TRUNCATION_NOTICE: &str = "\n\n[truncated for dx serializer/rlm reduced-context budget]";

pub(crate) const DX_SERIALIZER_RLM_REDUCED_CONTEXT_SCHEMA: &str =
    "zed.dx.serializer_rlm.reduced_context.v1";
pub(crate) const DX_SERIALIZER_RLM_REDUCED_CONTEXT_RECEIPT_SCHEMA: &str =
    "zed.dx.serializer_rlm.reduced_context_receipt.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxSerializerRlmReducedContextRequest {
    pub runner_gate: Value,
    pub context_bundle: Value,
    pub max_output_tokens: Option<usize>,
    pub require_runner_ready: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmReducedContext {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxSerializerRlmReducedContextRequestSummary,
    pub gate: DxSerializerRlmReducedContextGateSummary,
    pub context: DxSerializerRlmReducedContextBundleSummary,
    pub reduction: DxSerializerRlmReducedContextSummary,
    pub reduced_context_text: String,
    pub sources: Vec<DxSerializerRlmReducedContextSource>,
    pub reduced_context_receipt: Option<DxSerializerRlmReducedContextReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmReducedContextRequestSummary {
    pub max_output_tokens: usize,
    pub char_budget: usize,
    pub require_runner_ready: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmReducedContextGateSummary {
    pub schema: String,
    pub received_from_runner_receipt: bool,
    pub runner_receipt_schema: Option<String>,
    pub status: String,
    pub runner_ready: bool,
    pub runner_approved: bool,
    pub model_calls_approved: bool,
    pub reducer: String,
    pub step_count: usize,
    pub would_run_external_serializer: bool,
    pub would_run_external_rlm: bool,
    pub would_run_model_calls: bool,
    pub tool_ran_external_process: bool,
    pub tool_ran_model_calls: bool,
    pub tool_wrote_reduced_context: bool,
    pub blocker_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmReducedContextBundleSummary {
    pub schema: String,
    pub received_from_context_receipt: bool,
    pub context_receipt_schema: Option<String>,
    pub input_source_count: usize,
    pub included_source_count: usize,
    pub omitted_source_count: usize,
    pub estimated_chars: usize,
    pub estimated_tokens: usize,
    pub item_count: usize,
    pub context_text_chars: usize,
    pub budget_exceeded: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmReducedContextSummary {
    pub status: String,
    pub reduced_context_ready: bool,
    pub deterministic_only: bool,
    pub approx_chars_per_token: usize,
    pub selected_chars: usize,
    pub selected_estimated_tokens: usize,
    pub source_count: usize,
    pub truncated: bool,
    pub runs_external_serializer: bool,
    pub runs_external_rlm: bool,
    pub runs_model_calls: bool,
    pub writes_reduced_context_file: bool,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmReducedContextSource {
    pub source_id: String,
    pub title: String,
    pub url: String,
    pub source_kind: String,
    pub estimated_tokens: usize,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmReducedContextReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub reduced_context_schema: &'static str,
    pub runner_gate_status: String,
    pub source_count: usize,
    pub selected_estimated_tokens: usize,
    pub next_action: String,
}

pub(crate) fn build_serializer_rlm_reduced_context(
    request: DxSerializerRlmReducedContextRequest,
) -> Result<DxSerializerRlmReducedContext, String> {
    let output_tokens = request
        .max_output_tokens
        .unwrap_or(DEFAULT_OUTPUT_TOKEN_BUDGET)
        .clamp(MIN_OUTPUT_TOKEN_BUDGET, MAX_OUTPUT_TOKEN_BUDGET);
    let char_budget = output_tokens.saturating_mul(APPROX_CHARS_PER_TOKEN);

    let gate_value = decode_json_string(&request.runner_gate, "runner gate")?;
    let (runner_gate, runner_receipt_schema) = locate_runner_gate(&gate_value).ok_or_else(|| {
        "DX serializer/RLM reduced context needs a zed.dx.serializer_rlm.runner_gate.v1 object or runner-gate receipt."
            .to_string()
    })?;
    let gate = summarize_runner_gate(runner_gate, runner_receipt_schema);

    let context_value = decode_json_string(&request.context_bundle, "context bundle")?;
    let (context_bundle, context_receipt_schema) =
        locate_context_bundle(&context_value).ok_or_else(|| {
            "DX serializer/RLM reduced context needs a zed.dx.serializer_rlm.context_bundle.v1 object or context receipt."
                .to_string()
        })?;
    let context = summarize_context_bundle(context_bundle, context_receipt_schema);
    let sources = summarize_sources(context_bundle);

    let blockers = reduced_context_blockers(&request, &gate, &context);
    let ready = blockers.is_empty();
    let context_text = string_field(context_bundle, &["context_text"]).unwrap_or_default();
    let (reduced_context_text, selected_chars, truncated) = if ready {
        truncate_context_text(&context_text, char_budget)
    } else {
        (String::new(), 0, false)
    };
    let selected_estimated_tokens = estimate_tokens(selected_chars);
    let status = if ready {
        "reduced_context_ready"
    } else if gate.runner_ready {
        "context_blocked"
    } else {
        "runner_gate_blocked"
    };
    let next_action = if ready {
        "Use this reduced-context receipt as the deterministic input contract before wiring any external serializer/RLM reducer or model-call runner."
            .to_string()
    } else {
        "Resolve the listed reduced-context blockers before writing or consuming a reduced-context receipt."
            .to_string()
    };

    Ok(DxSerializerRlmReducedContext {
        schema: DX_SERIALIZER_RLM_REDUCED_CONTEXT_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxSerializerRlmReducedContextRequestSummary {
            max_output_tokens: output_tokens,
            char_budget,
            require_runner_ready: request.require_runner_ready,
            root_mode: request.root_mode,
        },
        gate,
        context,
        reduction: DxSerializerRlmReducedContextSummary {
            status: status.to_string(),
            reduced_context_ready: ready,
            deterministic_only: true,
            approx_chars_per_token: APPROX_CHARS_PER_TOKEN,
            selected_chars,
            selected_estimated_tokens,
            source_count: sources.len(),
            truncated,
            runs_external_serializer: false,
            runs_external_rlm: false,
            runs_model_calls: false,
            writes_reduced_context_file: false,
            blockers,
        },
        reduced_context_text,
        sources,
        reduced_context_receipt: None,
        next_action,
    })
}

fn reduced_context_blockers(
    request: &DxSerializerRlmReducedContextRequest,
    gate: &DxSerializerRlmReducedContextGateSummary,
    context: &DxSerializerRlmReducedContextBundleSummary,
) -> Vec<String> {
    let mut blockers = Vec::new();

    if gate.schema != DX_SERIALIZER_RLM_RUNNER_GATE_SCHEMA {
        blockers.push(format!(
            "Expected runner gate schema {DX_SERIALIZER_RLM_RUNNER_GATE_SCHEMA}, got {}.",
            gate.schema
        ));
    }
    if request.require_runner_ready && !gate.runner_ready {
        blockers.push(format!(
            "Runner gate status is `{}` instead of `runner_ready`.",
            gate.status
        ));
    }
    if gate.tool_ran_external_process {
        blockers.push("Runner gate tool unexpectedly ran an external process.".to_string());
    }
    if gate.tool_ran_model_calls {
        blockers.push("Runner gate tool unexpectedly ran model calls.".to_string());
    }
    if gate.tool_wrote_reduced_context {
        blockers.push("Runner gate tool unexpectedly wrote reduced context.".to_string());
    }
    if context.schema != DX_METASEARCH_CONTEXT_BUNDLE_SCHEMA {
        blockers.push(format!(
            "Expected context bundle schema {DX_METASEARCH_CONTEXT_BUNDLE_SCHEMA}, got {}.",
            context.schema
        ));
    }
    if context.context_text_chars == 0 {
        blockers.push("Context bundle contains no context_text to reduce.".to_string());
    }
    if context.item_count == 0 {
        blockers.push("Context bundle contains no cited source items.".to_string());
    }

    blockers
}

fn summarize_runner_gate(
    gate: &Value,
    receipt_schema: Option<String>,
) -> DxSerializerRlmReducedContextGateSummary {
    let validation = gate.get("validation").unwrap_or(&Value::Null);
    let plan = gate.get("plan").unwrap_or(&Value::Null);

    DxSerializerRlmReducedContextGateSummary {
        schema: string_field(gate, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        received_from_runner_receipt: receipt_schema.is_some(),
        runner_receipt_schema: receipt_schema,
        status: string_field(validation, &["status"]).unwrap_or_else(|| "unknown".to_string()),
        runner_ready: bool_field(validation, &["runner_ready"]).unwrap_or(false),
        runner_approved: bool_field(validation, &["runner_approved"]).unwrap_or(false),
        model_calls_approved: bool_field(validation, &["model_calls_approved"]).unwrap_or(false),
        reducer: string_field(plan, &["reducer"]).unwrap_or_else(|| "unknown".to_string()),
        step_count: usize_field(plan, &["step_count"]).unwrap_or_default(),
        would_run_external_serializer: bool_field(validation, &["would_run_external_serializer"])
            .unwrap_or(false),
        would_run_external_rlm: bool_field(validation, &["would_run_external_rlm"])
            .unwrap_or(false),
        would_run_model_calls: bool_field(validation, &["would_run_model_calls"]).unwrap_or(false),
        tool_ran_external_process: bool_field(validation, &["tool_ran_external_process"])
            .unwrap_or(false),
        tool_ran_model_calls: bool_field(validation, &["tool_ran_model_calls"]).unwrap_or(false),
        tool_wrote_reduced_context: bool_field(validation, &["tool_wrote_reduced_context"])
            .unwrap_or(false),
        blocker_count: array_len(validation, &["blockers"]),
    }
}

fn summarize_context_bundle(
    bundle: &Value,
    receipt_schema: Option<String>,
) -> DxSerializerRlmReducedContextBundleSummary {
    let summary = bundle.get("summary").unwrap_or(&Value::Null);
    let context_text = string_field(bundle, &["context_text"]).unwrap_or_default();

    DxSerializerRlmReducedContextBundleSummary {
        schema: string_field(bundle, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        received_from_context_receipt: receipt_schema.is_some(),
        context_receipt_schema: receipt_schema,
        input_source_count: usize_field(summary, &["input_source_count"]).unwrap_or_default(),
        included_source_count: usize_field(summary, &["included_source_count"]).unwrap_or_default(),
        omitted_source_count: usize_field(summary, &["omitted_source_count"]).unwrap_or_default(),
        estimated_chars: usize_field(summary, &["estimated_chars"]).unwrap_or_default(),
        estimated_tokens: usize_field(summary, &["estimated_tokens"]).unwrap_or_default(),
        item_count: array_len(bundle, &["items"]),
        context_text_chars: context_text.chars().count(),
        budget_exceeded: bool_field(summary, &["budget_exceeded"]).unwrap_or(false),
    }
}

fn summarize_sources(bundle: &Value) -> Vec<DxSerializerRlmReducedContextSource> {
    bundle
        .get("items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| DxSerializerRlmReducedContextSource {
                    source_id: string_field(item, &["source_id"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    title: string_field(item, &["title"]).unwrap_or_else(|| "Untitled".to_string()),
                    url: string_field(item, &["url"]).unwrap_or_default(),
                    source_kind: string_field(item, &["source_kind"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    estimated_tokens: usize_field(item, &["estimated_tokens"]).unwrap_or_default(),
                    truncated: bool_field(item, &["truncated"]).unwrap_or(false),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn locate_runner_gate(value: &Value) -> Option<(&Value, Option<String>)> {
    if value.get("schema").and_then(Value::as_str) == Some(DX_SERIALIZER_RLM_RUNNER_GATE_SCHEMA) {
        return Some((value, None));
    }

    let receipt_schema = value
        .get("schema")
        .and_then(Value::as_str)
        .filter(|schema| *schema == DX_SERIALIZER_RLM_RUNNER_GATE_RECEIPT_SCHEMA)
        .map(ToOwned::to_owned);
    let gate = value.get("runner_gate").filter(|gate| {
        gate.get("schema").and_then(Value::as_str) == Some(DX_SERIALIZER_RLM_RUNNER_GATE_SCHEMA)
    })?;

    Some((gate, receipt_schema))
}

fn locate_context_bundle(value: &Value) -> Option<(&Value, Option<String>)> {
    if value.get("schema").and_then(Value::as_str) == Some(DX_METASEARCH_CONTEXT_BUNDLE_SCHEMA) {
        return Some((value, None));
    }

    let receipt_schema = value
        .get("schema")
        .and_then(Value::as_str)
        .filter(|schema| *schema == DX_METASEARCH_CONTEXT_RECEIPT_SCHEMA)
        .map(ToOwned::to_owned);
    let bundle = value.get("context_bundle").filter(|bundle| {
        bundle.get("schema").and_then(Value::as_str) == Some(DX_METASEARCH_CONTEXT_BUNDLE_SCHEMA)
    })?;

    Some((bundle, receipt_schema))
}

fn decode_json_string(value: &Value, label: &str) -> Result<Value, String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if text.starts_with('{') {
            return serde_json::from_str(text).map_err(|error| {
                format!("Failed to parse stringified DX serializer/RLM {label} JSON: {error}")
            });
        }
    }

    Ok(value.clone())
}

fn truncate_context_text(text: &str, char_budget: usize) -> (String, usize, bool) {
    let trimmed = text.trim();
    let text_chars = trimmed.chars().count();
    if text_chars <= char_budget {
        return (trimmed.to_string(), text_chars, false);
    }

    let notice_chars = TRUNCATION_NOTICE.chars().count();
    let take_chars = char_budget
        .saturating_sub(notice_chars)
        .max(MIN_OUTPUT_TOKEN_BUDGET);
    let mut selected = trimmed.chars().take(take_chars).collect::<String>();
    selected.truncate(selected.trim_end().len());
    selected.push_str(TRUNCATION_NOTICE);
    let selected_chars = selected.chars().count();

    (selected, selected_chars, true)
}

fn estimate_tokens(chars: usize) -> usize {
    chars.saturating_add(APPROX_CHARS_PER_TOKEN - 1) / APPROX_CHARS_PER_TOKEN
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

fn bool_field(value: &Value, path: &[&str]) -> Option<bool> {
    value_at(value, path).and_then(Value::as_bool)
}

fn usize_field(value: &Value, path: &[&str]) -> Option<usize> {
    value_at(value, path)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn array_len(value: &Value, path: &[&str]) -> usize {
    value_at(value, path)
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
