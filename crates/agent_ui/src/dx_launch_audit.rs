mod packet_fields;
mod packets;
mod review;
mod status_summaries;

use self::packet_fields::{array_len, bool_field, bool_label, string_field, usize_field};
use self::packets::read_checked_packet;
use self::review::{command_fanout_count, redaction_requires_review};
use self::status_summaries::{
    status_agent_summary, status_discovery_summary, status_token_summary,
};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const DX_LAUNCH_EXAMPLES_ROOT: &str = r"G:\Dx\cli\fixtures\launch-examples";
const SCHEMAS_FILE: &str = "schemas.json";
const FIXTURES_FILE: &str = "fixtures.json";
const SMOKE_FILE: &str = "smoke.json";
const STATUS_FILE: &str = "status.json";
const SCHEMAS_SCHEMA: &str = "dx.launch.schemas.v1";
const FIXTURES_SCHEMA: &str = "dx.launch.fixtures.v1";
const SMOKE_SCHEMA: &str = "dx.launch.smoke.v1";
const STATUS_SCHEMA: &str = "dx.launch.status.v1";
const DX_LAUNCH_SCHEMAS_COMMAND: &str = "dx launch schemas --json";
const LAUNCH_AUDIT_CACHE_TTL: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(crate) struct DxLaunchAuditSnapshot {
    pub root: PathBuf,
    pub root_exists: bool,
    pub schemas_path: PathBuf,
    pub fixtures_path: PathBuf,
    pub smoke_path: PathBuf,
    pub status_path: PathBuf,
    pub schemas_present: bool,
    pub fixtures_present: bool,
    pub smoke_present: bool,
    pub status_present: bool,
    pub status: String,
    pub operator_summary: String,
    pub command_count: usize,
    pub startup_poll_count: usize,
    pub user_action_count: usize,
    pub write_path_count: usize,
    pub metadata_only_count: usize,
    pub fixture_count: usize,
    pub fixture_match_count: usize,
    pub smoke_check_count: usize,
    pub smoke_passed_count: usize,
    pub smoke_warning_count: usize,
    pub smoke_failed_count: usize,
    pub example_status: String,
    pub example_agents: String,
    pub example_tokens: String,
    pub example_discovery: String,
    pub command_fanout_count: usize,
    pub redaction_requires_review: bool,
    pub command_rows: Vec<String>,
    pub fixture_rows: Vec<String>,
    pub smoke_rows: Vec<String>,
    pub first_issue: Option<String>,
    pub next_action: String,
}

static LAUNCH_AUDIT_CACHE: OnceLock<Mutex<Option<(Instant, DxLaunchAuditSnapshot)>>> =
    OnceLock::new();

pub(crate) fn launch_audit_snapshot() -> DxLaunchAuditSnapshot {
    let cache = LAUNCH_AUDIT_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, snapshot)) = cache.as_ref() {
            if now.duration_since(*cached_at) <= LAUNCH_AUDIT_CACHE_TTL {
                return snapshot.clone();
            }
        }

        let snapshot = scan_launch_audit();
        *cache = Some((now, snapshot.clone()));
        return snapshot;
    }

    scan_launch_audit()
}

