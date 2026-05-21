use crate::{
    dx_serializer_rlm_execution_preview::{
        DX_SERIALIZER_RLM_EXECUTION_PREVIEW_RECEIPT_SCHEMA,
        DX_SERIALIZER_RLM_EXECUTION_PREVIEW_SCHEMA,
    },
    dx_serializer_rlm_reduced_context::{
        DX_SERIALIZER_RLM_REDUCED_CONTEXT_RECEIPT_SCHEMA, DX_SERIALIZER_RLM_REDUCED_CONTEXT_SCHEMA,
    },
};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    env,
    io::Write,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const DX_SERIALIZER_ROOT_ENV: &str = "DX_SERIALIZER_ROOT";
const DX_RLM_ROOT_ENV: &str = "DX_RLM_ROOT";
const DX_HOME_ENV: &str = "DX_HOME";
const DX_SERIALIZER_RLM_RUNNER_ROOT_ENV: &str = "DX_SERIALIZER_RLM_RUNNER_ROOT";
const DEFAULT_SERIALIZER_ROOT: &str = r"G:\Workspaces\flow\serializer";
const DEFAULT_RLM_ROOT: &str = r"G:\Workspaces\flow\rlm";
const DEFAULT_DX_ROOT: &str = r"G:\Dx";
const DEFAULT_MAX_STDIN_CHARS: usize = 120_000;
const MAX_STDIN_CHARS: usize = 500_000;
const DEFAULT_MAX_OUTPUT_PREVIEW_CHARS: usize = 4_000;
const MAX_OUTPUT_PREVIEW_CHARS: usize = 20_000;

pub(crate) const DX_SERIALIZER_RLM_EXTERNAL_EXECUTION_SCHEMA: &str =
    "zed.dx.serializer_rlm.external_execution.v1";
pub(crate) const DX_SERIALIZER_RLM_EXTERNAL_EXECUTION_RECEIPT_SCHEMA: &str =
    "zed.dx.serializer_rlm.external_execution_receipt.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxSerializerRlmExternalExecutionRequest {
    pub execution_preview: Value,
    pub reduced_context: Value,
    pub command_vector: Vec<String>,
    pub approve_external_execution: bool,
    pub allow_model_calls: bool,
    pub require_execution_receipt: bool,
    pub stdin_mode: DxSerializerRlmExternalExecutionStdinMode,
    pub max_stdin_chars: Option<usize>,
    pub max_output_preview_chars: Option<usize>,
    pub root_mode: String,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub(crate) enum DxSerializerRlmExternalExecutionStdinMode {
    None,
    ReducedContextText,
}

