use std::path::PathBuf;

use serde_json::Value;

use super::{
    CHECK_RECEIPT_SCHEMA, DxCheckPanelNotice, DxCheckPanelQuickFix, DxCheckPanelSection,
    DxCheckPanelSnapshot, VIEW_MODEL_SCHEMA, ZED_PANEL_SCHEMA,
};

const MAX_PANEL_TEXT_CHARS: usize = 320;

pub(super) fn panel_from_receipt_value(path: PathBuf, receipt: &Value) -> DxCheckPanelSnapshot {
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
        bounded_string_from(scoring_config.and_then(|value| value.get("status")))
            .unwrap_or_else(|| "unknown".to_string());
    let scoring_config_applies_to_score =
        bool_from(scoring_config.and_then(|value| value.get("applies_to_score"))).unwrap_or(true);
    let config_path =
        bounded_string_from(scoring_config.and_then(|value| value.get("config_path")));

    let next_action = first_non_empty([
        bounded_string_from(receipt.get("next_actions").and_then(|value| value.get(0))),
        bounded_string_at(zed, ["scoring_config", "next_action"]),
        bounded_string_from(zed.get("detail_command")),
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
        status: bounded_string_from(zed.get("status")).unwrap_or_else(|| "unknown".to_string()),
        title: "dx-check project health".to_string(),
        score_value: u32_from(zed.get("score_value")),
        score_max: u32_from(zed.get("score_max")),
        score_percent: u8_from(zed.get("score_percent")),
        score_estimated: bool_from(zed.get("score_estimated")).unwrap_or(false),
        weight_profile: bounded_string_from(zed.get("weight_profile"))
            .unwrap_or_else(|| "dx-check.launch-default.v1".to_string()),
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
        refresh_command: bounded_string_from(zed.get("refresh_command"))
            .unwrap_or_else(|| "dx check --json".to_string()),
        detail_command: bounded_string_from(zed.get("detail_command")),
        scoring_config_status: scoring_config_status.clone(),
        scoring_config_applies_to_score,
        scoring_config_summary: scoring_config_summary(
            &scoring_config_status,
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
        bounded_string_from(scoring_config.and_then(|value| value.get("status")))
            .unwrap_or_else(|| "unknown".to_string());
    let scoring_config_applies_to_score =
        bool_from(scoring_config.and_then(|value| value.get("applies_to_score"))).unwrap_or(true);
    let config_path =
        bounded_string_from(scoring_config.and_then(|value| value.get("config_path")));
    let score_meter = view_model.get("score_meter");
    let status =
        bounded_string_from(view_model.get("status")).unwrap_or_else(|| "unknown".to_string());

    let next_action = first_non_empty([
        bounded_string_at(view_model, ["blocker_rows", "0", "next_action"]),
        bounded_string_at(view_model, ["warning_rows", "0", "next_action"]),
        bounded_string_at(view_model, ["quick_fix_rows", "0", "next_action"]),
        bounded_string_at(view_model, ["scoring_config", "next_action"]),
        bounded_string_from(view_model.get("empty_state")),
        bounded_string_at(view_model, ["primary_action", "command"]),
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
        title: bounded_string_from(view_model.get("title"))
            .unwrap_or_else(|| "dx-check project health".to_string()),
        score_value: u32_from(score_meter.and_then(|value| value.get("value"))),
        score_max: u32_from(score_meter.and_then(|value| value.get("max"))),
        score_percent: u8_from(score_meter.and_then(|value| value.get("percent"))),
        score_estimated: bool_from(score_meter.and_then(|value| value.get("estimated")))
            .unwrap_or(false),
        weight_profile: bounded_string_from(view_model.get("weight_profile"))
            .or_else(|| bounded_string_from(receipt.get("weight_profile")))
            .unwrap_or_else(|| "dx-check.launch-default.v1".to_string()),
        receipt_path: path,
        receipt_present: true,
        receipt_error: if status == "malformed" {
            bounded_string_from(view_model.get("empty_state"))
        } else {
            None
        },
        generated_at_unix_ms,
        last_run_label: last_run_label(
            bounded_string_from(view_model.get("last_run_label")),
            generated_at_unix_ms,
        ),
        pass_count,
        fail_count,
        warn_count,
        skipped_count,
        duration_ms,
        checked_paths,
        skipped_expensive_checks,
        refresh_command: bounded_string_at(view_model, ["primary_action", "command"])
            .unwrap_or_else(|| "dx check --json".to_string()),
        detail_command: bounded_string_at(view_model, ["secondary_action", "command"]),
        scoring_config_status: scoring_config_status.clone(),
        scoring_config_applies_to_score,
        scoring_config_summary: scoring_config_summary(
            &scoring_config_status,
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

fn last_run_label(receipt_label: Option<String>, generated_at_unix_ms: Option<u64>) -> String {
    if let Some(label) = receipt_label {
        return label;
    }

    match generated_at_unix_ms {
        Some(value) => format!("Last run Unix ms: {value}"),
        None => "No dx-check run has been loaded.".to_string(),
    }
}

pub(super) fn missing_snapshot(path: PathBuf) -> DxCheckPanelSnapshot {
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

pub(super) fn malformed_snapshot(path: PathBuf, message: String) -> DxCheckPanelSnapshot {
    let message = bounded_panel_text(&message)
        .unwrap_or_else(|| "dx-check receipt could not be parsed".to_string());

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
    let summary = match (status, applies_to_score, config_path) {
        ("detected_not_applied", false, Some(path)) => {
            format!("detected_not_applied at {path}; not applied")
        }
        ("detected_not_applied", false, None) => "detected_not_applied; not applied".to_string(),
        ("default", _, _) => "launch default weights".to_string(),
        (status, false, Some(path)) => format!("{status} at {path}; not applied"),
        (status, true, Some(path)) => format!("{status} at {path}"),
        (status, true, None) => status.to_string(),
        (status, false, None) => format!("{status}; not applied"),
    };
    bounded_panel_text(&summary).unwrap_or_else(|| "unknown".to_string())
}

fn section_rows(value: Option<&Value>) -> Vec<DxCheckPanelSection> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(8)
        .filter_map(|section| {
            let title = bounded_string_from(section.get("title"))
                .or_else(|| bounded_string_from(section.get("label")))
                .or_else(|| bounded_string_from(section.get("id")))?;
            Some(DxCheckPanelSection {
                title,
                score: u32_from(section.get("score")),
                max_score: u32_from(section.get("max_score")),
                estimated: bool_from(section.get("estimated")).unwrap_or(false),
                status: bounded_string_from(section.get("status"))
                    .unwrap_or_else(|| "unknown".to_string()),
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
            let message = bounded_string_from(notice.get("message"))?;
            Some(DxCheckPanelNotice {
                code: bounded_string_from(notice.get("code"))
                    .unwrap_or_else(|| "dx-check-notice".to_string()),
                message,
                next_action: bounded_string_from(notice.get("next_action")),
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
            let label = bounded_string_from(fix.get("label"))?;
            let next_action = bounded_string_from(fix.get("next_action"))?;
            let raw_command = string_from(fix.get("command"));
            let command = raw_command.and_then(bounded_panel_text);
            Some(DxCheckPanelQuickFix {
                label,
                next_action,
                risk_level: bounded_string_from(fix.get("risk_level"))
                    .unwrap_or_else(|| quick_fix_risk_level(raw_command).to_string()),
                requires_user_approval: bool_from(fix.get("requires_user_approval"))
                    .unwrap_or_else(|| quick_fix_requires_approval(raw_command)),
                writes_receipts: bool_from(fix.get("writes_receipts"))
                    .unwrap_or_else(|| quick_fix_writes_receipts(raw_command)),
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

fn bounded_string_at<const N: usize>(value: &Value, path: [&str; N]) -> Option<String> {
    string_at(value, &path).and_then(|value| bounded_panel_text(&value))
}

fn bounded_string_from(value: Option<&Value>) -> Option<String> {
    bounded_panel_text(string_from(value)?)
}

fn bounded_panel_text(value: &str) -> Option<String> {
    let compacted = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let compacted = compacted
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>();

    if compacted.is_empty() {
        return None;
    }

    if compacted.chars().count() <= MAX_PANEL_TEXT_CHARS {
        return Some(compacted);
    }

    let mut bounded = compacted
        .chars()
        .take(MAX_PANEL_TEXT_CHARS.saturating_sub(3))
        .collect::<String>();
    bounded.push_str("...");
    Some(bounded)
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
        .filter_map(|value| bounded_panel_text(string_from(Some(value))?))
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
