use anyhow::{Context as _, Result};
use client::{Client, telemetry::MINIDUMP_ENDPOINT};
use feature_flags::FeatureFlagAppExt;
use futures::{AsyncReadExt, TryStreamExt};
use gpui::{App, AppContext as _, SerializedThreadTaskTimings, TaskExt};
use http_client::{self, AsyncBody, HttpClient, Request};
use log::info;
use project::Project;
use proto::{CrashReport, GetCrashFilesResponse};
use reqwest::{
    Method,
    multipart::{Form, Part},
};
use serde::Deserialize;
use smol::stream::StreamExt;
use std::{ffi::OsStr, fs, path::Path, sync::Arc, thread::ThreadId, time::Duration};
use sysinfo::{MemoryRefreshKind, RefreshKind, System};
use util::ResultExt;

use crate::STARTUP_TIME;

const MAX_HANG_TRACES: usize = 3;
const MAX_HANG_TRACE_SCAN_ENTRIES: usize = 256;
const MAX_HANG_TRACE_THREAD_TIMINGS: usize = 128;
const MAX_REMOTE_MINIDUMP_METADATA_BYTES: usize = 64 * 1024;

pub fn init(client: Arc<Client>, cx: &mut App) {
    if cfg!(debug_assertions) {
        log::info!("Debug assertions enabled, skipping hang monitoring");
    } else {
        monitor_hangs(cx);
    }

    cx.on_flags_ready({
        let client = client.clone();
        move |flags_ready, cx| {
            if flags_ready.is_staff {
                let client = client.clone();
                cx.background_spawn(async move {
                    upload_build_timings(client).await.warn_on_err();
                })
                .detach();
            }
        }
    })
    .detach();

    if client.telemetry().diagnostics_enabled() {
        let client = client.clone();
        cx.background_spawn(async move {
            upload_previous_minidumps(client).await.warn_on_err();
        })
        .detach()
    }

    cx.observe_new(move |project: &mut Project, _, cx| {
        let client = client.clone();

        let Some(remote_client) = project.remote_client() else {
            return;
        };
        remote_client.update(cx, |remote_client, cx| {
            if !client.telemetry().diagnostics_enabled() {
                return;
            }
            let Some(endpoint) = MINIDUMP_ENDPOINT.as_ref().cloned() else {
                log::debug!("Minidump endpoint not set; skipping remote minidump upload");
                return;
            };
            let request = remote_client
                .proto_client()
                .request(proto::GetCrashFiles {});
            cx.background_spawn(async move {
                let GetCrashFilesResponse { crashes } = request.await?;

                for CrashReport {
                    metadata,
                    minidump_contents,
                } in crashes
                {
                    if metadata.len() > MAX_REMOTE_MINIDUMP_METADATA_BYTES {
                        log::warn!(
                            "Remote minidump metadata is too large to parse: {} bytes (limit {} bytes)",
                            metadata.len(),
                            MAX_REMOTE_MINIDUMP_METADATA_BYTES
                        );
                        continue;
                    }

                    if let Some(metadata) = serde_json::from_str(&metadata).log_err() {
                        upload_minidump(client.clone(), &endpoint, minidump_contents, &metadata)
                            .await
                            .log_err();
                    }
                }

                anyhow::Ok(())
            })
            .detach_and_log_err(cx);
        })
    })
    .detach();
}

fn monitor_hangs(cx: &App) {
    let main_thread_id = std::thread::current().id();

    let foreground_executor = cx.foreground_executor();
    let background_executor = cx.background_executor();

    // 3 seconds hang
    let (mut tx, mut rx) = futures::channel::mpsc::channel(3);
    foreground_executor
        .spawn(async move { while (rx.next().await).is_some() {} })
        .detach();

    background_executor
        .spawn({
            let background_executor = background_executor.clone();
            async move {
                cleanup_old_hang_traces();

                let mut hang_time = None;

                let mut hanging = false;
                loop {
                    background_executor.timer(Duration::from_secs(1)).await;
                    match tx.try_send(()) {
                        Ok(_) => {
                            hang_time = None;
                            hanging = false;
                            continue;
                        }
                        Err(e) => {
                            let is_full = e.into_send_error().is_full();
                            if is_full && !hanging {
                                hanging = true;
                                hang_time = Some(chrono::Local::now());
                            }

                            if is_full {
                                save_hang_trace(
                                    main_thread_id,
                                    &background_executor,
                                    hang_time.unwrap(),
                                );
                            }
                        }
                    }
                }
            }
        })
        .detach();
}

