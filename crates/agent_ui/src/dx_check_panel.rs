use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use serde_json::Value;

const CHECK_RECEIPT_SCHEMA: &str = "dx.check.receipt.v1";
const ZED_PANEL_SCHEMA: &str = "dx.check.zed_panel.v1";
const VIEW_MODEL_SCHEMA: &str = "dx.www.check_panel_view_model.v1";
const CHECK_RECEIPT_RELATIVE_PATH: &[&str] = &[".dx", "receipts", "check", "check-latest.json"];
const DX_FALLBACK_CHECK_RECEIPT: &str = r"G:\Dx\.dx\receipts\check\check-latest.json";
const CHECK_PANEL_CACHE_TTL: Duration = Duration::from_secs(5);
const MAX_RECEIPT_BYTES: u64 = 256 * 1024;

#[derive(Clone)]
pub(crate) struct DxCheckPanelSnapshot {
    pub status: String,
    pub title: String,
    pub score_value: Option<u32>,
    pub score_max: Option<u32>,
    pub score_percent: Option<u8>,
    pub score_estimated: bool,
    pub weight_profile: String,
    pub receipt_path: PathBuf,
    pub receipt_present: bool,
    pub receipt_error: Option<String>,
    pub generated_at_unix_ms: Option<u64>,
    pub last_run_label: String,
    pub pass_count: Option<u32>,
    pub fail_count: Option<u32>,
    pub warn_count: Option<u32>,
    pub skipped_count: Option<u32>,
    pub duration_ms: Option<u64>,
    pub checked_paths: Vec<String>,
    pub skipped_expensive_checks: Vec<String>,
    pub refresh_command: String,
    pub detail_command: Option<String>,
    pub scoring_config_status: String,
    pub scoring_config_applies_to_score: bool,
    pub scoring_config_summary: String,
    pub sections: Vec<DxCheckPanelSection>,
    pub blockers: Vec<DxCheckPanelNotice>,
    pub warnings: Vec<DxCheckPanelNotice>,
    pub quick_fixes: Vec<DxCheckPanelQuickFix>,
    pub next_action: String,
    pub source_schema: String,
}

#[derive(Clone)]
pub(crate) struct DxCheckPanelSection {
    pub title: String,
    pub score: Option<u32>,
    pub max_score: Option<u32>,
    pub estimated: bool,
    pub status: String,
}

#[derive(Clone)]
pub(crate) struct DxCheckPanelNotice {
    pub code: String,
    pub message: String,
    pub next_action: Option<String>,
}

#[derive(Clone)]
pub(crate) struct DxCheckPanelQuickFix {
    pub label: String,
    pub next_action: String,
    pub risk_level: String,
    pub requires_user_approval: bool,
    pub writes_receipts: bool,
    pub command: Option<String>,
}

struct DxCheckPanelCache {
    cached_at: Instant,
    workspace_roots: Vec<String>,
    snapshot: DxCheckPanelSnapshot,
}

static CHECK_PANEL_CACHE: OnceLock<Mutex<Option<DxCheckPanelCache>>> = OnceLock::new();

pub(crate) fn dx_check_panel_snapshot(workspace_roots: &[String]) -> DxCheckPanelSnapshot {
    let normalized_roots = workspace_roots
        .iter()
        .map(|root| root.trim().to_string())
        .filter(|root| !root.is_empty())
        .collect::<Vec<_>>();

    let cache = CHECK_PANEL_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();
    if let Ok(mut cache) = cache.lock() {
        if let Some(cached) = cache.as_ref() {
            if cached.workspace_roots == normalized_roots
                && now.duration_since(cached.cached_at) <= CHECK_PANEL_CACHE_TTL
            {
                return cached.snapshot.clone();
            }
        }

        let snapshot = read_latest_check_panel(&normalized_roots);
        *cache = Some(DxCheckPanelCache {
            cached_at: now,
            workspace_roots: normalized_roots,
            snapshot: snapshot.clone(),
        });
        return snapshot;
    }

    read_latest_check_panel(&normalized_roots)
}

