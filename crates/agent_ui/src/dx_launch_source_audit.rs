mod fields;
mod packet_io;
mod paths;
mod rows;
mod snapshot;
mod status;

use self::fields::{bool_field, string_field, usize_field};
use self::packet_io::{packet_schema, read_json_packet};
use self::paths::source_audit_paths;
use self::rows::{delta_row, repo_row};
pub(crate) use self::snapshot::DxLaunchSourceAuditSnapshot;
use self::status::{source_audit_operator_summary, source_audit_status};
use serde_json::Value;
use std::{
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const SOURCE_AUDIT_ROOT: &str = r"G:\Dx\.dx\audit\launch-source";
const SOURCE_AUDIT_LATEST: &str = "latest.json";
const SOURCE_AUDIT_MARKDOWN: &str = "latest.md";
const DX_STUDIO_QA_LATEST: &str = r"G:\Dx\.dx\audit\dx-studio-www-qa\latest.json";
const SOURCE_AUDIT_SCHEMA: &str = "dx.launch_audit.source_guard.v1";
const SOURCE_AUDIT_CACHE_TTL: Duration = Duration::from_secs(5);

static SOURCE_AUDIT_CACHE: OnceLock<Mutex<Option<(Instant, DxLaunchSourceAuditSnapshot)>>> =
    OnceLock::new();

pub(crate) fn launch_source_audit_snapshot() -> DxLaunchSourceAuditSnapshot {
    let cache = SOURCE_AUDIT_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, snapshot)) = cache.as_ref() {
            if now.duration_since(*cached_at) <= SOURCE_AUDIT_CACHE_TTL {
                return snapshot.clone();
            }
        }

        let snapshot = scan_source_audit();
        *cache = Some((now, snapshot.clone()));
        return snapshot;
    }

    scan_source_audit()
}

fn scan_source_audit() -> DxLaunchSourceAuditSnapshot {
    let paths = source_audit_paths();
    let packet = read_json_packet(&paths.latest_path);
    let mut issues = Vec::new();
    let mut last_error = None;
    let audit = match packet {
        Ok(packet) => {
            let schema = packet_schema(&packet);
            if schema != SOURCE_AUDIT_SCHEMA {
                let error = format!(
                    "{} uses schema {schema}, expected {SOURCE_AUDIT_SCHEMA}",
                    paths.latest_path.display()
                );
                issues.push(error.clone());
                last_error = Some(error);
                None
            } else {
                Some(packet)
            }
        }
        Err(error) => {
            if paths.latest_present {
                issues.push(error.clone());
            } else {
                issues.push(format!(
                    "Missing source audit receipt: {}",
                    paths.latest_path.display()
                ));
            }
            last_error = Some(error);
            None
        }
    };
    let audit_ref = audit.as_ref();

    let null_value = Value::Null;
    let coordination = audit_ref
        .and_then(|packet| packet.get("coordination_verdict"))
        .unwrap_or(&null_value);
    let template_trust = audit_ref
        .and_then(|packet| packet.get("template_trust_scan"))
        .unwrap_or(&null_value);
    let dx_studio = audit_ref
        .and_then(|packet| packet.get("dx_studio_www_qa"))
        .unwrap_or(&null_value);
    let repo_readiness = audit_ref
        .and_then(|packet| packet.get("repo_readiness"))
        .and_then(Value::as_array);
    let deltas = audit_ref
        .and_then(|packet| packet.get("worker_output_delta"))
        .and_then(Value::as_array);

    let repo_count = repo_readiness.map(Vec::len).unwrap_or_default();
    let active_output_count = repo_readiness
        .map(|repos| {
            repos
                .iter()
                .filter(|repo| bool_field(repo, "active_output"))
                .count()
        })
        .unwrap_or_default();
    let source_clean_count = array_count(coordination, "source_clean");
    let risk_review_count = array_count(coordination, "blocked_by_risk_review");
    let owner_review_count = array_count(coordination, "owner_review");
    let diff_check_failure_count = array_count(coordination, "diff_check_failures");
    let blocker_rows = audit_ref
        .and_then(|packet| packet.get("blockers"))
        .and_then(Value::as_array)
        .map(|blockers| {
            blockers
                .iter()
                .filter_map(Value::as_str)
                .take(5)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let repo_rows = repo_readiness
        .map(|repos| repos.iter().take(4).map(repo_row).collect::<Vec<_>>())
        .unwrap_or_default();
    let delta_rows = deltas
        .map(|deltas| deltas.iter().take(3).map(delta_row).collect::<Vec<_>>())
        .unwrap_or_default();
    let ready_for_commit_coordination = bool_field(coordination, "ready_for_commit_coordination");
    let next_target = string_field(coordination, "next_exact_target")
        .or_else(|| audit_ref.and_then(|packet| string_field(packet, "next_exact_target")))
        .unwrap_or("Rerun G:\\Dx source audit after active worker output is reconciled.")
        .to_string();
    let score = audit_ref
        .and_then(|packet| usize_field(packet, "score_out_of_100"))
        .unwrap_or_default();
    let passed = audit_ref
        .map(|packet| bool_field(packet, "passed"))
        .unwrap_or_default();
    let coordination_status = string_field(coordination, "status").unwrap_or("missing");
    let status = source_audit_status(
        paths.root_exists,
        paths.latest_present,
        audit_ref.is_some(),
        ready_for_commit_coordination,
        coordination_status,
        risk_review_count,
        diff_check_failure_count,
        passed,
        !issues.is_empty(),
        !blocker_rows.is_empty(),
        score,
    );

    DxLaunchSourceAuditSnapshot {
        root: paths.root,
        latest_path: paths.latest_path,
        markdown_path: paths.markdown_path,
        dx_studio_qa_path: paths.dx_studio_qa_path,
        root_exists: paths.root_exists,
        latest_present: paths.latest_present,
        markdown_present: paths.markdown_present,
        dx_studio_qa_present: paths.dx_studio_qa_present,
        schema_valid: audit_ref.is_some(),
        status: status.to_string(),
        operator_summary: source_audit_operator_summary(
            &issues,
            score,
            coordination_status,
            &next_target,
        ),
        generated_at: audit_ref
            .and_then(|packet| string_field(packet, "generated_at"))
            .unwrap_or("missing")
            .to_string(),
        mode: audit_ref
            .and_then(|packet| string_field(packet, "mode"))
            .unwrap_or("source audit receipt unavailable")
            .to_string(),
        score,
        passed,
        ready_for_commit_coordination,
        next_target,
        repo_count,
        active_output_count,
        source_clean_count,
        risk_review_count,
        owner_review_count,
        diff_check_failure_count,
        template_trust_passed: bool_field(template_trust, "passed"),
        template_roots_scanned: usize_field(template_trust, "scanned_roots").unwrap_or_default(),
        template_roots_total: usize_field(template_trust, "total_roots").unwrap_or_default(),
        template_node_modules_found: usize_field(template_trust, "found_count").unwrap_or_default(),
        dx_studio_score: usize_field(dx_studio, "score_out_of_100").unwrap_or_default(),
        dx_studio_passed: bool_field(dx_studio, "passed"),
        dx_studio_passed_checks: usize_field(dx_studio, "passed_checks").unwrap_or_default(),
        dx_studio_total_checks: usize_field(dx_studio, "total_checks").unwrap_or_default(),
        repo_rows,
        blocker_rows,
        delta_rows,
        first_issue: issues.first().cloned(),
        last_error,
    }
}

fn array_count(value: &Value, field: &str) -> usize {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}
