use serde_json::Value;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const SOURCE_AUDIT_ROOT: &str = r"G:\Dx\.dx\audit\launch-source";
const SOURCE_AUDIT_LATEST: &str = "latest.json";
const SOURCE_AUDIT_MARKDOWN: &str = "latest.md";
const DX_STUDIO_QA_LATEST: &str = r"G:\Dx\.dx\audit\dx-studio-www-qa\latest.json";
const SOURCE_AUDIT_SCHEMA: &str = "dx.launch_audit.source_guard.v1";
const SOURCE_AUDIT_CACHE_TTL: Duration = Duration::from_secs(5);
const MAX_AUDIT_BYTES: u64 = 512 * 1024;

#[derive(Clone)]
pub(crate) struct DxLaunchSourceAuditSnapshot {
    pub root: PathBuf,
    pub latest_path: PathBuf,
    pub markdown_path: PathBuf,
    pub dx_studio_qa_path: PathBuf,
    pub root_exists: bool,
    pub latest_present: bool,
    pub markdown_present: bool,
    pub dx_studio_qa_present: bool,
    pub schema_valid: bool,
    pub status: String,
    pub operator_summary: String,
    pub generated_at: String,
    pub mode: String,
    pub score: usize,
    pub passed: bool,
    pub ready_for_commit_coordination: bool,
    pub next_target: String,
    pub repo_count: usize,
    pub active_output_count: usize,
    pub source_clean_count: usize,
    pub risk_review_count: usize,
    pub owner_review_count: usize,
    pub diff_check_failure_count: usize,
    pub template_trust_passed: bool,
    pub template_roots_scanned: usize,
    pub template_roots_total: usize,
    pub template_node_modules_found: usize,
    pub dx_studio_score: usize,
    pub dx_studio_passed: bool,
    pub dx_studio_passed_checks: usize,
    pub dx_studio_total_checks: usize,
    pub repo_rows: Vec<String>,
    pub blocker_rows: Vec<String>,
    pub delta_rows: Vec<String>,
    pub first_issue: Option<String>,
    pub last_error: Option<String>,
}

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
    let root = PathBuf::from(SOURCE_AUDIT_ROOT);
    let latest_path = root.join(SOURCE_AUDIT_LATEST);
    let markdown_path = root.join(SOURCE_AUDIT_MARKDOWN);
    let dx_studio_qa_path = PathBuf::from(DX_STUDIO_QA_LATEST);
    let root_exists = root.is_dir();
    let latest_present = latest_path.is_file();
    let markdown_present = markdown_path.is_file();
    let dx_studio_qa_present = dx_studio_qa_path.is_file();

    let packet = read_json_packet(&latest_path);
    let mut issues = Vec::new();
    let mut last_error = None;
    let audit = match packet {
        Ok(packet) => {
            let schema = packet_schema(&packet);
            if schema != SOURCE_AUDIT_SCHEMA {
                let error = format!(
                    "{} uses schema {schema}, expected {SOURCE_AUDIT_SCHEMA}",
                    latest_path.display()
                );
                issues.push(error.clone());
                last_error = Some(error);
                None
            } else {
                Some(packet)
            }
        }
        Err(error) => {
            if latest_present {
                issues.push(error.clone());
            } else {
                issues.push(format!(
                    "Missing source audit receipt: {}",
                    latest_path.display()
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
    let source_clean_count = coordination
        .get("source_clean")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    let risk_review_count = coordination
        .get("blocked_by_risk_review")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    let owner_review_count = coordination
        .get("owner_review")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    let diff_check_failure_count = coordination
        .get("diff_check_failures")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
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
    let status = if !root_exists || !latest_present {
        "missing"
    } else if audit_ref.is_none() {
        "invalid"
    } else if !ready_for_commit_coordination
        || coordination_status.contains("blocked")
        || risk_review_count > 0
        || diff_check_failure_count > 0
    {
        "blocked"
    } else if !passed || !issues.is_empty() || !blocker_rows.is_empty() || score < 100 {
        "warning"
    } else {
        "ready"
    };
    let operator_summary = if let Some(first_issue) = issues.first() {
        first_issue.clone()
    } else {
        format!(
            "DX source audit {score}/100 reports {coordination_status}; next target: {next_target}"
        )
    };

    DxLaunchSourceAuditSnapshot {
        root,
        latest_path,
        markdown_path,
        dx_studio_qa_path,
        root_exists,
        latest_present,
        markdown_present,
        dx_studio_qa_present,
        schema_valid: audit_ref.is_some(),
        status: status.to_string(),
        operator_summary,
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

fn repo_row(repo: &Value) -> String {
    format!(
        "{}: {} / {} (entries {}, staged {}, unstaged {}, untracked {}, split {}, diff {})",
        string_field(repo, "name").unwrap_or("unknown"),
        string_field(repo, "state").unwrap_or("unknown"),
        string_field(repo, "commit_gate").unwrap_or("unknown"),
        usize_field(repo, "total_entries").unwrap_or_default(),
        usize_field(repo, "staged_count").unwrap_or_default(),
        usize_field(repo, "unstaged_count").unwrap_or_default(),
        usize_field(repo, "untracked_count").unwrap_or_default(),
        usize_field(repo, "split_index_worktree_count").unwrap_or_default(),
        usize_field(repo, "diff_check_exit")
            .map(|exit| exit.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    )
}

fn delta_row(delta: &Value) -> String {
    format!(
        "{}: branch_changed={} active={} -> {} entries_delta={} tracked_delta={}",
        string_field(delta, "name").unwrap_or("unknown"),
        bool_label(bool_field(delta, "branch_changed")),
        bool_label(bool_field(delta, "previous_active_output")),
        bool_label(bool_field(delta, "current_active_output")),
        signed_field(delta, "total_entries_delta"),
        signed_field(delta, "tracked_dirty_delta")
    )
}

fn read_json_packet(path: &Path) -> Result<Value, String> {
    let metadata = path
        .metadata()
        .map_err(|error| format!("Unable to inspect source audit packet: {error}"))?;
    if metadata.len() > MAX_AUDIT_BYTES {
        return Err(format!(
            "Source audit packet is too large to render safely: {} bytes",
            metadata.len()
        ));
    }

    let mut contents = String::new();
    File::open(path)
        .and_then(|mut file| file.read_to_string(&mut contents))
        .map_err(|error| format!("Unable to read source audit packet: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Unable to parse source audit packet: {error}"))
}

fn packet_schema(packet: &Value) -> String {
    packet
        .get("schema")
        .or_else(|| packet.get("schema_version"))
        .and_then(Value::as_str)
        .unwrap_or("missing")
        .to_string()
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn usize_field(value: &Value, field: &str) -> Option<usize> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| value.try_into().ok())
}

fn signed_field(value: &Value, field: &str) -> i64 {
    value.get(field).and_then(Value::as_i64).unwrap_or_default()
}

fn bool_field(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_bool).unwrap_or(false)
}

fn bool_label(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