impl DxCheckPanelSnapshot {
    pub(crate) fn score_label(&self) -> String {
        match (self.score_value, self.score_max, self.score_percent) {
            (Some(score), Some(max_score), Some(percent)) => {
                let estimated = if self.score_estimated {
                    ", estimated"
                } else {
                    ""
                };
                format!("{score}/{max_score} ({percent}%{estimated})")
            }
            (Some(score), Some(max_score), None) => {
                let estimated = if self.score_estimated {
                    " estimated"
                } else {
                    ""
                };
                format!("{score}/{max_score}{estimated}")
            }
            _ => "No score claimed".to_string(),
        }
    }
}

fn read_latest_check_panel(workspace_roots: &[String]) -> DxCheckPanelSnapshot {
    let candidates = check_receipt_candidates(workspace_roots);
    for candidate in &candidates {
        if candidate.is_file() {
            return read_check_receipt(candidate);
        }
    }

    missing_snapshot(
        candidates
            .first()
            .cloned()
            .unwrap_or_else(|| PathBuf::from(DX_FALLBACK_CHECK_RECEIPT)),
    )
}

fn check_receipt_candidates(workspace_roots: &[String]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for root in workspace_roots {
        let mut path = PathBuf::from(root);
        for component in CHECK_RECEIPT_RELATIVE_PATH {
            path.push(*component);
        }
        push_unique_path(&mut candidates, path);
    }

    push_unique_path(&mut candidates, PathBuf::from(DX_FALLBACK_CHECK_RECEIPT));
    candidates
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn read_check_receipt(path: &Path) -> DxCheckPanelSnapshot {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return malformed_snapshot(
                path.to_path_buf(),
                format!("dx-check receipt metadata could not be read: {error}"),
            );
        }
    };

    if metadata.len() > MAX_RECEIPT_BYTES {
        return malformed_snapshot(
            path.to_path_buf(),
            format!(
                "dx-check receipt is too large for the launch rail: {} bytes",
                metadata.len()
            ),
        );
    }

    let receipt = match fs::read_to_string(path) {
        Ok(receipt) => receipt,
        Err(error) => {
            return malformed_snapshot(
                path.to_path_buf(),
                format!("dx-check receipt could not be read: {error}"),
            );
        }
    };

    let parsed = match serde_json::from_str::<Value>(&receipt) {
        Ok(parsed) => parsed,
        Err(error) => {
            return malformed_snapshot(
                path.to_path_buf(),
                format!("dx-check receipt JSON is malformed: {error}"),
            );
        }
    };

    panel_from_receipt_value(path.to_path_buf(), &parsed)
}

fn panel_from_receipt_value(path: PathBuf, receipt: &Value) -> DxCheckPanelSnapshot {
    if string_at(receipt, &["schema_version"]).as_deref() != Some(CHECK_RECEIPT_SCHEMA) {
        return malformed_snapshot(
            path,
            format!("dx-check receipt schema must be {CHECK_RECEIPT_SCHEMA}"),
        );
    }

    let view_model_fallback_warning = match receipt.get("zed") {
        Some(zed) => {
            if string_at(zed, &["schema_version"]).as_deref() == Some(ZED_PANEL_SCHEMA) {
                None
            } else {
                Some(view_model_fallback_warning(
                    "Zed-specific dx-check panel schema is missing or unsupported.",
                ))
            }
        }
        None => Some(view_model_fallback_warning(
            "Zed-specific dx-check panel payload is missing.",
        )),
    };

    if let Some(zed) = receipt.get("zed") {
        if view_model_fallback_warning.is_none() {
            return panel_from_zed_value(path, receipt, zed);
        }
    }

    if let Some(view_model) = receipt.get("view_model") {
        if string_at(view_model, &["schema_version"]).as_deref() == Some(VIEW_MODEL_SCHEMA) {
            return panel_from_view_model_value(
                path,
                receipt,
                view_model,
                view_model_fallback_warning,
            );
        }
    }

    if receipt.get("zed").is_some() {
        return malformed_snapshot(
            path,
            format!("dx-check zed panel schema must be {ZED_PANEL_SCHEMA}"),
        );
    }

    malformed_snapshot(
        path,
        "dx-check receipt is missing zed panel or DX-WWW view-model data".to_string(),
    )
}

