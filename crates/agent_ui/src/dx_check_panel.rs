use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const CHECK_RECEIPT_SCHEMA: &str = "dx.check.receipt.v1";
const ZED_PANEL_SCHEMA: &str = "dx.check.zed_panel.v1";
const VIEW_MODEL_SCHEMA: &str = "dx.www.check_panel_view_model.v1";
const CHECK_RECEIPT_RELATIVE_PATH: &[&str] = &[".dx", "receipts", "check", "check-latest.json"];
const DX_FALLBACK_CHECK_RECEIPT: &str = r"G:\Dx\.dx\receipts\check\check-latest.json";
const CHECK_PANEL_CACHE_TTL: Duration = Duration::from_secs(5);
const MAX_RECEIPT_BYTES: u64 = 256 * 1024;

mod parser;
mod reader;

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

        let snapshot = reader::read_latest_check_panel(&normalized_roots);
        *cache = Some(DxCheckPanelCache {
            cached_at: now,
            workspace_roots: normalized_roots,
            snapshot: snapshot.clone(),
        });
        return snapshot;
    }

    reader::read_latest_check_panel(&normalized_roots)
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parser::panel_from_receipt_value;
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