fn hang_trace_files_for_pruning() -> Vec<std::fs::DirEntry> {
    let Ok(entries) = std::fs::read_dir(paths::hang_traces_dir()) else {
        return Vec::new();
    };

    entries
        .filter_map(|entry| entry.ok())
        .take(MAX_HANG_TRACE_SCAN_ENTRIES)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "json" || ext == "miniprof")
        })
        .collect()
}

fn cleanup_old_hang_traces() {
    let mut files = hang_trace_files_for_pruning();

    if files.len() > MAX_HANG_TRACES {
        files.sort_by_key(|entry| entry.file_name());
        for entry in files.iter().take(files.len() - MAX_HANG_TRACES) {
            std::fs::remove_file(entry.path()).log_err();
        }
    }
}

fn save_hang_trace(
    main_thread_id: ThreadId,
    background_executor: &gpui::BackgroundExecutor,
    hang_time: chrono::DateTime<chrono::Local>,
) {
    let thread_timings = background_executor.dispatcher().get_all_timings();
    let thread_timings = thread_timings
        .into_iter()
        .take(MAX_HANG_TRACE_THREAD_TIMINGS)
        .map(|mut timings| {
            if timings.thread_id == main_thread_id {
                timings.thread_name = Some("main".to_string());
            }

            SerializedThreadTaskTimings::convert(*STARTUP_TIME.get().unwrap(), timings)
        })
        .collect::<Vec<_>>();

    let trace_path = paths::hang_traces_dir().join(&format!(
        "hang-{}.miniprof.json",
        hang_time.format("%Y-%m-%d_%H-%M-%S")
    ));

    let Some(timings) = serde_json::to_string(&thread_timings)
        .context("hang timings serialization")
        .log_err()
    else {
        return;
    };

    let mut files = hang_trace_files_for_pruning();

    if files.len() >= MAX_HANG_TRACES {
        files.sort_by_key(|entry| entry.file_name());
        for entry in files.iter().take(files.len() - (MAX_HANG_TRACES - 1)) {
            std::fs::remove_file(entry.path()).log_err();
        }
    }

    std::fs::write(&trace_path, timings)
        .context("hang trace file writing")
        .log_err();

    info!(
        "hang detected, trace file saved at: {}",
        trace_path.display()
    );
}

const MAX_PREVIOUS_MINIDUMP_METADATA_BYTES: u64 = 64 * 1024;
const MAX_PREVIOUS_MINIDUMP_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PREVIOUS_MINIDUMP_SCAN_ENTRIES: usize = 256;

async fn read_limited_previous_minidump_file(
    path: &Path,
    max_bytes: u64,
    label: &str,
) -> Result<Option<Vec<u8>>> {
    let file = smol::fs::File::open(path).await?;
    let mut contents = Vec::new();
    let mut limited_file = file.take(max_bytes + 1);
    limited_file.read_to_end(&mut contents).await?;

    if contents.len() as u64 > max_bytes {
        log::warn!(
            "Previous minidump {label} file {:?} is too large for previous minidump upload: {} bytes read (limit {} bytes)",
            path,
            contents.len(),
            max_bytes
        );
        return Ok(None);
    }

    Ok(Some(contents))
}

async fn read_previous_minidump_metadata(path: &Path) -> Result<Option<crashes::CrashInfo>> {
    let Some(data) =
        read_limited_previous_minidump_file(path, MAX_PREVIOUS_MINIDUMP_METADATA_BYTES, "metadata")
            .await?
    else {
        return Ok(None);
    };

    Ok(serde_json::from_slice(&data).ok())
}

async fn read_previous_minidump_payload(path: &Path) -> Result<Option<Vec<u8>>> {
    read_limited_previous_minidump_file(path, MAX_PREVIOUS_MINIDUMP_BYTES, "payload").await
}