fn scan_launch_audit() -> DxLaunchAuditSnapshot {
    let root = PathBuf::from(DX_LAUNCH_EXAMPLES_ROOT);
    let schemas_path = root.join(SCHEMAS_FILE);
    let fixtures_path = root.join(FIXTURES_FILE);
    let smoke_path = root.join(SMOKE_FILE);
    let status_path = root.join(STATUS_FILE);
    let root_exists = root.is_dir();
    let schemas_present = schemas_path.is_file();
    let fixtures_present = fixtures_path.is_file();
    let smoke_present = smoke_path.is_file();
    let status_present = status_path.is_file();
    let mut issues = Vec::new();

    let schemas = read_checked_packet(&schemas_path, SCHEMAS_SCHEMA);
    let fixtures = read_checked_packet(&fixtures_path, FIXTURES_SCHEMA);
    let smoke = read_checked_packet(&smoke_path, SMOKE_SCHEMA);
    let status_packet = read_checked_packet(&status_path, STATUS_SCHEMA);

    collect_packet_issue(schemas_present, &schemas_path, &schemas, &mut issues);
    collect_packet_issue(fixtures_present, &fixtures_path, &fixtures, &mut issues);
    collect_packet_issue(smoke_present, &smoke_path, &smoke, &mut issues);
    collect_packet_issue(status_present, &status_path, &status_packet, &mut issues);

    let schemas_ref = schemas.as_ref().ok();
    let fixtures_ref = fixtures.as_ref().ok();
    let smoke_ref = smoke.as_ref().ok();
    let status_ref = status_packet.as_ref().ok();
    let commands = schemas_ref
        .and_then(|value| value.get("commands"))
        .and_then(Value::as_array);
    let fixtures = fixtures_ref
        .and_then(|value| value.get("fixtures"))
        .and_then(Value::as_array);
    let checks = smoke_ref
        .and_then(|value| value.get("checks"))
        .and_then(Value::as_array);

    let command_count = schemas_ref
        .and_then(|value| usize_field(value, "command_count"))
        .or_else(|| commands.map(Vec::len))
        .unwrap_or_default();
    let startup_poll_count = commands
        .map(|commands| {
            commands
                .iter()
                .filter(|command| bool_field(command, "poll_on_startup"))
                .count()
        })
        .unwrap_or_default();
    let user_action_count = commands
        .map(|commands| {
            commands
                .iter()
                .filter(|command| bool_field(command, "user_action_required"))
                .count()
        })
        .unwrap_or_default();
    let write_path_count = commands
        .map(|commands| {
            commands
                .iter()
                .map(|command| array_len(command, "writes"))
                .sum::<usize>()
        })
        .unwrap_or_default();
    let metadata_only_count = commands
        .map(|commands| {
            commands
                .iter()
                .filter(|command| {
                    string_field(command, "execution_risk")
                        .unwrap_or("")
                        .contains("metadata_only")
                })
                .count()
        })
        .unwrap_or_default();
    let fixture_count = fixtures_ref
        .and_then(|value| usize_field(value, "fixture_count"))
        .or_else(|| fixtures.map(Vec::len))
        .unwrap_or_default();
    let fixture_match_count = fixtures
        .map(|fixtures| {
            fixtures
                .iter()
                .filter(|fixture| bool_field(fixture, "status_matches_expected"))
                .count()
        })
        .unwrap_or_default();
    let smoke_check_count = smoke_ref
        .and_then(|value| usize_field(value, "check_count"))
        .or_else(|| checks.map(Vec::len))
        .unwrap_or_default();
    let smoke_passed_count = smoke_ref
        .and_then(|value| usize_field(value, "passed_count"))
        .unwrap_or_default();
    let smoke_warning_count = smoke_ref
        .and_then(|value| usize_field(value, "warning_count"))
        .unwrap_or_default();
    let smoke_failed_count = smoke_ref
        .and_then(|value| usize_field(value, "failed_count"))
        .unwrap_or_default();
    let command_rows = commands
        .map(|commands| {
            commands
                .iter()
                .take(5)
                .filter_map(|command| {
                    Some(format!(
                        "{} -> {}",
                        string_field(command, "cli_command")?,
                        string_field(command, "schema_version").unwrap_or("unknown")
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let fixture_rows = fixtures
        .map(|fixtures| {
            fixtures
                .iter()
                .take(3)
                .filter_map(|fixture| {
                    let render = fixture.get("render_state");
                    Some(format!(
                        "{}: {} / {}",
                        string_field(fixture, "label")?,
                        string_field(fixture, "expected_status").unwrap_or("unknown"),
                        render
                            .and_then(|render| string_field(render, "primary_action"))
                            .unwrap_or("no action")
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let smoke_rows = checks
        .map(|checks| {
            checks
                .iter()
                .take(4)
                .filter_map(|check| {
                    Some(format!(
                        "{}: {}",
                        string_field(check, "label")?,
                        string_field(check, "status").unwrap_or("unknown")
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let command_fanout_count = schemas_ref.map(command_fanout_count).unwrap_or_default()
        + fixtures_ref.map(command_fanout_count).unwrap_or_default()
        + smoke_ref.map(command_fanout_count).unwrap_or_default()
        + status_ref.map(command_fanout_count).unwrap_or_default();
    let redaction_requires_review = schemas_ref.is_some_and(redaction_requires_review)
        || fixtures_ref.is_some_and(redaction_requires_review)
        || smoke_ref.is_some_and(redaction_requires_review)
        || status_ref.is_some_and(redaction_requires_review);

    let missing_packet =
        !schemas_present || !fixtures_present || !smoke_present || !status_present || !root_exists;
    let status = if missing_packet {
        "missing"
    } else if smoke_failed_count > 0 || command_fanout_count > 0 {
        "blocked"
    } else if !issues.is_empty() || smoke_warning_count > 0 || redaction_requires_review {
        "warning"
    } else {
        "ready"
    };
    let operator_summary = smoke_ref
        .and_then(|value| string_field(value, "operator_summary"))
        .or_else(|| schemas_ref.and_then(|value| string_field(value, "operator_summary")))
        .unwrap_or("Launch audit packets are not available.")
        .to_string();
    let next_action = if issues.is_empty() {
        smoke_ref
            .and_then(|value| string_field(value, "next_action"))
            .or_else(|| schemas_ref.and_then(|value| string_field(value, "next_action")))
            .unwrap_or(DX_LAUNCH_SCHEMAS_COMMAND)
    } else {
        DX_LAUNCH_SCHEMAS_COMMAND
    };

    DxLaunchAuditSnapshot {
        root,
        root_exists,
        schemas_path,
        fixtures_path,
        smoke_path,
        status_path,
        schemas_present,
        fixtures_present,
        smoke_present,
        status_present,
        status: status.to_string(),
        operator_summary,
        command_count,
        startup_poll_count,
        user_action_count,
        write_path_count,
        metadata_only_count,
        fixture_count,
        fixture_match_count,
        smoke_check_count,
        smoke_passed_count,
        smoke_warning_count,
        smoke_failed_count,
        example_status: status_ref
            .and_then(|value| string_field(value, "status"))
            .unwrap_or("missing")
            .to_string(),
        example_agents: status_agent_summary(status_ref),
        example_tokens: status_token_summary(status_ref),
        example_discovery: status_discovery_summary(status_ref),
        command_fanout_count,
        redaction_requires_review,
        command_rows,
        fixture_rows,
        smoke_rows,
        first_issue: issues.first().cloned(),
        next_action: next_action.to_string(),
    }
}

fn collect_packet_issue(
    present: bool,
    path: &Path,
    packet: &Result<Value, String>,
    issues: &mut Vec<String>,
) {
    if !present {
        issues.push(format!("Missing {}", path.display()));
    } else if let Err(error) = packet {
        issues.push(error.clone());
    }
}
