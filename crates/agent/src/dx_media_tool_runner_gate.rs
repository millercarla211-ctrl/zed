use serde::Serialize;
use serde_json::Value;
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const DX_MEDIA_TOOL_PLAN_SCHEMA: &str = "zed.dx.media_tool.plan.v1";

pub(crate) const DX_MEDIA_TOOL_RUNNER_GATE_SCHEMA: &str = "zed.dx.media_tool.runner_gate.v1";
pub(crate) const DX_MEDIA_TOOL_RUNNER_GATE_RECEIPT_SCHEMA: &str =
    "zed.dx.media_tool.runner_gate_receipt.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxMediaToolRunnerGateRequest {
    pub media_plan: Value,
    pub approve_runner: bool,
    pub require_existing_source: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolRunnerGate {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxMediaToolRunnerGateRequestSummary,
    pub plan: DxMediaToolRunnerGatePlanSummary,
    pub validation: DxMediaToolRunnerGateValidation,
    pub runner_receipt: Option<DxMediaToolRunnerGateReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolRunnerGateRequestSummary {
    pub approve_runner: bool,
    pub require_existing_source: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolRunnerGatePlanSummary {
    pub schema: String,
    pub action: String,
    pub plan_status: String,
    pub executable_tool: String,
    pub argument_count: usize,
    pub managed_output_dir: Option<String>,
    pub planned_output_count: usize,
    pub source_kind: String,
    pub media_kind: String,
    pub source_exists: bool,
    pub source_path: Option<String>,
    pub plan_approved: bool,
    pub dry_run_only: bool,
    pub no_shell_string: bool,
    pub output_under_managed_root: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolRunnerGateValidation {
    pub status: String,
    pub runner_ready: bool,
    pub permission_required: bool,
    pub runner_approved: bool,
    pub would_run_external_process: bool,
    pub tool_ran_external_process: bool,
    pub tool_wrote_media_outputs: bool,
    pub tool_deleted_files: bool,
    pub tool_overwrites_outputs: bool,
    pub no_shell_interpolation: bool,
    pub argument_vector_safe: bool,
    pub executable_matches_plan: bool,
    pub outputs_managed: bool,
    pub blockers: Vec<String>,
    pub argument_vector: Vec<String>,
    pub planned_outputs: Vec<DxMediaToolRunnerGateOutput>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolRunnerGateOutput {
    pub label: String,
    pub path: String,
    pub media_kind: String,
    pub format: String,
    pub under_managed_output_dir: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolRunnerGateReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub runner_gate_schema: &'static str,
    pub action: String,
    pub runner_ready: bool,
    pub planned_output_count: usize,
    pub next_action: String,
}

pub(crate) fn build_dx_media_tool_runner_gate(
    request: DxMediaToolRunnerGateRequest,
) -> Result<DxMediaToolRunnerGate, String> {
    let media_plan_value = decode_json_string(&request.media_plan)?;
    let media_plan = locate_media_plan(&media_plan_value).ok_or_else(|| {
        "DX media runner gate needs a zed.dx.media_tool.plan.v1 object or plan receipt.".to_string()
    })?;
    let summary = summarize_plan(media_plan);
    let argument_vector = string_array(media_plan, &["action_plan", "argument_vector"]);
    let planned_outputs = planned_outputs(media_plan, summary.managed_output_dir.as_deref());
    let argument_vector_safe = argument_vector_is_safe(&argument_vector);
    let executable_matches_plan = executable_matches_plan(&argument_vector, &summary);
    let outputs_managed = planned_outputs
        .iter()
        .all(|output| output.under_managed_output_dir);

    let mut blockers = Vec::new();
    if !request.approve_runner {
        blockers.push("Runner execution has not been approved for this gate.".to_string());
    }
    if summary.schema != DX_MEDIA_TOOL_PLAN_SCHEMA {
        blockers.push(format!(
            "Expected media plan schema {DX_MEDIA_TOOL_PLAN_SCHEMA}, got {}.",
            summary.schema
        ));
    }
    if summary.plan_status != "approved_plan_ready" {
        blockers.push(format!(
            "Media plan status is `{}` instead of `approved_plan_ready`.",
            summary.plan_status
        ));
    }
    if request.require_existing_source && !summary.source_exists {
        blockers.push("Media source must exist before runner execution.".to_string());
    }
    if summary.source_kind == "remote_url" {
        blockers.push(
            "Remote URL media requires a managed download/source receipt before runner execution."
                .to_string(),
        );
    }
    if !summary.plan_approved {
        blockers.push("Media plan was not approved for execution.".to_string());
    }
    if !summary.dry_run_only {
        blockers
            .push("Media plan must be a dry-run plan before the runner gate executes.".to_string());
    }
    if !summary.no_shell_string {
        blockers.push("Media plan must declare no_shell_string=true.".to_string());
    }
    if !summary.output_under_managed_root {
        blockers.push("Media plan must keep outputs under the managed root.".to_string());
    }
    if argument_vector.is_empty() {
        blockers.push("Media plan argument_vector is empty.".to_string());
    }
    if !argument_vector_safe {
        blockers.push(
            "Media plan argument_vector contains shell-wrapper or shell-operator risk.".to_string(),
        );
    }
    if !executable_matches_plan {
        blockers.push(
            "Media plan executable does not match the requested ffmpeg/ffprobe action.".to_string(),
        );
    }
    if summary.action != "inspect" && planned_outputs.is_empty() {
        blockers
            .push("Media extraction plans must include at least one planned output.".to_string());
    }
    if !outputs_managed {
        blockers.push(
            "One or more planned outputs are outside the managed output directory.".to_string(),
        );
    }

    let runner_ready = blockers.is_empty();
    let status = if runner_ready {
        "runner_ready"
    } else if request.approve_runner {
        "blocked_after_approval"
    } else {
        "approval_required"
    };
    let next_action = if runner_ready {
        "Use this gate receipt to run the future no-shell ffmpeg runner, then write produced-file receipts before returning media to the Agent."
            .to_string()
    } else if request.approve_runner {
        "Resolve the listed runner blockers before executing any media command.".to_string()
    } else {
        "Review this runner gate, then rerun with approve_runner=true when ready to authorize the future no-shell runner."
            .to_string()
    };

    Ok(DxMediaToolRunnerGate {
        schema: DX_MEDIA_TOOL_RUNNER_GATE_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxMediaToolRunnerGateRequestSummary {
            approve_runner: request.approve_runner,
            require_existing_source: request.require_existing_source,
            root_mode: request.root_mode,
        },
        plan: summary,
        validation: DxMediaToolRunnerGateValidation {
            status: status.to_string(),
            runner_ready,
            permission_required: true,
            runner_approved: request.approve_runner,
            would_run_external_process: runner_ready,
            tool_ran_external_process: false,
            tool_wrote_media_outputs: false,
            tool_deleted_files: false,
            tool_overwrites_outputs: false,
            no_shell_interpolation: true,
            argument_vector_safe,
            executable_matches_plan,
            outputs_managed,
            blockers,
            argument_vector,
            planned_outputs,
        },
        runner_receipt: None,
        next_action,
    })
}

fn summarize_plan(plan: &Value) -> DxMediaToolRunnerGatePlanSummary {
    DxMediaToolRunnerGatePlanSummary {
        schema: string_field(plan, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        action: string_field(plan, &["action_plan", "action"])
            .unwrap_or_else(|| "unknown".to_string()),
        plan_status: string_field(plan, &["action_plan", "status"])
            .unwrap_or_else(|| "unknown".to_string()),
        executable_tool: string_field(plan, &["action_plan", "executable_tool"])
            .unwrap_or_else(|| "unknown".to_string()),
        argument_count: array_len(plan, &["action_plan", "argument_vector"]),
        managed_output_dir: string_field(plan, &["action_plan", "managed_output_dir"]),
        planned_output_count: array_len(plan, &["action_plan", "planned_outputs"]),
        source_kind: string_field(plan, &["source", "source_kind"])
            .unwrap_or_else(|| "unknown".to_string()),
        media_kind: string_field(plan, &["source", "media_kind"])
            .unwrap_or_else(|| "unknown".to_string()),
        source_exists: bool_field(plan, &["source", "exists"]).unwrap_or(false),
        source_path: string_field(plan, &["source", "resolved_path"]),
        plan_approved: bool_field(plan, &["safety", "media_execution_approved"]).unwrap_or(false),
        dry_run_only: bool_field(plan, &["safety", "dry_run_only"]).unwrap_or(false),
        no_shell_string: bool_field(plan, &["safety", "no_shell_string"]).unwrap_or(false),
        output_under_managed_root: bool_field(plan, &["safety", "output_under_managed_root"])
            .unwrap_or(false),
    }
}

fn planned_outputs(
    plan: &Value,
    managed_output_dir: Option<&str>,
) -> Vec<DxMediaToolRunnerGateOutput> {
    let managed_output_dir = managed_output_dir.map(Path::new);
    value_at(plan, &["action_plan", "planned_outputs"])
        .and_then(Value::as_array)
        .map(|outputs| {
            outputs
                .iter()
                .filter_map(|output| {
                    let path = string_field(output, &["path"])?;
                    let under_managed_output_dir = managed_output_dir
                        .map(|root| Path::new(&path).starts_with(root))
                        .unwrap_or(false);
                    Some(DxMediaToolRunnerGateOutput {
                        label: string_field(output, &["label"])
                            .unwrap_or_else(|| "output".to_string()),
                        path,
                        media_kind: string_field(output, &["media_kind"])
                            .unwrap_or_else(|| "unknown".to_string()),
                        format: string_field(output, &["format"])
                            .unwrap_or_else(|| "unknown".to_string()),
                        under_managed_output_dir,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn argument_vector_is_safe(arguments: &[String]) -> bool {
    let Some(executable) = arguments.first() else {
        return false;
    };
    let executable_name = executable_name(executable);
    if matches!(
        executable_name.as_str(),
        "cmd" | "powershell" | "pwsh" | "sh" | "bash" | "zsh"
    ) {
        return false;
    }

    arguments.iter().all(|argument| {
        !argument.contains("&&") && !argument.contains("||") && !argument.contains('\n')
    })
}

fn executable_matches_plan(arguments: &[String], plan: &DxMediaToolRunnerGatePlanSummary) -> bool {
    let Some(executable) = arguments.first() else {
        return false;
    };
    let executable_name = executable_name(executable);
    match plan.action.as_str() {
        "inspect" => plan.executable_tool == "ffprobe" && executable_name == "ffprobe",
        "extract_audio" | "extract_frame" => {
            plan.executable_tool == "ffmpeg" && executable_name == "ffmpeg"
        }
        _ => false,
    }
}

fn executable_name(executable: &str) -> String {
    Path::new(executable)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(executable)
        .to_ascii_lowercase()
}

fn locate_media_plan(value: &Value) -> Option<&Value> {
    if value.get("schema").and_then(Value::as_str) == Some(DX_MEDIA_TOOL_PLAN_SCHEMA) {
        return Some(value);
    }

    value.get("media_plan").filter(|plan| {
        plan.get("schema").and_then(Value::as_str) == Some(DX_MEDIA_TOOL_PLAN_SCHEMA)
    })
}

fn decode_json_string(value: &Value) -> Result<Value, String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if text.starts_with('{') {
            return serde_json::from_str(text).map_err(|error| {
                format!("Failed to parse stringified DX media plan JSON: {error}")
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

fn array_len(value: &Value, path: &[&str]) -> usize {
    value_at(value, path)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}

fn string_array(value: &Value, path: &[&str]) -> Vec<String> {
    value_at(value, path)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