fn panel_from_zed_value(path: PathBuf, receipt: &Value, zed: &Value) -> DxCheckPanelSnapshot {
    let scoring_config = zed
        .get("scoring_config")
        .or_else(|| receipt.get("scoring_config"));
    let scoring_config_status =
        string_from(scoring_config.and_then(|value| value.get("status"))).unwrap_or("unknown");
    let scoring_config_applies_to_score =
        bool_from(scoring_config.and_then(|value| value.get("applies_to_score"))).unwrap_or(true);
    let config_path = string_from(scoring_config.and_then(|value| value.get("config_path")));

    let next_action = first_non_empty([
        string_from(receipt.get("next_actions").and_then(|value| value.get(0))).map(str::to_string),
        string_at(zed, &["scoring_config", "next_action"]),
        string_from(zed.get("detail_command")).map(str::to_string),
        Some("Run dx check --json from the DX project root.".to_string()),
    ])
    .unwrap_or_else(|| "Run dx check --json from the DX project root.".to_string());

    let generated_at_unix_ms = u64_from(zed.get("generated_at_unix_ms"))
        .or_else(|| u64_from(receipt.get("generated_at_unix_ms")));
    let checked_paths = first_non_empty_string_array([
        string_array(zed.get("checked_paths")),
        string_array(receipt.get("checked_paths")),
    ]);
    let skipped_expensive_checks = first_non_empty_string_array([
        string_array(zed.get("skipped_expensive_checks")),
        string_array(receipt.get("skipped_expensive_checks")),
    ]);
    let pass_count =
        u32_from(zed.get("pass_count")).or_else(|| u32_from(receipt.get("pass_count")));
    let fail_count =
        u32_from(zed.get("fail_count")).or_else(|| u32_from(receipt.get("fail_count")));
    let warn_count =
        u32_from(zed.get("warn_count")).or_else(|| u32_from(receipt.get("warn_count")));
    let skipped_count =
        u32_from(zed.get("skipped_count")).or_else(|| u32_from(receipt.get("skipped_count")));
    let duration_ms =
        u64_from(zed.get("duration_ms")).or_else(|| u64_from(receipt.get("duration_ms")));

    DxCheckPanelSnapshot {
        status: string_from(zed.get("status"))
            .unwrap_or("unknown")
            .to_string(),
        title: "dx-check project health".to_string(),
        score_value: u32_from(zed.get("score_value")),
        score_max: u32_from(zed.get("score_max")),
        score_percent: u8_from(zed.get("score_percent")),
        score_estimated: bool_from(zed.get("score_estimated")).unwrap_or(false),
        weight_profile: string_from(zed.get("weight_profile"))
            .unwrap_or("dx-check.launch-default.v1")
            .to_string(),
        receipt_path: path,
        receipt_present: true,
        receipt_error: None,
        generated_at_unix_ms,
        last_run_label: last_run_label(None, generated_at_unix_ms),
        pass_count,
        fail_count,
        warn_count,
        skipped_count,
        duration_ms,
        checked_paths,
        skipped_expensive_checks,
        refresh_command: string_from(zed.get("refresh_command"))
            .unwrap_or("dx check --json")
            .to_string(),
        detail_command: string_from(zed.get("detail_command")).map(str::to_string),
        scoring_config_status: scoring_config_status.to_string(),
        scoring_config_applies_to_score,
        scoring_config_summary: scoring_config_summary(
            scoring_config_status,
            scoring_config_applies_to_score,
            config_path.as_deref(),
        ),
        sections: section_rows(zed.get("sections")),
        blockers: notice_rows(zed.get("blockers")),
        warnings: notice_rows(zed.get("warnings")),
        quick_fixes: quick_fix_rows(zed.get("quick_fixes")),
        next_action,
        source_schema: ZED_PANEL_SCHEMA.to_string(),
    }
}