pub async fn upload_previous_minidumps(client: Arc<Client>) -> anyhow::Result<()> {
    let Some(minidump_endpoint) = MINIDUMP_ENDPOINT.as_ref() else {
        log::debug!("Minidump endpoint not set; skipping previous minidump upload");
        return Ok(());
    };

    let mut children = smol::fs::read_dir(paths::logs_dir()).await?;
    let mut scanned_entries = 0;
    loop {
        if scanned_entries >= MAX_PREVIOUS_MINIDUMP_SCAN_ENTRIES {
            log::debug!(
                "Previous minidump scan reached {MAX_PREVIOUS_MINIDUMP_SCAN_ENTRIES} entries; skipping remaining files for this upload pass"
            );
            break;
        }
        let Some(child) = children.next().await else {
            break;
        };
        scanned_entries += 1;
        let child = child?;
        let child_path = child.path();
        if child_path.extension() != Some(OsStr::new("dmp")) {
            continue;
        }
        let mut json_path = child_path.clone();
        json_path.set_extension("json");
        let Ok(Some(metadata)) = read_previous_minidump_metadata(&json_path).await else {
            continue;
        };
        let minidump = match read_previous_minidump_payload(&child_path)
            .await
            .context("Failed to read minidump")?
        {
            Some(minidump) => minidump,
            None => continue,
        };
        if upload_minidump(client.clone(), minidump_endpoint, minidump, &metadata)
            .await
            .log_err()
            .is_some()
        {
            fs::remove_file(child_path).ok();
            fs::remove_file(json_path).ok();
        }
    }
    Ok(())
}

fn has_missing_minidump_commit_sha(commit_sha: &str) -> bool {
    matches!(commit_sha, "no sha" | "no_sha")
}

fn log_missing_minidump_commit_sha(metadata: &crashes::CrashInfo) {
    if metadata.init.release_channel.eq_ignore_ascii_case("dev") {
        log::debug!("No commit sha set; skipping dev minidump upload");
    } else {
        log::warn!("No commit sha set, skipping minidump upload");
    }
}

const MAX_MINIDUMP_UPLOAD_BODY_BYTES: u64 = 65 * 1024 * 1024;

async fn read_limited_minidump_upload_body(form: Form) -> Result<Vec<u8>> {
    let mut body_bytes = Vec::new();
    let mut limited_stream = form
        .into_stream()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        .into_async_read()
        .take(MAX_MINIDUMP_UPLOAD_BODY_BYTES + 1);
    limited_stream.read_to_end(&mut body_bytes).await?;

    if body_bytes.len() as u64 > MAX_MINIDUMP_UPLOAD_BODY_BYTES {
        anyhow::bail!(
            "minidump upload request body exceeded {MAX_MINIDUMP_UPLOAD_BODY_BYTES} bytes: {} bytes read",
            body_bytes.len()
        );
    }

    Ok(body_bytes)
}

const MAX_MINIDUMP_UPLOAD_RESPONSE_BYTES: u64 = 64 * 1024;
const MAX_MINIDUMP_UPLOAD_RESPONSE_DISPLAY_CHARS: usize = 1024;

struct LimitedMinidumpUploadResponse {
    text: String,
    truncated: bool,
}

async fn read_limited_minidump_upload_response(
    response: &mut http_client::Response<AsyncBody>,
) -> Result<LimitedMinidumpUploadResponse> {
    let mut response_body = Vec::new();
    let mut limited_response_body = response
        .body_mut()
        .take(MAX_MINIDUMP_UPLOAD_RESPONSE_BYTES + 1);
    limited_response_body
        .read_to_end(&mut response_body)
        .await?;

    let truncated = response_body.len() as u64 > MAX_MINIDUMP_UPLOAD_RESPONSE_BYTES;
    if truncated {
        response_body.truncate(MAX_MINIDUMP_UPLOAD_RESPONSE_BYTES as usize);
    }

    Ok(LimitedMinidumpUploadResponse {
        text: String::from_utf8_lossy(&response_body).into_owned(),
        truncated,
    })
}

fn compact_minidump_upload_response_text(response: &LimitedMinidumpUploadResponse) -> String {
    let mut compact_response = response
        .text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let suffix = if response.truncated {
        format!("... [response body exceeded {MAX_MINIDUMP_UPLOAD_RESPONSE_BYTES} bytes]")
    } else {
        "...".to_string()
    };
    let suffix_chars = suffix.chars().count();

    if compact_response.is_empty() {
        compact_response = "<empty response>".to_string();
    }

    if compact_response.chars().count() > MAX_MINIDUMP_UPLOAD_RESPONSE_DISPLAY_CHARS
        || response.truncated
    {
        let max_text_chars =
            MAX_MINIDUMP_UPLOAD_RESPONSE_DISPLAY_CHARS.saturating_sub(suffix_chars);
        compact_response = compact_response.chars().take(max_text_chars).collect();
        compact_response.push_str(&suffix);
    }

    compact_response
}

