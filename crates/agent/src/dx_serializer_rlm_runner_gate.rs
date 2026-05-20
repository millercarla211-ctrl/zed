use crate::dx_serializer_rlm_execution_plan::{
    DX_SERIALIZER_RLM_EXECUTION_PLAN_SCHEMA, DX_SERIALIZER_RLM_EXECUTION_RECEIPT_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const DX_SERIALIZER_RLM_RUNNER_GATE_SCHEMA: &str =
    "zed.dx.serializer_rlm.runner_gate.v1";
pub(crate) const DX_SERIALIZER_RLM_RUNNER_GATE_RECEIPT_SCHEMA: &str =
    "zed.dx.serializer_rlm.runner_gate_receipt.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxSerializerRlmRunnerGateRequest {
    pub execution_plan: Value,
    pub approve_runner: bool,
    pub allow_model_calls: bool,
    pub require_execution_receipt: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmRunnerGate {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxSerializerRlmRunnerGateRequestSummary,
    pub plan: DxSerializerRlmRunnerGatePlanSummary,
    pub validation: DxSerializerRlmRunnerGateValidation,
    pub runner_receipt: Option<DxSerializerRlmRunnerGateReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmRunnerGateRequestSummary {
    pub approve_runner: bool,
    pub allow_model_calls: bool,
    pub require_execution_receipt: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmRunnerGatePlanSummary {
    pub schema: String,
    pub received_from_execution_receipt: bool,
    pub execution_receipt_schema: Option<String>,
    pub approval_status: String,
    pub reducer: String,
    pub task: Option<String>,
    pub external_runner_ready: bool,
    pub external_execution_approved: bool,
    pub dry_run_only: bool,
    pub tool_executed_external_process: bool,
    pub tool_ran_model_calls: bool,
    pub included_source_count: usize,
    pub estimated_tokens: usize,
    pub step_count: usize,
    pub ready_step_count: usize,
    pub serializer_step_count: usize,
    pub rlm_step_count: usize,
    pub model_call_step_count: usize,
    pub steps: Vec<DxSerializerRlmRunnerGateStepSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmRunnerGateStepSummary {
    pub step_id: String,
    pub target: String,
    pub status: String,
    pub external_process: bool,
    pub model_calls: bool,
    pub blocker_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmRunnerGateValidation {
    pub status: String,
    pub runner_ready: bool,
    pub permission_required: bool,
    pub runner_approved: bool,
    pub model_calls_approved: bool,
    pub would_run_external_serializer: bool,
    pub would_run_external_rlm: bool,
    pub would_run_model_calls: bool,
    pub tool_ran_external_process: bool,
    pub tool_ran_model_calls: bool,
    pub tool_wrote_reduced_context: bool,
    pub dry_run_plan_only: bool,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmRunnerGateReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub runner_gate_schema: &'static str,
    pub reducer: String,
    pub runner_ready: bool,
    pub step_count: usize,
    pub next_action: String,
}

pub(crate) fn build_serializer_rlm_runner_gate(
    request: DxSerializerRlmRunnerGateRequest,
) -> Result<DxSerializerRlmRunnerGate, String> {
    let plan_value = decode_json_string(&request.execution_plan)?;
    let (plan, receipt_schema) = locate_execution_plan(&plan_value).ok_or_else(|| {
        "DX serializer/RLM runner gate needs a zed.dx.serializer_rlm.execution_plan.v1 object or execution receipt."
            .to_string()
    })?;
    let summary = summarize_execution_plan(plan, receipt_schema);
    let blockers = runner_blockers(&request, &summary);
    let runner_ready = blockers.is_empty();
    let status = if runner_ready {
        "runner_ready"
    } else if request.approve_runner {
        "blocked_after_approval"
    } else {
        "approval_required"
    };
    let next_action = if runner_ready {
        "Use this runner-gate receipt to add the serializer/RLM reducer executor under the same approval and model-call gates."
            .to_string()
    } else if request.approve_runner {
        "Resolve the listed serializer/RLM runner blockers before any reducer execution is wired."
            .to_string()
    } else {
        "Review this runner gate, then rerun with approve_runner=true after the execution receipt and model-call policy are acceptable."
            .to_string()
    };

    Ok(DxSerializerRlmRunnerGate {
        schema: DX_SERIALIZER_RLM_RUNNER_GATE_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxSerializerRlmRunnerGateRequestSummary {
            approve_runner: request.approve_runner,
            allow_model_calls: request.allow_model_calls,
            require_execution_receipt: request.require_execution_receipt,
            root_mode: request.root_mode,
        },
        validation: DxSerializerRlmRunnerGateValidation {
            status: status.to_string(),
            runner_ready,
            permission_required: true,
            runner_approved: request.approve_runner,
            model_calls_approved: request.allow_model_calls,
            would_run_external_serializer: summary.serializer_step_count > 0 && runner_ready,
            would_run_external_rlm: summary.rlm_step_count > 0 && runner_ready,
            would_run_model_calls: summary.model_call_step_count > 0 && runner_ready,
            tool_ran_external_process: false,
            tool_ran_model_calls: false,
            tool_wrote_reduced_context: false,
            dry_run_plan_only: true,
            blockers,
        },
        plan: summary,
        runner_receipt: None,
        next_action,
    })
}

fn runner_blockers(
    request: &DxSerializerRlmRunnerGateRequest,
    plan: &DxSerializerRlmRunnerGatePlanSummary,
) -> Vec<String> {
    let mut blockers = Vec::new();

    if !request.approve_runner {
        blockers.push("Serializer/RLM runner execution has not been approved.".to_string());
    }
    if request.require_execution_receipt && !plan.received_from_execution_receipt {
        blockers.push(
            "A managed serializer/RLM execution receipt is required before runner approval."
                .to_string(),
        );
    }
    if plan.schema != DX_SERIALIZER_RLM_EXECUTION_PLAN_SCHEMA {
        blockers.push(format!(
            "Expected execution plan schema {DX_SERIALIZER_RLM_EXECUTION_PLAN_SCHEMA}, got {}.",
            plan.schema
        ));
    }
    if plan.approval_status != "approved_plan_ready" {
        blockers.push(format!(
            "Execution plan approval status is `{}` instead of `approved_plan_ready`.",
            plan.approval_status
        ));
    }
    if !plan.external_runner_ready {
        blockers.push("Execution plan is not marked external_runner_ready.".to_string());
    }
    if !plan.external_execution_approved {
        blockers
            .push("Execution plan did not approve external serializer/RLM execution.".to_string());
    }
    if !plan.dry_run_only {
        blockers
            .push("Execution plan must remain dry_run_only before the runner gate.".to_string());
    }
    if plan.tool_executed_external_process {
        blockers.push("Execution plan tool unexpectedly ran an external process.".to_string());
    }
    if plan.tool_ran_model_calls {
        blockers.push("Execution plan tool unexpectedly ran model calls.".to_string());
    }
    if plan.step_count == 0 {
        blockers.push("Execution plan contains no reducer steps.".to_string());
    }
    if plan.ready_step_count != plan.step_count {
        blockers.push(format!(
            "{} of {} reducer step(s) are ready for the future runner.",
            plan.ready_step_count, plan.step_count
        ));
    }
    if plan.model_call_step_count > 0 && !request.allow_model_calls {
        blockers.push(
            "RLM reducer steps require allow_model_calls=true before runner approval.".to_string(),
        );
    }

    blockers
}

fn summarize_execution_plan(
    plan: &Value,
    receipt_schema: Option<String>,
) -> DxSerializerRlmRunnerGatePlanSummary {
    let request = plan.get("request").unwrap_or(&Value::Null);
    let approval = plan.get("approval").unwrap_or(&Value::Null);
    let context = plan.get("context").unwrap_or(&Value::Null);
    let steps = plan
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let step_summaries = steps
        .iter()
        .map(|step| DxSerializerRlmRunnerGateStepSummary {
            step_id: string_field(step, &["step_id"]).unwrap_or_else(|| "unknown".to_string()),
            target: string_field(step, &["target"]).unwrap_or_else(|| "unknown".to_string()),
            status: string_field(step, &["status"]).unwrap_or_else(|| "unknown".to_string()),
            external_process: bool_field(step, &["external_process"]).unwrap_or(false),
            model_calls: bool_field(step, &["model_calls"]).unwrap_or(false),
            blocker_count: array_len(step, &["blockers"]),
        })
        .collect::<Vec<_>>();
    let ready_step_count = step_summaries
        .iter()
        .filter(|step| step.status == "ready_for_future_runner" && step.blocker_count == 0)
        .count();
    let serializer_step_count = step_summaries
        .iter()
        .filter(|step| step.target == "serializer")
        .count();
    let rlm_step_count = step_summaries
        .iter()
        .filter(|step| step.target == "rlm")
        .count();
    let model_call_step_count = step_summaries
        .iter()
        .filter(|step| step.model_calls)
        .count();

    DxSerializerRlmRunnerGatePlanSummary {
        schema: string_field(plan, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        received_from_execution_receipt: receipt_schema.is_some(),
        execution_receipt_schema: receipt_schema,
        approval_status: string_field(approval, &["status"])
            .unwrap_or_else(|| "unknown".to_string()),
        reducer: string_field(request, &["reducer"]).unwrap_or_else(|| "unknown".to_string()),
        task: string_field(request, &["task"]),
        external_runner_ready: bool_field(approval, &["external_runner_ready"]).unwrap_or(false),
        external_execution_approved: bool_field(approval, &["external_execution_approved"])
            .unwrap_or(false),
        dry_run_only: bool_field(approval, &["dry_run_only"]).unwrap_or(false),
        tool_executed_external_process: bool_field(approval, &["tool_executed_external_process"])
            .unwrap_or(false),
        tool_ran_model_calls: bool_field(approval, &["tool_ran_model_calls"]).unwrap_or(false),
        included_source_count: usize_field(context, &["included_source_count"]).unwrap_or_default(),
        estimated_tokens: usize_field(context, &["estimated_tokens"]).unwrap_or_default(),
        step_count: step_summaries.len(),
        ready_step_count,
        serializer_step_count,
        rlm_step_count,
        model_call_step_count,
        steps: step_summaries,
    }
}

fn locate_execution_plan(value: &Value) -> Option<(&Value, Option<String>)> {
    if value.get("schema").and_then(Value::as_str) == Some(DX_SERIALIZER_RLM_EXECUTION_PLAN_SCHEMA)
    {
        return Some((value, None));
    }

    let receipt_schema = value
        .get("schema")
        .and_then(Value::as_str)
        .filter(|schema| *schema == DX_SERIALIZER_RLM_EXECUTION_RECEIPT_SCHEMA)
        .map(ToOwned::to_owned);
    let plan = value.get("execution_plan").filter(|plan| {
        plan.get("schema").and_then(Value::as_str) == Some(DX_SERIALIZER_RLM_EXECUTION_PLAN_SCHEMA)
    })?;

    Some((plan, receipt_schema))
}

fn decode_json_string(value: &Value) -> Result<Value, String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if text.starts_with('{') {
            return serde_json::from_str(text).map_err(|error| {
                format!("Failed to parse stringified DX serializer/RLM execution JSON: {error}")
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