fn panel_from_view_model_value(
    path: PathBuf,
    receipt: &Value,
    view_model: &Value,
    fallback_warning: Option<DxCheckPanelNotice>,
) -> DxCheckPanelSnapshot {
    let scoring_config = view_model
        .get("scoring_config")
        .or_else(|| receipt.get("scoring_config"));
    let scoring_config_status =
        string_from(scoring_config.and_then(|value| value.get("status"))).unwrap_or("unknown");
    let scoring_config_applies_to_score =
        bool_from(scoring_config.and_then(|value| value.get("applies_to_score"))).unwrap_or(true);
    let config_path = string_from(scoring_config.and_then(|value| value.get("config_path")));
    let score_meter = view_model.get("score_meter");
    let status = string_from(view_model.get("status"))
        .unwrap_or("unknown")
        .to_string();

    let next_action = first_non_empty([
        string_at(view_model, &["blocker_rows", "0", "next_action"]),
        string_at(view_model, &["warning_rows", "0", "next_action"]),
        string_at(view_model, &["quick_fix_rows", "0", "next_action"]),
        string_at(view_model, &["scoring_config", "next_action"]),
        string_from(view_model.get("empty_state")).map(str::to_string),
        string_at(view_model, &["primary_action", "command"]),
        Some("Run dx check --json from the DX project root.".to_string()),
    ])
    .unwrap_or_else(|| "Run dx check --json from the DX project root.".to_string());

    let mut warnings = notice_rows(view_model.get("warning_rows"));
    if let Some(fallback_warning) = fallback_warning {
        warnings.insert(0, fallback_warning);
    }

    let generated_at_unix_ms = u64_from(view_model.get("last_run_unix_ms"))
        .or_else(|| u64_from(receipt.get("generated_at_unix_ms")));
    let checked_paths = first_non_empty_string_array([
        string_array(view_model.get("checked_paths")),
        string_array(receipt.get("checked_paths")),
    ]);
    let skipped_expensive_checks = first_non_empty_string_array([
        string_array(view_model.get("skipped_expensive_checks")),
        string_array(receipt.get("skipped_expensive_checks")),
    ]);
    let pass_count =
        u32_from(view_model.get("pass_count")).or_else(|| u32_from(receipt.get("pass_count")));
    let fail_count =
        u32_from(view_model.get("fail_count")).or_else(|| u32_from(receipt.get("fail_count")));
    let warn_count =
        u32_from(view_model.get("warn_count")).or_else(|| u32_from(receipt.get("warn_count")));
    let skipped_count = u32_from(view_model.get("skipped_count"))
        .or_else(|| u32_from(receipt.get("skipped_count")));
    let duration_ms =
        u64_from(view_model.get("duration_ms")).or_else(|| u64_from(receipt.get("duration_ms")));

    DxCheckPanelSnapshot {
        status: status.clone(),
        title: string_from(view_model.get("title"))
            .unwrap_or("dx-check project health")
            .to_string(),
        score_value: u32_from(score_meter.and_then(|value| value.get("value"))),
        score_max: u32_from(score_meter.and_then(|value| value.get("max"))),
        score_percent: u8_from(score_meter.and_then(|value| value.get("percent"))),
        score_estimated: bool_from(score_meter.and_then(|value| value.get("estimated")))
            .unwrap_or(false),
        weight_profile: string_from(view_model.get("weight_profile"))
            .or_else(|| string_from(receipt.get("weight_profile")))
            .unwrap_or("dx-check.launch-default.v1")
            .to_string(),
        receipt_path: path,
        receipt_present: true,
        receipt_error: if status == "malformed" {
            string_from(view_model.get("empty_state")).map(str::to_string)
        } else {
            None
        },
        generated_at_unix_ms,
        last_run_label: last_run_label(
            string_from(view_model.get("last_run_label")),
            generated_at_unix_ms,
        ),
        pass_count,
        fail_count,
        warn_count,
        skipped_count,
        duration_ms,
        checked_paths,
        skipped_expensive_checks,
        refresh_command: string_at(view_model, &["primary_action", "command"])
            .unwrap_or_else(|| "dx check --json".to_string()),
        detail_command: string_at(view_model, &["secondary_action", "command"]),
        scoring_config_status: scoring_config_status.to_string(),
        scoring_config_applies_to_score,
        scoring_config_summary: scoring_config_summary(
            scoring_config_status,
            scoring_config_applies_to_score,
            config_path.as_deref(),
        ),
        sections: section_rows(view_model.get("bucket_rows")),
        blockers: notice_rows(view_model.get("blocker_rows")),
        warnings,
        quick_fixes: quick_fix_rows(view_model.get("quick_fix_rows")),
        next_action,
        source_schema: VIEW_MODEL_SCHEMA.to_string(),
    }
}