impl DxSerializerRlmExternalExecutionStdinMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ReducedContextText => "reduced_context_text",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExternalExecution {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxSerializerRlmExternalExecutionRequestSummary,
    pub preview: DxSerializerRlmExternalExecutionPreviewSummary,
    pub reduced_context: DxSerializerRlmExternalExecutionReducedContextSummary,
    pub command: DxSerializerRlmExternalExecutionCommandSummary,
    pub execution: DxSerializerRlmExternalExecutionSummary,
    pub external_execution_receipt: Option<DxSerializerRlmExternalExecutionReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExternalExecutionRequestSummary {
    pub approve_external_execution: bool,
    pub allow_model_calls: bool,
    pub require_execution_receipt: bool,
    pub stdin_mode: &'static str,
    pub max_stdin_chars: usize,
    pub max_output_preview_chars: usize,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExternalExecutionPreviewSummary {
    pub schema: String,
    pub received_from_preview_receipt: bool,
    pub preview_receipt_schema: Option<String>,
    pub status: String,
    pub execution_preview_ready: bool,
    pub dry_run_only: bool,
    pub reducer: String,
    pub step_count: usize,
    pub would_run_external_serializer: bool,
    pub would_run_external_rlm: bool,
    pub would_run_model_calls: bool,
    pub preview_tool_ran_external_process: bool,
    pub preview_tool_ran_model_calls: bool,
    pub preview_tool_wrote_execution_output: bool,
    pub blocker_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExternalExecutionReducedContextSummary {
    pub schema: String,
    pub received_from_reduced_context_receipt: bool,
    pub reduced_context_receipt_schema: Option<String>,
    pub status: String,
    pub reduced_context_ready: bool,
    pub source_count: usize,
    pub selected_estimated_tokens: usize,
    pub reduced_context_chars: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExternalExecutionCommandSummary {
    #[serde(skip_serializing)]
    pub command_vector: Vec<String>,
    pub command_vector_preview: Vec<String>,
    pub executable_path: String,
    pub executable_name: String,
    pub executable_exists: bool,
    pub executable_absolute: bool,
    pub executable_under_allowed_root: bool,
    pub shell_like_executable: bool,
    pub forbidden_build_tool: bool,
    pub arguments_safe: bool,
    pub allowed_roots: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExternalExecutionSummary {
    pub status: String,
    pub execution_ready: bool,
    pub permission_required: bool,
    pub external_execution_approved: bool,
    pub model_calls_approved: bool,
    pub ran_external_process: bool,
    pub ran_shell: bool,
    pub wrote_managed_receipt: bool,
    pub zed_wrote_unmanaged_files: bool,
    pub exit_code: Option<i32>,
    pub stdout_preview: String,
    pub stderr_preview: String,
    pub stdout_sha256: Option<String>,
    pub stderr_sha256: Option<String>,
    pub stdin_chars: usize,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxSerializerRlmExternalExecutionReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub execution_schema: &'static str,
    pub reducer: String,
    pub exit_code: Option<i32>,
    pub ran_external_process: bool,
    pub next_action: String,
}

pub(crate) fn execute_serializer_rlm_external_reducer(
    request: DxSerializerRlmExternalExecutionRequest,
) -> Result<DxSerializerRlmExternalExecution, String> {
    let preview_value = decode_json_string(&request.execution_preview, "execution preview")?;
    let (preview, preview_receipt_schema) = locate_execution_preview(&preview_value).ok_or_else(|| {
        "DX serializer/RLM external execution needs a zed.dx.serializer_rlm.execution_preview.v1 object or receipt."
            .to_string()
    })?;
    let preview = summarize_preview(preview, preview_receipt_schema);

    let reduced_context_value = decode_json_string(&request.reduced_context, "reduced context")?;
    let (reduced_context, reduced_context_receipt_schema) =
        locate_reduced_context(&reduced_context_value).ok_or_else(|| {
            "DX serializer/RLM external execution needs a zed.dx.serializer_rlm.reduced_context.v1 object or receipt."
                .to_string()
        })?;
    let reduced_context_summary =
        summarize_reduced_context(reduced_context, reduced_context_receipt_schema);
    let reduced_context_text =
        string_field(reduced_context, &["reduced_context_text"]).unwrap_or_default();
    let max_stdin_chars = request
        .max_stdin_chars
        .unwrap_or(DEFAULT_MAX_STDIN_CHARS)
        .clamp(1, MAX_STDIN_CHARS);
    let max_output_preview_chars = request
        .max_output_preview_chars
        .unwrap_or(DEFAULT_MAX_OUTPUT_PREVIEW_CHARS)
        .clamp(1, MAX_OUTPUT_PREVIEW_CHARS);
    let stdin_text = stdin_text_for_request(&request, &reduced_context_text, max_stdin_chars);
    let allowed_roots = allowed_execution_roots();
    let command = summarize_command(&request.command_vector, &allowed_roots);
    let blockers = execution_blockers(
        &request,
        &preview,
        &reduced_context_summary,
        &command,
        stdin_text.as_deref(),
        reduced_context_text.chars().count(),
        max_stdin_chars,
    );

    if !blockers.is_empty() {
        return Ok(blocked_response(
            request,
            preview,
            reduced_context_summary,
            command,
            blockers,
            max_stdin_chars,
            max_output_preview_chars,
        ));
    }

    run_external_command(
        request,
        preview,
        reduced_context_summary,
        command,
        stdin_text,
        max_stdin_chars,
        max_output_preview_chars,
    )
}

fn run_external_command(
    request: DxSerializerRlmExternalExecutionRequest,
    preview: DxSerializerRlmExternalExecutionPreviewSummary,
    reduced_context: DxSerializerRlmExternalExecutionReducedContextSummary,
    command: DxSerializerRlmExternalExecutionCommandSummary,
    stdin_text: Option<String>,
    max_stdin_chars: usize,
    max_output_preview_chars: usize,
) -> Result<DxSerializerRlmExternalExecution, String> {
    let mut process = Command::new(&command.command_vector[0]);
    process.args(&command.command_vector[1..]);
    process.stdout(Stdio::piped());
    process.stderr(Stdio::piped());

    let stdin_chars = stdin_text
        .as_deref()
        .map(|text| text.chars().count())
        .unwrap_or_default();
    let output = if let Some(stdin_text) = stdin_text.as_deref() {
        process.stdin(Stdio::piped());
        let mut child = match process.spawn() {
            Ok(child) => child,
            Err(error) => {
                return Ok(process_error_response(
                    request,
                    preview,
                    reduced_context,
                    command,
                    max_stdin_chars,
                    max_output_preview_chars,
                    "process_spawn_failed",
                    false,
                    None,
                    String::new(),
                    error.to_string(),
                    stdin_chars,
                ));
            }
        };
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(error) = stdin.write_all(stdin_text.as_bytes()) {
                let mut stderr_preview = format!("stdin_write_error: {error}");
                let mut stdout_preview = String::new();
                let mut exit_code = None;
                drop(stdin);
                if let Ok(output) = child.wait_with_output() {
                    stdout_preview = preview_bytes(&output.stdout, max_output_preview_chars);
                    exit_code = output.status.code();
                    let process_stderr = preview_bytes(&output.stderr, max_output_preview_chars);
                    if !process_stderr.is_empty() {
                        stderr_preview = format!("{process_stderr}\n{stderr_preview}");
                    }
                }
                return Ok(process_error_response(
                    request,
                    preview,
                    reduced_context,
                    command,
                    max_stdin_chars,
                    max_output_preview_chars,
                    "stdin_write_failed",
                    true,
                    exit_code,
                    stdout_preview,
                    stderr_preview,
                    stdin_chars,
                ));
            }
        }
        match child.wait_with_output() {
            Ok(output) => output,
            Err(error) => {
                return Ok(process_error_response(
                    request,
                    preview,
                    reduced_context,
                    command,
                    max_stdin_chars,
                    max_output_preview_chars,
                    "process_wait_failed",
                    true,
                    None,
                    String::new(),
                    error.to_string(),
                    stdin_chars,
                ));
            }
        }
    } else {
        match process.output() {
            Ok(output) => output,
            Err(error) => {
                return Ok(process_error_response(
                    request,
                    preview,
                    reduced_context,
                    command,
                    max_stdin_chars,
                    max_output_preview_chars,
                    "process_spawn_failed",
                    false,
                    None,
                    String::new(),
                    error.to_string(),
                    0,
                ));
            }
        }
    };

    let process_success = output.status.success();
    let status = if process_success {
        "external_execution_succeeded"
    } else {
        "external_execution_failed"
    };
    let next_action = if process_success {
        "Review the managed external execution receipt before using reducer stdout in Agent context."
    } else {
        "Inspect stderr_preview, repair the reducer command or receipts, and rerun only after review."
    };

    Ok(DxSerializerRlmExternalExecution {
        schema: DX_SERIALIZER_RLM_EXTERNAL_EXECUTION_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: request_summary(&request, max_stdin_chars, max_output_preview_chars),
        preview,
        reduced_context,
        command,
        execution: DxSerializerRlmExternalExecutionSummary {
            status: status.to_string(),
            execution_ready: true,
            permission_required: true,
            external_execution_approved: true,
            model_calls_approved: request.allow_model_calls,
            ran_external_process: true,
            ran_shell: false,
            wrote_managed_receipt: false,
            zed_wrote_unmanaged_files: false,
            exit_code: output.status.code(),
            stdout_preview: preview_bytes(&output.stdout, max_output_preview_chars),
            stderr_preview: preview_bytes(&output.stderr, max_output_preview_chars),
            stdout_sha256: Some(hex_digest(&Sha256::digest(&output.stdout))),
            stderr_sha256: Some(hex_digest(&Sha256::digest(&output.stderr))),
            stdin_chars,
            blockers: Vec::new(),
        },
        external_execution_receipt: None,
        next_action: next_action.to_string(),
    })
}

fn blocked_response(
    request: DxSerializerRlmExternalExecutionRequest,
    preview: DxSerializerRlmExternalExecutionPreviewSummary,
    reduced_context: DxSerializerRlmExternalExecutionReducedContextSummary,
    command: DxSerializerRlmExternalExecutionCommandSummary,
    blockers: Vec<String>,
    max_stdin_chars: usize,
    max_output_preview_chars: usize,
) -> DxSerializerRlmExternalExecution {
    let status = if request.approve_external_execution {
        "blocked_after_external_execution_approval"
    } else {
        "external_execution_approval_required"
    };

    DxSerializerRlmExternalExecution {
        schema: DX_SERIALIZER_RLM_EXTERNAL_EXECUTION_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: request_summary(&request, max_stdin_chars, max_output_preview_chars),
        preview,
        reduced_context,
        command,
        execution: DxSerializerRlmExternalExecutionSummary {
            status: status.to_string(),
            execution_ready: false,
            permission_required: true,
            external_execution_approved: request.approve_external_execution,
            model_calls_approved: request.allow_model_calls,
            ran_external_process: false,
            ran_shell: false,
            wrote_managed_receipt: false,
            zed_wrote_unmanaged_files: false,
            exit_code: None,
            stdout_preview: String::new(),
            stderr_preview: String::new(),
            stdout_sha256: None,
            stderr_sha256: None,
            stdin_chars: 0,
            blockers,
        },
        external_execution_receipt: None,
        next_action:
            "Resolve the listed external reducer execution blockers before running any process."
                .to_string(),
    }
}

fn process_error_response(
    request: DxSerializerRlmExternalExecutionRequest,
    preview: DxSerializerRlmExternalExecutionPreviewSummary,
    reduced_context: DxSerializerRlmExternalExecutionReducedContextSummary,
    command: DxSerializerRlmExternalExecutionCommandSummary,
    max_stdin_chars: usize,
    max_output_preview_chars: usize,
    status: &'static str,
    ran_external_process: bool,
    exit_code: Option<i32>,
    stdout_preview: String,
    stderr_preview: String,
    stdin_chars: usize,
) -> DxSerializerRlmExternalExecution {
    DxSerializerRlmExternalExecution {
        schema: DX_SERIALIZER_RLM_EXTERNAL_EXECUTION_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: request_summary(&request, max_stdin_chars, max_output_preview_chars),
        preview,
        reduced_context,
        command,
        execution: DxSerializerRlmExternalExecutionSummary {
            status: status.to_string(),
            execution_ready: true,
            permission_required: true,
            external_execution_approved: true,
            model_calls_approved: request.allow_model_calls,
            ran_external_process,
            ran_shell: false,
            wrote_managed_receipt: false,
            zed_wrote_unmanaged_files: false,
            exit_code,
            stdout_preview,
            stderr_preview,
            stdout_sha256: None,
            stderr_sha256: None,
            stdin_chars,
            blockers: Vec::new(),
        },
        external_execution_receipt: None,
        next_action:
            "Inspect stderr_preview, repair the reducer command or receipts, and rerun only after review."
                .to_string(),
    }
}

fn execution_blockers(
    request: &DxSerializerRlmExternalExecutionRequest,
    preview: &DxSerializerRlmExternalExecutionPreviewSummary,
    reduced_context: &DxSerializerRlmExternalExecutionReducedContextSummary,
    command: &DxSerializerRlmExternalExecutionCommandSummary,
    stdin_text: Option<&str>,
    reduced_context_chars: usize,
    max_stdin_chars: usize,
) -> Vec<String> {
    let mut blockers = Vec::new();

    if !request.approve_external_execution {
        blockers
            .push("External serializer/RLM reducer execution has not been approved.".to_string());
    }
    if !request.require_execution_receipt {
        blockers.push(
            "Approved external serializer/RLM execution requires a managed execution receipt."
                .to_string(),
        );
    }
    if preview.schema != DX_SERIALIZER_RLM_EXECUTION_PREVIEW_SCHEMA {
        blockers.push(format!(
            "Expected execution preview schema {DX_SERIALIZER_RLM_EXECUTION_PREVIEW_SCHEMA}, got {}.",
            preview.schema
        ));
    }
    if !preview.execution_preview_ready {
        blockers.push(format!(
            "Execution preview status is `{}` instead of `execution_preview_ready`.",
            preview.status
        ));
    }
    if !preview.dry_run_only {
        blockers.push("Execution preview must be dry_run_only.".to_string());
    }
    if preview.preview_tool_ran_external_process
        || preview.preview_tool_ran_model_calls
        || preview.preview_tool_wrote_execution_output
    {
        blockers
            .push("Execution preview receipt already reports unsafe prior execution.".to_string());
    }
    if preview.would_run_model_calls && !request.allow_model_calls {
        blockers.push(
            "Reducer preview includes model-call steps but allow_model_calls=false.".to_string(),
        );
    }
    if reduced_context.schema != DX_SERIALIZER_RLM_REDUCED_CONTEXT_SCHEMA {
        blockers.push(format!(
            "Expected reduced-context schema {DX_SERIALIZER_RLM_REDUCED_CONTEXT_SCHEMA}, got {}.",
            reduced_context.schema
        ));
    }
    if !reduced_context.reduced_context_ready {
        blockers.push(format!(
            "Reduced-context status is `{}` instead of `reduced_context_ready`.",
            reduced_context.status
        ));
    }
    if request.stdin_mode == DxSerializerRlmExternalExecutionStdinMode::ReducedContextText
        && reduced_context_chars == 0
    {
        blockers.push("Reduced-context stdin mode needs reduced_context_text.".to_string());
    }
    if request.stdin_mode == DxSerializerRlmExternalExecutionStdinMode::ReducedContextText
        && stdin_text.is_none()
    {
        blockers.push(format!(
            "Reduced-context text exceeds max_stdin_chars={max_stdin_chars}."
        ));
    }
    if command.command_vector.is_empty() {
        blockers.push("External reducer command_vector is empty.".to_string());
    }
    if !command.executable_absolute {
        blockers.push("External reducer executable must be an absolute path.".to_string());
    }
    if !command.executable_exists {
        blockers.push("External reducer executable does not exist.".to_string());
    }
    if !command.executable_under_allowed_root {
        blockers.push(
            "External reducer executable is outside approved DX serializer/RLM roots.".to_string(),
        );
    }
    if command.shell_like_executable {
        blockers.push(
            "Shell executables are not allowed for serializer/RLM reducer execution.".to_string(),
        );
    }
    if command.forbidden_build_tool {
        blockers.push("Build, package, compiler, and script interpreter tools such as cargo, just, npm, pnpm, yarn, bun, node, deno, and python are not allowed here.".to_string());
    }
    if !command.arguments_safe {
        blockers.push("External reducer command arguments failed shell-safety checks.".to_string());
    }

    blockers
}

fn request_summary(
    request: &DxSerializerRlmExternalExecutionRequest,
    max_stdin_chars: usize,
    max_output_preview_chars: usize,
) -> DxSerializerRlmExternalExecutionRequestSummary {
    DxSerializerRlmExternalExecutionRequestSummary {
        approve_external_execution: request.approve_external_execution,
        allow_model_calls: request.allow_model_calls,
        require_execution_receipt: request.require_execution_receipt,
        stdin_mode: request.stdin_mode.as_str(),
        max_stdin_chars,
        max_output_preview_chars,
        root_mode: request.root_mode.clone(),
    }
}

fn summarize_preview(
    preview: &Value,
    receipt_schema: Option<String>,
) -> DxSerializerRlmExternalExecutionPreviewSummary {
    let preview_summary = preview.get("preview").unwrap_or(&Value::Null);
    let gate = preview.get("gate").unwrap_or(&Value::Null);

    DxSerializerRlmExternalExecutionPreviewSummary {
        schema: string_field(preview, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        received_from_preview_receipt: receipt_schema.is_some(),
        preview_receipt_schema: receipt_schema,
        status: string_field(preview_summary, &["status"]).unwrap_or_else(|| "unknown".to_string()),
        execution_preview_ready: bool_field(preview_summary, &["execution_preview_ready"])
            .unwrap_or(false),
        dry_run_only: bool_field(preview_summary, &["dry_run_only"]).unwrap_or(false),
        reducer: string_field(gate, &["reducer"]).unwrap_or_else(|| "unknown".to_string()),
        step_count: array_len(preview, &["planned_steps"]),
        would_run_external_serializer: bool_field(
            preview_summary,
            &["would_run_external_serializer"],
        )
        .unwrap_or(false),
        would_run_external_rlm: bool_field(preview_summary, &["would_run_external_rlm"])
            .unwrap_or(false),
        would_run_model_calls: bool_field(preview_summary, &["would_run_model_calls"])
            .unwrap_or(false),
        preview_tool_ran_external_process: bool_field(
            preview_summary,
            &["tool_ran_external_process"],
        )
        .unwrap_or(false),
        preview_tool_ran_model_calls: bool_field(preview_summary, &["tool_ran_model_calls"])
            .unwrap_or(false),
        preview_tool_wrote_execution_output: bool_field(
            preview_summary,
            &["tool_wrote_execution_output"],
        )
        .unwrap_or(false),
        blocker_count: array_len(preview_summary, &["blockers"]),
    }
}

fn summarize_reduced_context(
    reduced_context: &Value,
    receipt_schema: Option<String>,
) -> DxSerializerRlmExternalExecutionReducedContextSummary {
    let reduction = reduced_context.get("reduction").unwrap_or(&Value::Null);
    let reduced_context_text =
        string_field(reduced_context, &["reduced_context_text"]).unwrap_or_default();

    DxSerializerRlmExternalExecutionReducedContextSummary {
        schema: string_field(reduced_context, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        received_from_reduced_context_receipt: receipt_schema.is_some(),
        reduced_context_receipt_schema: receipt_schema,
        status: string_field(reduction, &["status"]).unwrap_or_else(|| "unknown".to_string()),
        reduced_context_ready: bool_field(reduction, &["reduced_context_ready"]).unwrap_or(false),
        source_count: usize_field(reduction, &["source_count"]).unwrap_or_default(),
        selected_estimated_tokens: usize_field(reduction, &["selected_estimated_tokens"])
            .unwrap_or_default(),
        reduced_context_chars: reduced_context_text.chars().count(),
    }
}

fn summarize_command(
    command_vector: &[String],
    allowed_roots: &[PathBuf],
) -> DxSerializerRlmExternalExecutionCommandSummary {
    let executable_path = command_vector.first().cloned().unwrap_or_default();
    let executable = Path::new(&executable_path);
    let executable_name = executable_name(&executable_path);
    let executable_absolute = executable.is_absolute();
    let executable_exists = executable.is_file();
    let executable_under_allowed_root = executable_absolute
        && path_components_are_safe(executable)
        && allowed_roots
            .iter()
            .any(|root| executable.starts_with(root) && root.exists());
    let shell_like_executable = matches!(
        executable_name.as_str(),
        "cmd" | "powershell" | "pwsh" | "sh" | "bash" | "zsh"
    );
    let forbidden_build_tool = matches!(
        executable_name.as_str(),
        "cargo"
            | "just"
            | "npm"
            | "pnpm"
            | "yarn"
            | "bun"
            | "rustc"
            | "node"
            | "deno"
            | "python"
            | "python3"
            | "py"
    );

    DxSerializerRlmExternalExecutionCommandSummary {
        command_vector: command_vector.to_vec(),
        command_vector_preview: command_vector.iter().map(redacted_argument).collect(),
        executable_path,
        executable_name,
        executable_exists,
        executable_absolute,
        executable_under_allowed_root,
        shell_like_executable,
        forbidden_build_tool,
        arguments_safe: argument_vector_is_safe(command_vector),
        allowed_roots: allowed_roots.iter().map(path_string).collect(),
    }
}

fn stdin_text_for_request(
    request: &DxSerializerRlmExternalExecutionRequest,
    reduced_context_text: &str,
    max_stdin_chars: usize,
) -> Option<String> {
    if request.stdin_mode != DxSerializerRlmExternalExecutionStdinMode::ReducedContextText {
        return None;
    }
    if reduced_context_text.chars().count() > max_stdin_chars {
        return None;
    }

    Some(reduced_context_text.to_string())
}

fn locate_execution_preview(value: &Value) -> Option<(&Value, Option<String>)> {
    if value.get("schema").and_then(Value::as_str)
        == Some(DX_SERIALIZER_RLM_EXECUTION_PREVIEW_SCHEMA)
    {
        return Some((value, None));
    }

    let receipt_schema = value
        .get("schema")
        .and_then(Value::as_str)
        .filter(|schema| *schema == DX_SERIALIZER_RLM_EXECUTION_PREVIEW_RECEIPT_SCHEMA)
        .map(ToOwned::to_owned);
    let preview = value.get("execution_preview").filter(|preview| {
        preview.get("schema").and_then(Value::as_str)
            == Some(DX_SERIALIZER_RLM_EXECUTION_PREVIEW_SCHEMA)
    })?;

    Some((preview, receipt_schema))
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

fn allowed_execution_roots() -> Vec<PathBuf> {
    [
        env::var_os(DX_SERIALIZER_ROOT_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SERIALIZER_ROOT)),
        env::var_os(DX_RLM_ROOT_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_RLM_ROOT)),
        env::var_os(DX_HOME_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_DX_ROOT)),
        env::var_os(DX_SERIALIZER_RLM_RUNNER_ROOT_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_DX_ROOT)),
    ]
    .into_iter()
    .filter(|root| root.is_absolute() && path_components_are_safe(root))
    .collect()
}

fn argument_vector_is_safe(arguments: &[String]) -> bool {
    !arguments.is_empty()
        && arguments
            .iter()
            .all(|argument| !argument_has_shell_token(argument))
}

fn argument_has_shell_token(argument: &str) -> bool {
    argument.contains("&&")
        || argument.contains("||")
        || argument.contains('|')
        || argument.contains(';')
        || argument.contains('>')
        || argument.contains('<')
        || argument.contains('\n')
        || argument.contains('\r')
        || argument.contains('`')
        || argument.contains("$(")
}

fn redacted_argument(argument: &String) -> String {
    let lower = argument.to_ascii_lowercase();
    if lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("authorization")
        || lower.contains("bearer ")
        || lower.contains("password")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.starts_with("sk-")
    {
        return "[redacted-argument]".to_string();
    }

    argument.clone()
}

fn path_components_are_safe(path: &Path) -> bool {
    path.components()
        .all(|component| !matches!(component, Component::ParentDir))
}

fn executable_name(executable: &str) -> String {
    Path::new(executable)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(executable)
        .to_ascii_lowercase()
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

fn preview_bytes(bytes: &[u8], max_preview_chars: usize) -> String {
    let text = String::from_utf8_lossy(bytes);
    if text.chars().count() <= max_preview_chars {
        return text.to_string();
    }

    let mut preview = text.chars().take(max_preview_chars).collect::<String>();
    preview.push_str("...");
    preview
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
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