async fn upload_minidump(
    client: Arc<Client>,
    endpoint: &str,
    minidump: Vec<u8>,
    metadata: &crashes::CrashInfo,
) -> Result<()> {
    if has_missing_minidump_commit_sha(&metadata.init.commit_sha) {
        log_missing_minidump_commit_sha(metadata);
        return Ok(());
    }
    let mut form = Form::new()
        .part(
            "upload_file_minidump",
            Part::bytes(minidump)
                .file_name("minidump.dmp")
                .mime_str("application/octet-stream")?,
        )
        .text(
            "sentry[tags][channel]",
            metadata.init.release_channel.clone(),
        )
        .text("sentry[tags][version]", metadata.init.zed_version.clone())
        .text("sentry[tags][binary]", metadata.init.binary.clone())
        .text("sentry[release]", metadata.init.commit_sha.clone())
        .text("platform", "rust");
    let mut panic_message = "".to_owned();
    if let Some(panic_info) = metadata.panic.as_ref() {
        panic_message = panic_info.message.clone();
        form = form
            .text("sentry[logentry][formatted]", panic_info.message.clone())
            .text("span", panic_info.span.clone());
    }
    if let Some(minidump_error) = metadata.minidump_error.clone() {
        form = form.text("minidump_error", minidump_error);
    }

    if let Some(is_staff) = &metadata
        .user_info
        .as_ref()
        .and_then(|user_info| user_info.is_staff)
    {
        form = form.text(
            "sentry[user][is_staff]",
            if *is_staff { "true" } else { "false" },
        );
    }

    if let Some(metrics_id) = metadata
        .user_info
        .as_ref()
        .and_then(|user_info| user_info.metrics_id.as_ref())
    {
        form = form.text("sentry[user][id]", metrics_id.clone());
    } else if let Some(id) = client.telemetry().installation_id() {
        form = form.text("sentry[user][id]", format!("installation-{}", id))
    }

    ::telemetry::event!(
        "Minidump Uploaded",
        panic_message = panic_message,
        crashed_version = metadata.init.zed_version.clone(),
        commit_sha = metadata.init.commit_sha.clone(),
    );

    let gpu_count = metadata.gpus.len();
    for (index, gpu) in metadata.gpus.iter().cloned().enumerate() {
        let system_specs::GpuInfo {
            device_name,
            device_pci_id,
            vendor_name,
            vendor_pci_id,
            driver_version,
            driver_name,
        } = gpu;
        let num = if gpu_count == 1 && metadata.active_gpu.is_none() {
            String::new()
        } else {
            index.to_string()
        };
        let name = format!("gpu{num}");
        let root = format!("sentry[contexts][{name}]");
        form = form
            .text(
                format!("{root}[Description]"),
                "A GPU found on the users system. May or may not be the GPU Zed is running on",
            )
            .text(format!("{root}[type]"), "gpu")
            .text(format!("{root}[name]"), device_name.unwrap_or(name))
            .text(format!("{root}[id]"), format!("{:#06x}", device_pci_id))
            .text(
                format!("{root}[vendor_id]"),
                format!("{:#06x}", vendor_pci_id),
            )
            .text_if_some(format!("{root}[vendor_name]"), vendor_name)
            .text_if_some(format!("{root}[driver_version]"), driver_version)
            .text_if_some(format!("{root}[driver_name]"), driver_name);
    }
    if let Some(active_gpu) = metadata.active_gpu.clone() {
        form = form
            .text(
                "sentry[contexts][Active_GPU][Description]",
                "The GPU Zed is running on",
            )
            .text("sentry[contexts][Active_GPU][type]", "gpu")
            .text("sentry[contexts][Active_GPU][name]", active_gpu.device_name)
            .text(
                "sentry[contexts][Active_GPU][driver_version]",
                active_gpu.driver_info,
            )
            .text(
                "sentry[contexts][Active_GPU][driver_name]",
                active_gpu.driver_name,
            )
            .text(
                "sentry[contexts][Active_GPU][is_software_emulated]",
                active_gpu.is_software_emulated.to_string(),
            );
    }

    // TODO: feature-flag-context, and more of device-context like screen resolution, available ram, device model, etc

    let content_type = format!("multipart/form-data; boundary={}", form.boundary());
    let body_bytes = read_limited_minidump_upload_body(form).await?;
    let req = Request::builder()
        .method(Method::POST)
        .uri(endpoint)
        .header("Content-Type", content_type)
        .body(AsyncBody::from(body_bytes))?;
    let mut response = client.http_client().send(req).await?;
    let response_text = read_limited_minidump_upload_response(&mut response).await?;
    let response_text = compact_minidump_upload_response_text(&response_text);
    if !response.status().is_success() {
        anyhow::bail!("failed to upload minidump: {response_text}");
    }
    log::info!("Uploaded minidump. event id: {response_text}");
    Ok(())
}

