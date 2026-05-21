use crate::{
    dx_serializer_rlm_reduced_context::{
        DX_SERIALIZER_RLM_REDUCED_CONTEXT_RECEIPT_SCHEMA, DX_SERIALIZER_RLM_REDUCED_CONTEXT_SCHEMA,
    },
    dx_serializer_rlm_runner_gate::{
        DX_SERIALIZER_RLM_RUNNER_GATE_RECEIPT_SCHEMA, DX_SERIALIZER_RLM_RUNNER_GATE_SCHEMA,
    },
};
use serde::Serialize;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const DX_SERIALIZER_RLM_EXECUTION_PREVIEW_SCHEMA: &str =
    "zed.dx.serializer_rlm.execution_preview.v1";
pub(crate) const DX_SERIALIZER_RLM_EXECUTION_PREVIEW_RECEIPT_SCHEMA: &str =
    "zed.dx.serializer_rlm.execution_preview_receipt.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxSerializerRlmExecutionPreviewRequest {
    pub runner_gate: Value,
    pub reduced_context: Value,
    pub approve_execution_preview: bool,
    pub allow_model_calls: bool,
    pub require_runner_ready: bool,
    pub require_reduced_context_ready: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionPreview {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxSerializerRlmExecutionPreviewRequestSummary,
    pub gate: DxSerializerRlmExecutionPreviewGateSummary,
    pub reduced_context: DxSerializerRlmExecutionPreviewReducedContextSummary,
    pub planned_steps: Vec<DxSerializerRlmExecutionPreviewStep>,
    pub preview: DxSerializerRlmExecutionPreviewSummary,
    pub execution_preview_receipt: Option<DxSerializerRlmExecutionPreviewReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionPreviewRequestSummary {
    pub approve_execution_preview: bool,
    pub allow_model_calls: bool,
    pub require_runner_ready: bool,
    pub require_reduced_context_ready: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionPreviewGateSummary {
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
pub(crate) struct DxSerializerRlmExecutionPreviewReducedContextSummary {
    pub schema: String,
    pub received_from_reduced_context_receipt: bool,
    pub reduced_context_receipt_schema: Option<String>,
    pub status: String,
    pub reduced_context_ready: bool,
    pub deterministic_only: bool,
    pub source_count: usize,
    pub selected_estimated_tokens: usize,
    pub reduced_context_chars: usize,
    pub runs_external_serializer: bool,
    pub runs_external_rlm: bool,
    pub runs_model_calls: bool,
    pub blocker_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionPreviewStep {
    pub step_id: String,
    pub target: String,
    pub status: String,
    pub external_process: bool,
    pub model_calls: bool,
    pub preview_status: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionPreviewSummary {
    pub status: &'static str,
    pub execution_preview_ready: bool,
    pub dry_run_only: bool,
    pub permission_required_for_real_execution: bool,
    pub would_run_external_serializer: bool,
    pub would_run_external_rlm: bool,
    pub would_run_model_calls: bool,
    pub tool_ran_external_process: bool,
    pub tool_ran_model_calls: bool,
    pub tool_wrote_execution_output: bool,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExecutionPreviewReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub execution_preview_ready: bool,
    pub reducer: String,
    pub step_count: usize,
    pub would_run_model_calls: bool,
    pub next_action: String,
}

pub(crate) fn build_serializer_rlm_execution_preview(
    request: DxSerializerRlmExecutionPreviewRequest,
) -> Result<DxSerializerRlmExecutionPreview, String> {
    let gate_value = decode_json_string(&request.runner_gate, "runner gate")?;
    let (runner_gate, runner_receipt_schema) = locate_runner_gate(&gate_value).ok_or_else(|| {
        "DX serializer/RLM execution preview needs a zed.dx.serializer_rlm.runner_gate.v1 object or runner-gate receipt."
            .to_string()
    })?;
    let gate = summarize_runner_gate(runner_gate, runner_receipt_schema);

    let reduced_context_value = decode_json_string(&request.reduced_context, "reduced context")?;
    let (reduced_context, reduced_context_receipt_schema) =
        locate_reduced_context(&reduced_context_value).ok_or_else(|| {
            "DX serializer/RLM execution preview needs a zed.dx.serializer_rlm.reduced_context.v1 object or reduced-context receipt."
                .to_string()
        })?;
    let reduced_context =
        summarize_reduced_context(reduced_context, reduced_context_receipt_schema);
    let planned_steps = summarize_steps(runner_gate);
    let blockers = execution_preview_blockers(&request, &gate, &reduced_context, &planned_steps);
    let execution_preview_ready = blockers.is_empty();
    let status = if execution_preview_ready {
        "execution_preview_ready"
    } else if request.approve_execution_preview {
        "blocked_after_preview_approval"
    } else {
        "preview_approval_required"
    };
    let next_action = if execution_preview_ready {
        "Review this dry-run preview receipt before adding any separate external serializer/RLM executor."
            .to_string()
    } else {
        "Resolve the listed reducer preview blockers before wiring or approving external execution."
            .to_string()
    };

    Ok(DxSerializerRlmExecutionPreview {
        schema: DX_SERIALIZER_RLM_EXECUTION_PREVIEW_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxSerializerRlmExecutionPreviewRequestSummary {
            approve_execution_preview: request.approve_execution_preview,
            allow_model_calls: request.allow_model_calls,
            require_runner_ready: request.require_runner_ready,
            require_reduced_context_ready: request.require_reduced_context_ready,
            root_mode: request.root_mode,
        },
        gate,
        reduced_context,
        planned_steps,
        preview: DxSerializerRlmExecutionPreviewSummary {
            status,
            execution_preview_ready,
            dry_run_only: true,
            permission_required_for_real_execution: true,
            would_run_external_serializer: execution_preview_ready
                && runner_would_run_serializer(runner_gate),
            would_run_external_rlm: execution_preview_ready && runner_would_run_rlm(runner_gate),
            would_run_model_calls: execution_preview_ready
                && runner_would_run_model_calls(runner_gate),
            tool_ran_external_process: false,
            tool_ran_model_calls: false,
            tool_wrote_execution_output: false,
            blockers,
        },
        execution_preview_receipt: None,
        next_action,
    })
}

fn execution_preview_blockers(
    request: &DxSerializerRlmExecutionPreviewRequest,
    gate: &DxSerializerRlmExecutionPreviewGateSummary,
    reduced_context: &DxSerializerRlmExecutionPreviewReducedContextSummary,
    planned_steps: &[DxSerializerRlmExecutionPreviewStep],
) -> Vec<String> {
    let mut blockers = Vec::new();

    if !request.approve_execution_preview {
        blockers
            .push("Serializer/RLM execution dry-run preview has not been approved.".to_string());
    }
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
        blockers.push("Runner gate reports an external process already ran.".to_string());
    }
    if gate.tool_ran_model_calls {
        blockers.push("Runner gate reports model calls already ran.".to_string());
    }
    if gate.tool_wrote_reduced_context {
        blockers.push("Runner gate reports reduced context was written by the gate.".to_string());
    }
    if gate.would_run_model_calls && !request.allow_model_calls {
        blockers.push(
            "Dry-run preview includes model-call steps but allow_model_calls=false.".to_string(),
        );
    }
    if gate.would_run_model_calls && !gate.model_calls_approved {
        blockers.push("Runner gate has not approved model-call steps.".to_string());
    }
    if reduced_context.schema != DX_SERIALIZER_RLM_REDUCED_CONTEXT_SCHEMA {
        blockers.push(format!(
            "Expected reduced-context schema {DX_SERIALIZER_RLM_REDUCED_CONTEXT_SCHEMA}, got {}.",
            reduced_context.schema
        ));
    }
    if request.require_reduced_context_ready && !reduced_context.reduced_context_ready {
        blockers.push(format!(
            "Reduced-context status is `{}` instead of `reduced_context_ready`.",
            reduced_context.status
        ));
    }
    if !reduced_context.deterministic_only {
        blockers.push("Reduced-context receipt must be deterministic_only.".to_string());
    }
    if reduced_context.runs_external_serializer
        || reduced_context.runs_external_rlm
        || reduced_context.runs_model_calls
    {
        blockers.push(
            "Reduced-context receipt unexpectedly reports external serializer/RLM work or model calls."
                .to_string(),
        );
    }
    if reduced_context.reduced_context_chars == 0 {
        blockers.push("Reduced-context receipt contains no reduced_context_text.".to_string());
    }
    if planned_steps.is_empty() {
        blockers.push("Runner gate contains no planned reducer steps.".to_string());
    }

    blockers
}

fn summarize_runner_gate(
    gate: &Value,
    receipt_schema: Option<String>,
) -> DxSerializerRlmExecutionPreviewGateSummary {
    let validation = gate.get("validation").unwrap_or(&Value::Null);
    let plan = gate.get("plan").unwrap_or(&Value::Null);

    DxSerializerRlmExecutionPreviewGateSummary {
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

fn summarize_reduced_context(
    reduced_context: &Value,
    receipt_schema: Option<String>,
) -> DxSerializerRlmExecutionPreviewReducedContextSummary {
    let reduction = reduced_context.get("reduction").unwrap_or(&Value::Null);
    let reduced_context_text =
        string_field(reduced_context, &["reduced_context_text"]).unwrap_or_default();

    DxSerializerRlmExecutionPreviewReducedContextSummary {
        schema: string_field(reduced_context, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        received_from_reduced_context_receipt: receipt_schema.is_some(),
        reduced_context_receipt_schema: receipt_schema,
        status: string_field(reduction, &["status"]).unwrap_or_else(|| "unknown".to_string()),
        reduced_context_ready: bool_field(reduction, &["reduced_context_ready"]).unwrap_or(false),
        deterministic_only: bool_field(reduction, &["deterministic_only"]).unwrap_or(false),
        source_count: usize_field(reduction, &["source_count"]).unwrap_or_default(),
        selected_estimated_tokens: usize_field(reduction, &["selected_estimated_tokens"])
            .unwrap_or_default(),
        reduced_context_chars: reduced_context_text.chars().count(),
        runs_external_serializer: bool_field(reduction, &["runs_external_serializer"])
            .unwrap_or(false),
        runs_external_rlm: bool_field(reduction, &["runs_external_rlm"]).unwrap_or(false),
        runs_model_calls: bool_field(reduction, &["runs_model_calls"]).unwrap_or(false),
        blocker_count: array_len(reduction, &["blockers"]),
    }
}

fn summarize_steps(gate: &Value) -> Vec<DxSerializerRlmExecutionPreviewStep> {
    gate.get("plan")
        .and_then(|plan| plan.get("steps"))
        .and_then(Value::as_array)
        .map(|steps| {
            steps
                .iter()
                .map(|step| {
                    let status =
                        string_field(step, &["status"]).unwrap_or_else(|| "unknown".to_string());
                    let blocker_count = array_len(step, &["blockers"]);
                    DxSerializerRlmExecutionPreviewStep {
                        step_id: string_field(step, &["step_id"])
                            .unwrap_or_else(|| "unknown".to_string()),
                        target: string_field(step, &["target"])
                            .unwrap_or_else(|| "unknown".to_string()),
                        status,
                        external_process: bool_field(step, &["external_process"]).unwrap_or(false),
                        model_calls: bool_field(step, &["model_calls"]).unwrap_or(false),
                        preview_status: if blocker_count == 0 {
                            "preview_ready"
                        } else {
                            "blocked"
                        },
                    }
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

fn locate_reduced_context(value: &Value) -> Option<(&Value, Option<String>)> {
    if value.get("schema").and_then(Value::as_str) == Some(DX_SERIALIZER_RLM_REDUCED_CONTEXT_SCHEMA)
    {
        return Some((value, None));
    }

    let receipt_schema = value
        .get("schema")
        .and_then(Value::as_str)
        .filter(|schema| *schema == DX_SERIALIZER_RLM_REDUCED_CONTEXT_RECEIPT_SCHEMA)
        .map(ToOwned::to_owned);
    let reduced_context = value.get("reduced_context").filter(|reduced_context| {
        reduced_context.get("schema").and_then(Value::as_str)
            == Some(DX_SERIALIZER_RLM_REDUCED_CONTEXT_SCHEMA)
    })?;

    Some((reduced_context, receipt_schema))
}

fn runner_would_run_serializer(gate: &Value) -> bool {
    bool_field(gate, &["validation", "would_run_external_serializer"]).unwrap_or(false)
}

fn runner_would_run_rlm(gate: &Value) -> bool {
    bool_field(gate, &["validation", "would_run_external_rlm"]).unwrap_or(false)
}

fn runner_would_run_model_calls(gate: &Value) -> bool {
    bool_field(gate, &["validation", "would_run_model_calls"]).unwrap_or(false)
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