fn view_model_fallback_warning(reason: impl Into<String>) -> DxCheckPanelNotice {
    DxCheckPanelNotice {
        code: "zed-panel-fallback-view-model".to_string(),
        message: format!(
            "{} Rendering the shared DX-WWW check panel view model instead.",
            reason.into()
        ),
        next_action: Some(
            "Refresh the receipt with the current `dx check --json` command before final native proof."
                .to_string(),
        ),
    }
}

fn last_run_label(receipt_label: Option<&str>, generated_at_unix_ms: Option<u64>) -> String {
    if let Some(label) = receipt_label.filter(|label| !label.trim().is_empty()) {
        return label.to_string();
    }

    match generated_at_unix_ms {
        Some(value) => format!("Last run Unix ms: {value}"),
        None => "No dx-check run has been loaded.".to_string(),
    }
}

fn missing_snapshot(path: PathBuf) -> DxCheckPanelSnapshot {
    DxCheckPanelSnapshot {
        status: "missing".to_string(),
        title: "dx-check receipt missing".to_string(),
        score_value: None,
        score_max: Some(500),
        score_percent: None,
        score_estimated: false,
        weight_profile: "dx-check.launch-default.v1".to_string(),
        receipt_path: path,
        receipt_present: false,
        receipt_error: None,
        generated_at_unix_ms: None,
        last_run_label: last_run_label(None, None),
        pass_count: None,
        fail_count: None,
        warn_count: None,
        skipped_count: None,
        duration_ms: None,
        checked_paths: Vec::new(),
        skipped_expensive_checks: Vec::new(),
        refresh_command: "dx check --json".to_string(),
        detail_command: Some("dx check score --json".to_string()),
        scoring_config_status: "unknown".to_string(),
        scoring_config_applies_to_score: true,
        scoring_config_summary: "No scoring config loaded".to_string(),
        sections: Vec::new(),
        blockers: Vec::new(),
        warnings: Vec::new(),
        quick_fixes: Vec::new(),
        next_action: "Run dx check --json from the DX project root.".to_string(),
        source_schema: "missing".to_string(),
    }
}

fn malformed_snapshot(path: PathBuf, message: String) -> DxCheckPanelSnapshot {
    DxCheckPanelSnapshot {
        status: "malformed".to_string(),
        title: "dx-check receipt malformed".to_string(),
        score_value: None,
        score_max: Some(500),
        score_percent: None,
        score_estimated: false,
        weight_profile: "dx-check.launch-default.v1".to_string(),
        receipt_path: path,
        receipt_present: true,
        receipt_error: Some(message.clone()),
        generated_at_unix_ms: None,
        last_run_label: last_run_label(None, None),
        pass_count: None,
        fail_count: None,
        warn_count: None,
        skipped_count: None,
        duration_ms: None,
        checked_paths: Vec::new(),
        skipped_expensive_checks: Vec::new(),
        refresh_command: "dx check --json".to_string(),
        detail_command: Some("dx check score --json".to_string()),
        scoring_config_status: "unknown".to_string(),
        scoring_config_applies_to_score: true,
        scoring_config_summary: "Receipt could not be parsed".to_string(),
        sections: Vec::new(),
        blockers: vec![DxCheckPanelNotice {
            code: "receipt-malformed".to_string(),
            message,
            next_action: Some("Rerun dx check --json with the current DX CLI.".to_string()),
        }],
        warnings: Vec::new(),
        quick_fixes: Vec::new(),
        next_action: "Rerun dx check --json with the current DX CLI.".to_string(),
        source_schema: "malformed".to_string(),
    }
}

