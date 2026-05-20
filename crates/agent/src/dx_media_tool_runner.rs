use crate::dx_media_tool_runner_gate::DX_MEDIA_TOOL_RUNNER_GATE_SCHEMA;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const DX_MEDIA_TOOL_EXECUTION_SCHEMA: &str = "zed.dx.media_tool.execution.v1";
pub(crate) const DX_MEDIA_TOOL_EXECUTION_RECEIPT_SCHEMA: &str =
    "zed.dx.media_tool.execution_receipt.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxMediaToolExecutionRequest {
    pub runner_gate: Value,
    pub approve_execution: bool,
    pub require_execution_receipt: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolExecution {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxMediaToolExecutionRequestSummary,
    pub gate: DxMediaToolExecutionGateSummary,
    pub execution: DxMediaToolExecutionSummary,
    pub produced_files: Vec<DxMediaToolProducedFile>,
    pub execution_receipt: Option<DxMediaToolExecutionReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolExecutionRequestSummary {
    pub approve_execution: bool,
    pub require_execution_receipt: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolExecutionGateSummary {
    pub schema: String,
    pub validation_status: String,
    pub runner_ready: bool,
    pub runner_approved: bool,
    pub action: String,
    pub executable_tool: String,
    pub managed_output_dir: Option<String>,
    pub argument_vector: Vec<String>,
    pub planned_outputs: Vec<DxMediaToolExecutionPlannedOutput>,
    pub no_shell_interpolation: bool,
    pub argument_vector_safe: bool,
    pub executable_matches_plan: bool,
    pub outputs_managed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolExecutionPlannedOutput {
    pub label: String,
    pub path: String,
    pub media_kind: String,
    pub format: String,
    pub absolute_path: bool,
    pub safe_path_components: bool,
    pub under_managed_output_dir: bool,
    pub existed_before_execution: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolExecutionSummary {
    pub status: String,
    pub execution_ready: bool,
    pub permission_required: bool,
    pub execution_approved: bool,
    pub ran_external_process: bool,
    pub ran_shell: bool,
    pub wrote_media_outputs: bool,
    pub deleted_files: bool,
    pub overwrote_outputs: bool,
    pub exit_code: Option<i32>,
    pub stdout_preview: String,
    pub stderr_preview: String,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolProducedFile {
    pub label: String,
    pub path: String,
    pub media_kind: String,
    pub format: String,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolExecutionReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub execution_schema: &'static str,
    pub action: String,
    pub produced_file_count: usize,
    pub next_action: String,
}

pub(crate) fn execute_dx_media_tool(
    request: DxMediaToolExecutionRequest,
) -> Result<DxMediaToolExecution, String> {
    let gate_value = decode_json_string(&request.runner_gate)?;
    let gate = locate_runner_gate(&gate_value).ok_or_else(|| {
        "DX media execution needs a zed.dx.media_tool.runner_gate.v1 object or gate receipt."
            .to_string()
    })?;
    let gate_summary = summarize_gate(gate);
    let blockers = execution_blockers(&request, &gate_summary);
    let execution_ready = blockers.is_empty();

    if !execution_ready {
        return Ok(blocked_response(request, gate_summary, blockers));
    }

    prepare_output_directories(&gate_summary.planned_outputs)?;

    let output = Command::new(&gate_summary.argument_vector[0])
        .args(&gate_summary.argument_vector[1..])
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return Ok(spawn_failed_response(
                request,
                gate_summary,
                error.to_string(),
            ));
        }
    };

    let produced_files = produced_files(&gate_summary.planned_outputs);
    let wrote_media_outputs = gate_summary.action != "inspect"
        && produced_files
            .iter()
            .any(|file| file.exists && file.size_bytes.unwrap_or_default() > 0);
    let process_success = output.status.success();
    let status = if process_success && gate_summary.action == "inspect" {
        "metadata_inspected"
    } else if process_success && wrote_media_outputs {
        "media_outputs_written"
    } else if process_success {
        "process_succeeded_outputs_missing"
    } else {
        "process_failed"
    };
    let next_action = if process_success {
        "Use the produced-file receipt paths as Agent sources or panel artifacts; do not paste binary media into model context."
    } else {
        "Inspect stderr_preview, fix the media plan or source, then rerun the plan/gate/execution flow."
    };

    Ok(DxMediaToolExecution {
        schema: DX_MEDIA_TOOL_EXECUTION_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxMediaToolExecutionRequestSummary {
            approve_execution: request.approve_execution,
            require_execution_receipt: request.require_execution_receipt,
            root_mode: request.root_mode,
        },
        gate: gate_summary,
        execution: DxMediaToolExecutionSummary {
            status: status.to_string(),
            execution_ready: true,
            permission_required: true,
            execution_approved: true,
            ran_external_process: true,
            ran_shell: false,
            wrote_media_outputs,
            deleted_files: false,
            overwrote_outputs: false,
            exit_code: output.status.code(),
            stdout_preview: preview_bytes(&output.stdout),
            stderr_preview: preview_bytes(&output.stderr),
            blockers: Vec::new(),
        },
        produced_files,
        execution_receipt: None,
        next_action: next_action.to_string(),
    })
}

fn blocked_response(
    request: DxMediaToolExecutionRequest,
    gate: DxMediaToolExecutionGateSummary,
    blockers: Vec<String>,
) -> DxMediaToolExecution {
    DxMediaToolExecution {
        schema: DX_MEDIA_TOOL_EXECUTION_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxMediaToolExecutionRequestSummary {
            approve_execution: request.approve_execution,
            require_execution_receipt: request.require_execution_receipt,
            root_mode: request.root_mode,
        },
        gate,
        execution: DxMediaToolExecutionSummary {
            status: if request.approve_execution {
                "blocked_after_approval"
            } else {
                "approval_required"
            }
            .to_string(),
            execution_ready: false,
            permission_required: true,
            execution_approved: request.approve_execution,
            ran_external_process: false,
            ran_shell: false,
            wrote_media_outputs: false,
            deleted_files: false,
            overwrote_outputs: false,
            exit_code: None,
            stdout_preview: String::new(),
            stderr_preview: String::new(),
            blockers,
        },
        produced_files: Vec::new(),
        execution_receipt: None,
        next_action: "Resolve the listed blockers, then rerun the media plan, runner gate, and execution flow."
            .to_string(),
    }
}

fn spawn_failed_response(
    request: DxMediaToolExecutionRequest,
    gate: DxMediaToolExecutionGateSummary,
    error: String,
) -> DxMediaToolExecution {
    DxMediaToolExecution {
        schema: DX_MEDIA_TOOL_EXECUTION_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxMediaToolExecutionRequestSummary {
            approve_execution: request.approve_execution,
            require_execution_receipt: request.require_execution_receipt,
            root_mode: request.root_mode,
        },
        gate,
        execution: DxMediaToolExecutionSummary {
            status: "process_spawn_failed".to_string(),
            execution_ready: true,
            permission_required: true,
            execution_approved: true,
            ran_external_process: false,
            ran_shell: false,
            wrote_media_outputs: false,
            deleted_files: false,
            overwrote_outputs: false,
            exit_code: None,
            stdout_preview: String::new(),
            stderr_preview: error,
            blockers: Vec::new(),
        },
        produced_files: Vec::new(),
        execution_receipt: None,
        next_action:
            "Install or configure the planned ffmpeg/ffprobe binary, then rerun the media plan, gate, and execution flow."
                .to_string(),
    }
}

fn execution_blockers(
    request: &DxMediaToolExecutionRequest,
    gate: &DxMediaToolExecutionGateSummary,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !request.approve_execution {
        blockers.push("Media execution has not been explicitly approved.".to_string());
    }
    if !request.require_execution_receipt {
        blockers.push("Approved media execution requires a managed execution receipt.".to_string());
    }
    if gate.schema != DX_MEDIA_TOOL_RUNNER_GATE_SCHEMA {
        blockers.push(format!(
            "Expected runner gate schema {DX_MEDIA_TOOL_RUNNER_GATE_SCHEMA}, got {}.",
            gate.schema
        ));
    }
    if !gate.runner_ready {
        blockers.push("Runner gate is not runner_ready.".to_string());
    }
    if !gate.runner_approved {
        blockers.push("Runner gate was not approved.".to_string());
    }
    if !gate.no_shell_interpolation {
        blockers.push("Runner gate must declare no_shell_interpolation=true.".to_string());
    }
    if !gate.argument_vector_safe {
        blockers.push("Runner gate argument vector is not safe.".to_string());
    }
    if !argument_vector_is_safe(&gate.argument_vector) {
        blockers.push(
            "Runner gate argument vector failed executor-side shell safety checks.".to_string(),
        );
    }
    if !gate.executable_matches_plan {
        blockers.push("Runner gate executable does not match the media plan.".to_string());
    }
    if !executable_matches_action(&gate.argument_vector, &gate.action) {
        blockers.push(
            "Runner gate executable failed executor-side action matching checks.".to_string(),
        );
    }
    if !gate.outputs_managed {
        blockers.push("Runner gate outputs are not all under the managed output root.".to_string());
    }
    match gate.managed_output_dir.as_deref() {
        Some(path)
            if Path::new(path).is_absolute() && path_components_are_safe(Path::new(path)) => {}
        Some(path) => blockers.push(format!(
            "Managed output directory must be an absolute path without parent traversal: {path}"
        )),
        None => blockers.push("Runner gate is missing a managed output directory.".to_string()),
    }
    if gate.argument_vector.is_empty() {
        blockers.push("Runner gate argument vector is empty.".to_string());
    }
    if gate.action != "inspect" && gate.planned_outputs.is_empty() {
        blockers.push("Media extraction execution needs planned outputs.".to_string());
    }
    for output in &gate.planned_outputs {
        if !output.absolute_path {
            blockers.push(format!(
                "Refusing to run because planned output is not an absolute path: {}",
                output.path
            ));
        }
        if !output.safe_path_components {
            blockers.push(format!(
                "Refusing to run because planned output contains parent traversal: {}",
                output.path
            ));
        }
        if output.existed_before_execution {
            blockers.push(format!(
                "Refusing to run because planned output already exists: {}",
                output.path
            ));
        }
        if !output.under_managed_output_dir {
            blockers.push(format!(
                "Refusing to run because planned output is outside the managed output directory: {}",
                output.path
            ));
        }
    }
    blockers
}

fn summarize_gate(gate: &Value) -> DxMediaToolExecutionGateSummary {
    DxMediaToolExecutionGateSummary {
        schema: string_field(gate, &["schema"]).unwrap_or_else(|| "unknown".to_string()),
        validation_status: string_field(gate, &["validation", "status"])
            .unwrap_or_else(|| "unknown".to_string()),
        runner_ready: bool_field(gate, &["validation", "runner_ready"]).unwrap_or(false),
        runner_approved: bool_field(gate, &["validation", "runner_approved"]).unwrap_or(false),
        action: string_field(gate, &["plan", "action"]).unwrap_or_else(|| "unknown".to_string()),
        executable_tool: string_field(gate, &["plan", "executable_tool"])
            .unwrap_or_else(|| "unknown".to_string()),
        managed_output_dir: string_field(gate, &["plan", "managed_output_dir"]),
        argument_vector: string_array(gate, &["validation", "argument_vector"]),
        planned_outputs: planned_outputs(gate, string_field(gate, &["plan", "managed_output_dir"])),
        no_shell_interpolation: bool_field(gate, &["validation", "no_shell_interpolation"])
            .unwrap_or(false),
        argument_vector_safe: bool_field(gate, &["validation", "argument_vector_safe"])
            .unwrap_or(false),
        executable_matches_plan: bool_field(gate, &["validation", "executable_matches_plan"])
            .unwrap_or(false),
        outputs_managed: bool_field(gate, &["validation", "outputs_managed"]).unwrap_or(false),
    }
}

fn planned_outputs(
    gate: &Value,
    managed_output_dir: Option<String>,
) -> Vec<DxMediaToolExecutionPlannedOutput> {
    let managed_output_dir = managed_output_dir.as_deref().map(Path::new);
    value_at(gate, &["validation", "planned_outputs"])
        .and_then(Value::as_array)
        .map(|outputs| {
            outputs
                .iter()
                .filter_map(|output| {
                    let path = string_field(output, &["path"])?;
                    let output_path = Path::new(&path);
                    let absolute_path = output_path.is_absolute();
                    let safe_path_components = path_components_are_safe(output_path);
                    Some(DxMediaToolExecutionPlannedOutput {
                        label: string_field(output, &["label"])
                            .unwrap_or_else(|| "output".to_string()),
                        path: path.clone(),
                        media_kind: string_field(output, &["media_kind"])
                            .unwrap_or_else(|| "unknown".to_string()),
                        format: string_field(output, &["format"])
                            .unwrap_or_else(|| "unknown".to_string()),
                        absolute_path,
                        safe_path_components,
                        under_managed_output_dir: managed_output_dir
                            .map(|root| {
                                absolute_path
                                    && safe_path_components
                                    && output_path.starts_with(root)
                            })
                            .unwrap_or(false),
                        existed_before_execution: output_path.exists(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn prepare_output_directories(outputs: &[DxMediaToolExecutionPlannedOutput]) -> Result<(), String> {
    for output in outputs {
        let path = Path::new(&output.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "Failed to prepare DX media output directory {}: {error}",
                    parent.display()
                )
            })?;
        }
    }
    Ok(())
}

fn produced_files(outputs: &[DxMediaToolExecutionPlannedOutput]) -> Vec<DxMediaToolProducedFile> {
    outputs
        .iter()
        .map(|output| {
            let path = PathBuf::from(&output.path);
            let metadata = fs::metadata(&path).ok();
            let exists = metadata.as_ref().is_some_and(|metadata| metadata.is_file());
            DxMediaToolProducedFile {
                label: output.label.clone(),
                path: output.path.clone(),
                media_kind: output.media_kind.clone(),
                format: output.format.clone(),
                exists,
                size_bytes: metadata.as_ref().map(|metadata| metadata.len()),
                sha256: exists.then(|| hash_file(&path)).and_then(Result::ok),
            }
        })
        .collect()
}

fn path_components_are_safe(path: &Path) -> bool {
    path.components()
        .all(|component| !matches!(component, Component::ParentDir))
}

fn argument_vector_is_safe(arguments: &[String]) -> bool {
    let Some(executable) = arguments.first() else {
        return false;
    };
    if matches!(
        executable_name(executable).as_str(),
        "cmd" | "powershell" | "pwsh" | "sh" | "bash" | "zsh"
    ) {
        return false;
    }

    arguments.iter().all(|argument| {
        !argument.contains("&&") && !argument.contains("||") && !argument.contains('\n')
    })
}

fn executable_matches_action(arguments: &[String], action: &str) -> bool {
    let Some(executable) = arguments.first() else {
        return false;
    };
    match action {
        "inspect" => executable_name(executable) == "ffprobe",
        "extract_audio" | "extract_frame" => executable_name(executable) == "ffmpeg",
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

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| {
        format!(
            "Failed to open produced media file {}: {error}",
            path.display()
        )
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            format!(
                "Failed to hash produced media file {}: {error}",
                path.display()
            )
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn locate_runner_gate(value: &Value) -> Option<&Value> {
    if value.get("schema").and_then(Value::as_str) == Some(DX_MEDIA_TOOL_RUNNER_GATE_SCHEMA) {
        return Some(value);
    }

    value.get("runner_gate").filter(|gate| {
        gate.get("schema").and_then(Value::as_str) == Some(DX_MEDIA_TOOL_RUNNER_GATE_SCHEMA)
    })
}

fn decode_json_string(value: &Value) -> Result<Value, String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if text.starts_with('{') {
            return serde_json::from_str(text).map_err(|error| {
                format!("Failed to parse stringified DX media runner gate JSON: {error}")
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

fn preview_bytes(bytes: &[u8]) -> String {
    const MAX_CHARS: usize = 4000;
    let text = String::from_utf8_lossy(bytes);
    if text.chars().count() <= MAX_CHARS {
        return text.to_string();
    }

    let mut preview = text.chars().take(MAX_CHARS).collect::<String>();
    preview.push_str("...");
    preview
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