#[derive(Debug, Deserialize)]
struct BuildTiming {
    started_at: chrono::DateTime<chrono::Utc>,
    duration_ms: f32,
    first_crate: String,
    target: String,
    blocked_ms: f32,
    command: String,
}

const MAX_BUILD_TIMING_JSON_BYTES: u64 = 64 * 1024;
const MAX_BUILD_TIMING_SCAN_ENTRIES: usize = 128;

async fn read_build_timing_json(path: &Path) -> Result<Option<String>> {
    let file = smol::fs::File::open(path).await?;
    let mut contents = Vec::new();
    let mut limited_file = file.take(MAX_BUILD_TIMING_JSON_BYTES + 1);
    limited_file.read_to_end(&mut contents).await?;

    if contents.len() as u64 > MAX_BUILD_TIMING_JSON_BYTES {
        log::warn!(
            "Build timing file {:?} is too large to parse: {} bytes read (limit {} bytes)",
            path,
            contents.len(),
            MAX_BUILD_TIMING_JSON_BYTES
        );
        return Ok(None);
    }

    Ok(Some(String::from_utf8(contents)?))
}

// NOTE: this is a bit of a hack. We want to be able to have internal
// metrics around build times, but we don't have an easy way to authenticate
// users - except - we know internal users use Zed.
// So, we have it upload the timings on their behalf, it'd be better to do
// this more directly in ./script/cargo-timing-info.js.
async fn upload_build_timings(_client: Arc<Client>) -> Result<()> {
    let build_timings_dir = paths::data_dir().join("build_timings");

    if !build_timings_dir.exists() {
        return Ok(());
    }

    let cpu_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let system = System::new_with_specifics(
        RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
    );
    let ram_size_gb = (system.total_memory() as f64) / (1024.0 * 1024.0 * 1024.0);

    let mut entries = smol::fs::read_dir(&build_timings_dir).await?;
    let mut scanned_entries = 0;
    loop {
        if scanned_entries >= MAX_BUILD_TIMING_SCAN_ENTRIES {
            log::debug!(
                "Build timing scan reached {MAX_BUILD_TIMING_SCAN_ENTRIES} entries; skipping remaining files for this upload pass"
            );
            break;
        }
        let Some(entry) = entries.next().await else {
            break;
        };
        scanned_entries += 1;
        let entry = entry?;
        let path = entry.path();

        if path.extension() != Some(OsStr::new("json")) {
            continue;
        }

        let contents = match read_build_timing_json(&path).await {
            Ok(Some(contents)) => contents,
            Ok(None) => continue,
            Err(err) => {
                log::warn!("Failed to read build timing file {:?}: {}", path, err);
                continue;
            }
        };

        let timing: BuildTiming = match serde_json::from_str(&contents) {
            Ok(timing) => timing,
            Err(err) => {
                log::warn!("Failed to parse build timing file {:?}: {}", path, err);
                continue;
            }
        };

        telemetry::event!(
            "Build Timing: Cargo Build",
            started_at = timing.started_at.to_rfc3339(),
            duration_ms = timing.duration_ms,
            first_crate = timing.first_crate,
            target = timing.target,
            blocked_ms = timing.blocked_ms,
            command = timing.command,
            cpu_count = cpu_count,
            ram_size_gb = ram_size_gb
        );

        if let Err(err) = smol::fs::remove_file(&path).await {
            log::warn!("Failed to delete build timing file {:?}: {}", path, err);
        }
    }

    Ok(())
}

trait FormExt {
    fn text_if_some(
        self,
        label: impl Into<std::borrow::Cow<'static, str>>,
        value: Option<impl Into<std::borrow::Cow<'static, str>>>,
    ) -> Self;
}

impl FormExt for Form {
    fn text_if_some(
        self,
        label: impl Into<std::borrow::Cow<'static, str>>,
        value: Option<impl Into<std::borrow::Cow<'static, str>>>,
    ) -> Self {
        match value {
            Some(value) => self.text(label.into(), value.into()),
            None => self,
        }
    }
}