fn scoring_config_summary(
    status: &str,
    applies_to_score: bool,
    config_path: Option<&str>,
) -> String {
    match (status, applies_to_score, config_path) {
        ("detected_not_applied", false, Some(path)) => {
            format!("detected_not_applied at {path}; not applied")
        }
        ("detected_not_applied", false, None) => "detected_not_applied; not applied".to_string(),
        ("default", _, _) => "launch default weights".to_string(),
        (status, false, Some(path)) => format!("{status} at {path}; not applied"),
        (status, true, Some(path)) => format!("{status} at {path}"),
        (status, true, None) => status.to_string(),
        (status, false, None) => format!("{status}; not applied"),
    }
}

fn section_rows(value: Option<&Value>) -> Vec<DxCheckPanelSection> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(8)
        .filter_map(|section| {
            let title = string_from(section.get("title"))
                .or_else(|| string_from(section.get("label")))
                .or_else(|| string_from(section.get("id")))?;
            Some(DxCheckPanelSection {
                title: title.to_string(),
                score: u32_from(section.get("score")),
                max_score: u32_from(section.get("max_score")),
                estimated: bool_from(section.get("estimated")).unwrap_or(false),
                status: string_from(section.get("status"))
                    .unwrap_or("unknown")
                    .to_string(),
            })
        })
        .collect()
}

fn notice_rows(value: Option<&Value>) -> Vec<DxCheckPanelNotice> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(8)
        .filter_map(|notice| {
            let message = string_from(notice.get("message"))?;
            Some(DxCheckPanelNotice {
                code: string_from(notice.get("code"))
                    .unwrap_or("dx-check-notice")
                    .to_string(),
                message: message.to_string(),
                next_action: string_from(notice.get("next_action")).map(str::to_string),
            })
        })
        .collect()
}

fn quick_fix_rows(value: Option<&Value>) -> Vec<DxCheckPanelQuickFix> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(8)
        .filter_map(|fix| {
            let label = string_from(fix.get("label"))?;
            let next_action = string_from(fix.get("next_action"))?;
            let command = string_from(fix.get("command")).map(str::to_string);
            Some(DxCheckPanelQuickFix {
                label: label.to_string(),
                next_action: next_action.to_string(),
                risk_level: string_from(fix.get("risk_level"))
                    .map(str::to_string)
                    .unwrap_or_else(|| quick_fix_risk_level(command.as_deref()).to_string()),
                requires_user_approval: bool_from(fix.get("requires_user_approval"))
                    .unwrap_or_else(|| quick_fix_requires_approval(command.as_deref())),
                writes_receipts: bool_from(fix.get("writes_receipts"))
                    .unwrap_or_else(|| quick_fix_writes_receipts(command.as_deref())),
                command,
            })
        })
        .collect()
}

fn quick_fix_risk_level(command: Option<&str>) -> &'static str {
    let Some(command) = command else {
        return "manual";
    };

    if command.contains("--run ") || command.contains("--run-web") || command.contains("--run-e2e")
    {
        "executes-approved-runner"
    } else if command.contains("--https-probe") {
        "network-probe"
    } else if command.contains("--lighthouse-json")
        || command.contains("--cdp-json")
        || command.contains("--smoke-evidence")
    {
        "evidence-import"
    } else if command.starts_with("dx check") {
        "receipt-write"
    } else {
        "manual"
    }
}

fn quick_fix_requires_approval(command: Option<&str>) -> bool {
    command.is_some_and(|command| {
        command.contains("--run ")
            || command.contains("--run-web")
            || command.contains("--run-e2e")
            || command.contains("--https-probe")
    })
}

fn quick_fix_writes_receipts(command: Option<&str>) -> bool {
    command.is_some_and(|command| command == "dx check --json" || command.starts_with("dx check "))
}

fn string_at(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for segment in path {
        current = match segment.parse::<usize>() {
            Ok(index) => current.get(index)?,
            Err(_) => current.get(*segment)?,
        };
    }
    string_from(Some(current)).map(str::to_string)
}

