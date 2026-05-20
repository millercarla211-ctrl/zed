use serde::Serialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const DX_FFMPEG_PATH_ENV: &str = "DX_FFMPEG_PATH";
const DX_FFPROBE_PATH_ENV: &str = "DX_FFPROBE_PATH";

pub(crate) const DX_MEDIA_TOOL_PLAN_SCHEMA: &str = "zed.dx.media_tool.plan.v1";
pub(crate) const DX_MEDIA_TOOL_PLAN_RECEIPT_SCHEMA: &str = "zed.dx.media_tool.plan_receipt.v1";

#[derive(Clone, Debug)]
pub(crate) struct DxMediaToolPlanRequest {
    pub media_source: String,
    pub action: Option<String>,
    pub output_format: Option<String>,
    pub start_time: Option<String>,
    pub duration: Option<String>,
    pub frame_timestamp: Option<String>,
    pub approve_media_execution: bool,
    pub workspace_root: Option<PathBuf>,
    pub managed_output_root: PathBuf,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolPlan {
    pub schema: &'static str,
    pub generated_at_ms: u64,
    pub request: DxMediaToolPlanRequestSummary,
    pub source: DxMediaToolSourceSummary,
    pub tools: DxMediaToolBinarySummary,
    pub safety: DxMediaToolSafety,
    pub action_plan: DxMediaToolActionPlan,
    pub plan_receipt: Option<DxMediaToolPlanReceipt>,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolPlanRequestSummary {
    pub action: String,
    pub output_format: Option<String>,
    pub start_time: Option<String>,
    pub duration: Option<String>,
    pub frame_timestamp: Option<String>,
    pub approve_media_execution: bool,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolSourceSummary {
    pub original: String,
    pub resolved_path: Option<String>,
    pub source_kind: String,
    pub media_kind: String,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub extension: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolBinarySummary {
    pub ffmpeg_command: String,
    pub ffprobe_command: String,
    pub configured_ffmpeg_exists: Option<bool>,
    pub configured_ffprobe_exists: Option<bool>,
    pub path_lookup_performed: bool,
    pub availability_confirmed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolSafety {
    pub permission_required: bool,
    pub media_execution_approved: bool,
    pub dry_run_only: bool,
    pub tool_ran_external_process: bool,
    pub tool_deleted_files: bool,
    pub tool_overwrites_outputs: bool,
    pub output_under_managed_root: bool,
    pub no_shell_string: bool,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolActionPlan {
    pub action: String,
    pub status: String,
    pub executable_tool: String,
    pub argument_vector: Vec<String>,
    pub managed_output_dir: String,
    pub planned_outputs: Vec<DxMediaToolOutput>,
    pub model_handoff: String,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolOutput {
    pub label: String,
    pub path: String,
    pub media_kind: String,
    pub format: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxMediaToolPlanReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub plan_schema: &'static str,
    pub action: String,
    pub planned_output_count: usize,
    pub next_action: String,
}

pub(crate) fn build_dx_media_tool_plan(
    request: DxMediaToolPlanRequest,
) -> Result<DxMediaToolPlan, String> {
    let action = normalize_action(request.action)?;
    let output_format = normalize_output_format(&action, request.output_format)?;
    let original_source = compact_text(&request.media_source);
    if original_source.is_empty() {
        return Err("DX media tool plan needs a media source path or URL.".to_string());
    }

    let source = summarize_source(&original_source, request.workspace_root.as_deref());
    let tools = binary_summary();
    let output_dir = request.managed_output_root.join("outputs");
    let planned_outputs = planned_outputs(
        &action,
        &output_format,
        &output_dir,
        &source,
        request.frame_timestamp.as_deref(),
    );
    let mut blockers = action_blockers(&action, &source);
    if !request.approve_media_execution {
        blockers.push("Media execution has not been approved for this plan.".to_string());
    }

    let action_status = if blockers.is_empty() {
        "approved_plan_ready"
    } else if request.approve_media_execution {
        "blocked_after_approval"
    } else {
        "approval_required"
    };
    let argument_vector = argument_vector(
        &action,
        &source,
        &tools,
        &planned_outputs,
        &output_format,
        request.start_time.as_deref(),
        request.duration.as_deref(),
        request.frame_timestamp.as_deref(),
    );
    let next_action = if blockers.is_empty() {
        "Review this plan receipt, then wire the future media runner to execute the argument vector with no shell interpolation and managed output paths."
            .to_string()
    } else if request.approve_media_execution {
        "Resolve the listed source/action blockers before enabling the media runner for this plan."
            .to_string()
    } else {
        "Review this dry-run media plan, then rerun with approve_media_execution=true when ready to authorize the future runner."
            .to_string()
    };

    Ok(DxMediaToolPlan {
        schema: DX_MEDIA_TOOL_PLAN_SCHEMA,
        generated_at_ms: current_unix_ms(),
        request: DxMediaToolPlanRequestSummary {
            action: action.clone(),
            output_format,
            start_time: clean_optional_text(request.start_time, 64),
            duration: clean_optional_text(request.duration, 64),
            frame_timestamp: clean_optional_text(request.frame_timestamp, 64),
            approve_media_execution: request.approve_media_execution,
            root_mode: request.root_mode,
        },
        source,
        tools,
        safety: DxMediaToolSafety {
            permission_required: true,
            media_execution_approved: request.approve_media_execution,
            dry_run_only: true,
            tool_ran_external_process: false,
            tool_deleted_files: false,
            tool_overwrites_outputs: false,
            output_under_managed_root: true,
            no_shell_string: true,
            blockers: blockers.clone(),
        },
        action_plan: DxMediaToolActionPlan {
            action,
            status: action_status.to_string(),
            executable_tool: if action == "inspect" {
                "ffprobe".to_string()
            } else {
                "ffmpeg".to_string()
            },
            argument_vector,
            managed_output_dir: output_dir.display().to_string(),
            planned_outputs,
            model_handoff: "Return the receipt path and planned output paths to the Agent. Future execution should attach produced media as sources instead of pasting binary data into context."
                .to_string(),
            blockers,
        },
        plan_receipt: None,
        next_action,
    })
}

fn normalize_action(action: Option<String>) -> Result<String, String> {
    let action = action
        .unwrap_or_else(|| "inspect".to_string())
        .trim()
        .to_ascii_lowercase()
        .replace('-', "_");

    match action.as_str() {
        "" | "inspect" | "probe" | "metadata" => Ok("inspect".to_string()),
        "extract_audio" | "audio" => Ok("extract_audio".to_string()),
        "extract_frame" | "frame" | "thumbnail" => Ok("extract_frame".to_string()),
        _ => Err(format!(
            "Unsupported DX media action `{action}`. Use inspect, extract_audio, or extract_frame."
        )),
    }
}

fn normalize_output_format(
    action: &str,
    output_format: Option<String>,
) -> Result<Option<String>, String> {
    let requested = output_format
        .map(|format| format.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|format| !format.is_empty());

    match action {
        "inspect" => Ok(None),
        "extract_audio" => {
            let format = requested.unwrap_or_else(|| "wav".to_string());
            match format.as_str() {
                "wav" | "mp3" | "m4a" | "flac" | "ogg" => Ok(Some(format)),
                other => Err(format!(
                    "Unsupported audio extraction format `{other}`. Use wav, mp3, m4a, flac, or ogg."
                )),
            }
        }
        "extract_frame" => {
            let format = requested.unwrap_or_else(|| "png".to_string());
            match format.as_str() {
                "png" | "jpg" | "jpeg" | "webp" => Ok(Some(format)),
                other => Err(format!(
                    "Unsupported frame extraction format `{other}`. Use png, jpg, jpeg, or webp."
                )),
            }
        }
        _ => Ok(requested),
    }
}

fn summarize_source(source: &str, workspace_root: Option<&Path>) -> DxMediaToolSourceSummary {
    if is_remote_url(source) {
        return DxMediaToolSourceSummary {
            original: source.to_string(),
            resolved_path: None,
            source_kind: "remote_url".to_string(),
            media_kind: "unknown".to_string(),
            exists: false,
            size_bytes: None,
            extension: extension_from_text(source),
        };
    }

    let candidate = PathBuf::from(source);
    let resolved = if candidate.is_absolute() {
        candidate
    } else if let Some(root) = workspace_root {
        root.join(candidate)
    } else {
        candidate
    };
    let exists = resolved.is_file();
    let extension = resolved
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());
    let media_kind = media_kind_for_extension(extension.as_deref());
    let size_bytes = fs::metadata(&resolved).ok().map(|metadata| metadata.len());

    DxMediaToolSourceSummary {
        original: source.to_string(),
        resolved_path: Some(resolved.display().to_string()),
        source_kind: if resolved.is_absolute() {
            "local_absolute_path".to_string()
        } else {
            "local_relative_path".to_string()
        },
        media_kind,
        exists,
        size_bytes,
        extension,
    }
}

fn binary_summary() -> DxMediaToolBinarySummary {
    let ffmpeg = env::var(DX_FFMPEG_PATH_ENV).unwrap_or_else(|_| "ffmpeg".to_string());
    let ffprobe = env::var(DX_FFPROBE_PATH_ENV).unwrap_or_else(|_| "ffprobe".to_string());

    DxMediaToolBinarySummary {
        configured_ffmpeg_exists: configured_binary_exists(&ffmpeg),
        configured_ffprobe_exists: configured_binary_exists(&ffprobe),
        ffmpeg_command: ffmpeg,
        ffprobe_command: ffprobe,
        path_lookup_performed: false,
        availability_confirmed: false,
    }
}

fn configured_binary_exists(command: &str) -> Option<bool> {
    let path = Path::new(command);
    path.is_absolute().then(|| path.is_file())
}

fn planned_outputs(
    action: &str,
    output_format: &Option<String>,
    output_dir: &Path,
    source: &DxMediaToolSourceSummary,
    frame_timestamp: Option<&str>,
) -> Vec<DxMediaToolOutput> {
    let Some(format) = output_format else {
        return Vec::new();
    };

    let stem = output_stem(source, frame_timestamp);
    let media_kind = match action {
        "extract_audio" => "audio",
        "extract_frame" => "image",
        _ => "metadata",
    };
    let label = match action {
        "extract_audio" => "extracted_audio",
        "extract_frame" => "extracted_frame",
        _ => "output",
    };

    vec![DxMediaToolOutput {
        label: label.to_string(),
        path: output_dir
            .join(format!("{stem}.{format}"))
            .display()
            .to_string(),
        media_kind: media_kind.to_string(),
        format: format.clone(),
    }]
}

fn argument_vector(
    action: &str,
    source: &DxMediaToolSourceSummary,
    tools: &DxMediaToolBinarySummary,
    planned_outputs: &[DxMediaToolOutput],
    output_format: &Option<String>,
    start_time: Option<&str>,
    duration: Option<&str>,
    frame_timestamp: Option<&str>,
) -> Vec<String> {
    let input = source
        .resolved_path
        .clone()
        .unwrap_or_else(|| source.original.clone());

    match action {
        "inspect" => vec![
            tools.ffprobe_command.clone(),
            "-v".to_string(),
            "error".to_string(),
            "-show_format".to_string(),
            "-show_streams".to_string(),
            "-print_format".to_string(),
            "json".to_string(),
            input,
        ],
        "extract_audio" => {
            let mut args = vec![tools.ffmpeg_command.clone(), "-n".to_string()];
            push_optional_time_args(&mut args, start_time, duration);
            args.extend([
                "-i".to_string(),
                input,
                "-vn".to_string(),
                "-map".to_string(),
                "0:a:0".to_string(),
            ]);
            if output_format.as_deref() == Some("wav") {
                args.extend(["-acodec".to_string(), "pcm_s16le".to_string()]);
            }
            if let Some(output) = planned_outputs.first() {
                args.push(output.path.clone());
            }
            args
        }
        "extract_frame" => {
            let mut args = vec![tools.ffmpeg_command.clone(), "-n".to_string()];
            if source.media_kind == "image" {
                args.extend([
                    "-i".to_string(),
                    input,
                    "-frames:v".to_string(),
                    "1".to_string(),
                ]);
            } else {
                let timestamp = frame_timestamp.or(start_time).unwrap_or("00:00:01");
                args.extend([
                    "-ss".to_string(),
                    timestamp.to_string(),
                    "-i".to_string(),
                    input,
                    "-frames:v".to_string(),
                    "1".to_string(),
                ]);
            }
            if let Some(output) = planned_outputs.first() {
                args.push(output.path.clone());
            }
            args
        }
        _ => Vec::new(),
    }
}

fn push_optional_time_args(
    args: &mut Vec<String>,
    start_time: Option<&str>,
    duration: Option<&str>,
) {
    if let Some(start_time) = clean_optional_str(start_time, 64) {
        args.extend(["-ss".to_string(), start_time]);
    }
    if let Some(duration) = clean_optional_str(duration, 64) {
        args.extend(["-t".to_string(), duration]);
    }
}

fn action_blockers(action: &str, source: &DxMediaToolSourceSummary) -> Vec<String> {
    let mut blockers = Vec::new();
    if source.source_kind == "remote_url" {
        blockers.push(
            "Remote URL media execution needs a separate download/source receipt first."
                .to_string(),
        );
    } else if !source.exists {
        blockers.push("Local media source does not exist or is not a file.".to_string());
    }

    match action {
        "extract_audio" if source.media_kind != "video" && source.media_kind != "audio" => {
            blockers.push("Audio extraction needs a video or audio source.".to_string());
        }
        "extract_frame" if source.media_kind != "video" && source.media_kind != "image" => {
            blockers.push("Frame extraction needs a video or image source.".to_string());
        }
        _ => {}
    }

    blockers
}

fn output_stem(source: &DxMediaToolSourceSummary, frame_timestamp: Option<&str>) -> String {
    let source_stem = source
        .resolved_path
        .as_deref()
        .and_then(|path| Path::new(path).file_stem())
        .and_then(|stem| stem.to_str())
        .or_else(|| {
            source
                .original
                .rsplit('/')
                .next()
                .and_then(|name| name.split('.').next())
        })
        .unwrap_or("media");
    let time_suffix = frame_timestamp
        .map(|timestamp| sanitize_file_component(timestamp, 24))
        .filter(|timestamp| !timestamp.is_empty());
    let mut stem = sanitize_file_component(source_stem, 80);
    if stem.is_empty() {
        stem.push_str("media");
    }
    if let Some(time_suffix) = time_suffix {
        stem.push('-');
        stem.push_str(&time_suffix);
    }
    stem
}

fn media_kind_for_extension(extension: Option<&str>) -> String {
    match extension.unwrap_or_default() {
        "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" => "video",
        "mp3" | "wav" | "m4a" | "flac" | "ogg" | "aac" => "audio",
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tiff" | "avif" => "image",
        _ => "unknown",
    }
    .to_string()
}

fn extension_from_text(text: &str) -> Option<String> {
    text.rsplit(['/', '\\', '?', '#'])
        .next()
        .and_then(|segment| segment.rsplit_once('.').map(|(_, extension)| extension))
        .map(|extension| {
            extension
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|extension| !extension.is_empty())
}

fn is_remote_url(source: &str) -> bool {
    source.starts_with("https://") || source.starts_with("http://")
}

fn clean_optional_text(value: Option<String>, max_chars: usize) -> Option<String> {
    value
        .and_then(|value| clean_optional_str(Some(&value), max_chars))
        .filter(|value| !value.is_empty())
}

fn clean_optional_str(value: Option<&str>, max_chars: usize) -> Option<String> {
    value.map(|value| truncate_for_char_budget(&compact_text(value), max_chars))
}

fn compact_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn sanitize_file_component(text: &str, max_chars: usize) -> String {
    let mut sanitized = String::new();
    for character in text.chars().take(max_chars) {
        if character.is_ascii_alphanumeric() {
            sanitized.push(character.to_ascii_lowercase());
        } else if matches!(character, '-' | '_' | '.') {
            sanitized.push(character);
        } else if character.is_whitespace() || matches!(character, ':' | '/' | '\\') {
            sanitized.push('-');
        }
    }
    sanitized.trim_matches(['-', '.', '_']).to_string()
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