fn string_from(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn bool_from(value: Option<&Value>) -> Option<bool> {
    value.and_then(Value::as_bool)
}

fn u8_from(value: Option<&Value>) -> Option<u8> {
    u64_from(value).and_then(|value| u8::try_from(value).ok())
}

fn u32_from(value: Option<&Value>) -> Option<u32> {
    u64_from(value).and_then(|value| u32::try_from(value).ok())
}

fn u64_from(value: Option<&Value>) -> Option<u64> {
    value.and_then(Value::as_u64)
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| {
            let value = string_from(Some(value))?.trim();
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
        .take(8)
        .collect()
}

fn first_non_empty_string_array(values: impl IntoIterator<Item = Vec<String>>) -> Vec<String> {
    values
        .into_iter()
        .find(|values| !values.is_empty())
        .unwrap_or_default()
}

fn first_non_empty(values: impl IntoIterator<Item = Option<String>>) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .find(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn panel_receipt_keeps_detected_config_out_of_score() {
        let receipt = json!({
            "schema_version": "dx.check.receipt.v1",
            "next_actions": ["Review skipped expensive checks before final launch."],
            "pass_count": 9,
            "fail_count": 0,
            "warn_count": 2,
            "skipped_count": 5,
            "duration_ms": 37,
            "checked_paths": ["G:\\Dx", "G:\\Dx\\www"],
            "skipped_expensive_checks": [
                "Lighthouse execution skipped by default.",
                "Full E2E execution skipped by default."
            ],
            "zed": {
                "schema_version": "dx.check.zed_panel.v1",
                "status": "warning",
                "score_value": 410,
                "score_max": 500,
                "score_percent": 82,
                "score_estimated": true,
                "weight_profile": "dx-check.launch-default.v1",
                "generated_at_unix_ms": 1779400000000_u64,
                "refresh_command": "dx check --json",
                "detail_command": "dx check score --json",
                "scoring_config": {
                    "status": "detected_not_applied",
                    "config_path": ".dx/check/config.json",
                    "applies_to_score": false
                },
                "sections": [
                    {
                        "title": "Structure",
                        "score": 88,
                        "max_score": 100,
                        "estimated": false,
                        "status": "ready"
                    }
                ],
                "warnings": [
                    {
                        "code": "score-config-detected-not-applied",
                        "message": "Config detected, but launch scoring still uses defaults.",
                        "next_action": "Review configured weights."
                    }
                ],
                "quick_fixes": [
                    {
                        "label": "Review scoring config",
                        "next_action": "Open .dx/check/config.json.",
                        "risk_level": "config-review",
                        "requires_user_approval": false,
                        "writes_receipts": false
                    }
                ]
            }
        });

        let snapshot = panel_from_receipt_value(PathBuf::from("check-latest.json"), &receipt);

        assert_eq!(snapshot.score_value, Some(410));
        assert_eq!(snapshot.score_max, Some(500));
        assert_eq!(snapshot.score_percent, Some(82));
        assert!(snapshot.score_estimated);
        assert_eq!(snapshot.last_run_label, "Last run Unix ms: 1779400000000");
        assert_eq!(snapshot.pass_count, Some(9));
        assert_eq!(snapshot.fail_count, Some(0));
        assert_eq!(snapshot.warn_count, Some(2));
        assert_eq!(snapshot.skipped_count, Some(5));
        assert_eq!(snapshot.duration_ms, Some(37));
        assert_eq!(snapshot.checked_paths, vec!["G:\\Dx", "G:\\Dx\\www"]);
        assert_eq!(snapshot.skipped_expensive_checks.len(), 2);
        assert_eq!(
            snapshot.skipped_expensive_checks[0],
            "Lighthouse execution skipped by default."
        );
        assert_eq!(snapshot.scoring_config_status, "detected_not_applied");
        assert!(!snapshot.scoring_config_applies_to_score);
        assert!(snapshot.scoring_config_summary.contains("not applied"));
        assert_eq!(snapshot.sections.len(), 1);
        assert_eq!(
            snapshot.warnings[0].code,
            "score-config-detected-not-applied"
        );
        assert_eq!(snapshot.quick_fixes[0].label, "Review scoring config");
        assert_eq!(snapshot.quick_fixes[0].risk_level, "config-review");
        assert!(!snapshot.quick_fixes[0].requires_user_approval);
        assert!(!snapshot.quick_fixes[0].writes_receipts);
    }

    #[test]
    fn unsupported_zed_schema_does_not_claim_score() {
        let receipt = json!({
            "schema_version": "dx.check.receipt.v1",
            "zed": {
                "schema_version": "dx.check.zed_panel.v0",
                "score_value": 500,
                "score_max": 500
            }
        });

        let snapshot = panel_from_receipt_value(PathBuf::from("check-latest.json"), &receipt);

        assert_eq!(snapshot.status, "malformed");
        assert_eq!(snapshot.score_value, None);
        assert_eq!(snapshot.score_max, Some(500));
        assert_eq!(snapshot.blockers.len(), 1);
    }

    #[test]
    fn view_model_only_receipt_can_render_without_zed_panel() {
        let receipt = json!({
            "schema_version": "dx.check.receipt.v1",
            "weight_profile": "dx-check.launch-default.v1",
            "pass_count": 9,
            "fail_count": 0,
            "warn_count": 2,
            "skipped_count": 5,
            "duration_ms": 37,
            "checked_paths": ["."],
            "skipped_expensive_checks": ["CDP/browser metrics skipped by default."],
            "view_model": {
                "schema_version": "dx.www.check_panel_view_model.v1",
                "status": "ready",
                "title": "dx-check project health",
                "score_meter": {
                    "value": 410,
                    "max": 500,
                    "percent": 82,
                    "estimated": true
                },
                "last_run_unix_ms": 1779400000000_u64,
                "last_run_label": "2 minutes ago",
                "bucket_rows": [
                    {
                        "title": "Web performance",
                        "score": 70,
                        "max_score": 100,
                        "estimated": true,
                        "status": "warning"
                    }
                ],
                "blocker_rows": [],
                "warning_rows": [
                    {
                        "code": "web-lighthouse-skipped",
                        "message": "Lighthouse did not run.",
                        "next_action": "Run an approved Lighthouse adapter later."
                    }
                ],
                "quick_fix_rows": [
                    {
                        "label": "Run web probe",
                        "next_action": "Collect bounded HTTP metadata.",
                        "command": "dx check web --url http://localhost:3000 --json"
                    }
                ],
                "primary_action": {
                    "command": "dx check --json"
                },
                "secondary_action": {
                    "command": "dx check score --json"
                },
                "scoring_config": {
                    "status": "default",
                    "applies_to_score": true
                }
            }
        });

        let snapshot = panel_from_receipt_value(PathBuf::from("check-latest.json"), &receipt);

        assert_eq!(snapshot.source_schema, "dx.www.check_panel_view_model.v1");
        assert_eq!(snapshot.status, "ready");
        assert_eq!(snapshot.score_value, Some(410));
        assert_eq!(snapshot.score_max, Some(500));
        assert_eq!(snapshot.last_run_label, "2 minutes ago");
        assert_eq!(snapshot.pass_count, Some(9));
        assert_eq!(snapshot.fail_count, Some(0));
        assert_eq!(snapshot.warn_count, Some(2));
        assert_eq!(snapshot.skipped_count, Some(5));
        assert_eq!(snapshot.duration_ms, Some(37));
        assert_eq!(snapshot.checked_paths, vec!["."]);
        assert_eq!(
            snapshot.skipped_expensive_checks,
            vec!["CDP/browser metrics skipped by default."]
        );
        assert_eq!(snapshot.sections[0].title, "Web performance");
        assert_eq!(snapshot.warnings[0].code, "zed-panel-fallback-view-model");
        assert_eq!(snapshot.warnings[1].code, "web-lighthouse-skipped");
        assert_eq!(snapshot.quick_fixes[0].label, "Run web probe");
        assert_eq!(snapshot.quick_fixes[0].risk_level, "receipt-write");
        assert!(!snapshot.quick_fixes[0].requires_user_approval);
        assert!(snapshot.quick_fixes[0].writes_receipts);
        assert_eq!(snapshot.refresh_command, "dx check --json");
        assert_eq!(
            snapshot.detail_command.as_deref(),
            Some("dx check score --json")
        );
    }
}
