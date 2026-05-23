mod examples;
mod packet_fields;
mod packets;
mod review;
mod status_counts;

use self::examples::{balanced_examples, push_recovery_commands, push_unique};
use self::packet_fields::{
    bool_field, packet_status, pointer_string, pointer_usize, string_field, usize_field,
};
use self::packets::read_checked_packet;
use self::review::{command_fanout_count, redaction_requires_review};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const DX_LAUNCH_EXAMPLES_ROOT: &str = r"G:\Dx\cli\fixtures\launch-examples";
const IMPORT_SUMMARY_SCHEMA: &str = "dx.launch.import_summary.v1";
const RELEASE_GATE_SCHEMA: &str = "dx.launch.release_gate.v1";
const FALLBACK_DRILL_SCHEMA: &str = "dx.launch.fallback_drill.v1";
const DX_LAUNCH_IMPORT_SUMMARY_COMMAND: &str = "dx launch import-summary --json";
const DX_LAUNCH_RELEASE_GATE_COMMAND: &str = "dx launch release-gate --json";
const DX_LAUNCH_FALLBACK_DRILL_COMMAND: &str = "dx launch fallback-drill --json";
const LAUNCH_READINESS_CACHE_TTL: Duration = Duration::from_secs(5);

const IMPORT_SUMMARY_FILES: &[&str] = &[
    "import-summary-ready.json",
    "import-summary-warning.json",
    "import-summary-blocked.json",
];

const RELEASE_GATE_FILES: &[&str] = &[
    "release-gate-fresh.json",
    "release-gate-stale.json",
    "release-gate-expired.json",
    "release-gate-malformed.json",
    "release-gate-missing.json",
];

const FALLBACK_DRILL_FILES: &[&str] = &[
    "fallback-drill-ready.json",
    "fallback-drill-warning.json",
    "fallback-drill-blocked.json",
];

#[derive(Clone)]
pub(crate) struct DxLaunchReadinessSnapshot {
    pub root: PathBuf,
    pub root_exists: bool,
    pub status: String,
    pub operator_summary: String,
    pub import_summary_count: usize,
    pub release_gate_count: usize,
    pub fallback_drill_count: usize,
    pub import_status_counts: DxLaunchReadinessStatusCounts,
    pub release_gate_status_counts: DxLaunchReadinessStatusCounts,
    pub fallback_status_counts: DxLaunchReadinessStatusCounts,
    pub acceptance_count: usize,
    pub passed_count: usize,
    pub warning_count: usize,
    pub failed_count: usize,
    pub fallback_state_count: usize,
    pub freshness_states: Vec<String>,
    pub fallback_states: Vec<String>,
    pub recovery_commands: Vec<String>,
    pub no_command_fanout: bool,
    pub command_fanout_count: usize,
    pub redaction_requires_review: bool,
    pub first_issue: Option<String>,
    pub next_action: String,
    pub examples: Vec<DxLaunchReadinessExample>,
}

#[derive(Clone, Default)]
pub(crate) struct DxLaunchReadinessStatusCounts {
    pub ready: usize,
    pub warning: usize,
    pub blocked: usize,
    pub unknown: usize,
}

#[derive(Clone)]
pub(crate) struct DxLaunchReadinessExample {
    pub label: String,
    pub status: String,
    pub detail: String,
    pub next_action: Option<String>,
}

static LAUNCH_READINESS_CACHE: OnceLock<Mutex<Option<(Instant, DxLaunchReadinessSnapshot)>>> =
    OnceLock::new();

pub(crate) fn launch_readiness_snapshot() -> DxLaunchReadinessSnapshot {
    let cache = LAUNCH_READINESS_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, snapshot)) = cache.as_ref() {
            if now.duration_since(*cached_at) <= LAUNCH_READINESS_CACHE_TTL {
                return snapshot.clone();
            }
        }

        let snapshot = scan_launch_readiness();
        *cache = Some((now, snapshot.clone()));
        return snapshot;
    }

    scan_launch_readiness()
}

fn scan_launch_readiness() -> DxLaunchReadinessSnapshot {
    let root = PathBuf::from(DX_LAUNCH_EXAMPLES_ROOT);
    let root_exists = root.is_dir();
    let mut snapshot = DxLaunchReadinessSnapshot {
        root: root.clone(),
        root_exists,
        status: "missing".to_string(),
        operator_summary: "Launch readiness examples are not available.".to_string(),
        import_summary_count: 0,
        release_gate_count: 0,
        fallback_drill_count: 0,
        import_status_counts: DxLaunchReadinessStatusCounts::default(),
        release_gate_status_counts: DxLaunchReadinessStatusCounts::default(),
        fallback_status_counts: DxLaunchReadinessStatusCounts::default(),
        acceptance_count: 0,
        passed_count: 0,
        warning_count: 0,
        failed_count: 0,
        fallback_state_count: 0,
        freshness_states: Vec::new(),
        fallback_states: Vec::new(),
        recovery_commands: vec![
            DX_LAUNCH_IMPORT_SUMMARY_COMMAND.to_string(),
            DX_LAUNCH_RELEASE_GATE_COMMAND.to_string(),
            DX_LAUNCH_FALLBACK_DRILL_COMMAND.to_string(),
        ],
        no_command_fanout: true,
        command_fanout_count: 0,
        redaction_requires_review: false,
        first_issue: None,
        next_action: DX_LAUNCH_IMPORT_SUMMARY_COMMAND.to_string(),
        examples: Vec::new(),
    };

    if !root_exists {
        snapshot.first_issue = Some(format!("Missing {}", root.display()));
        return snapshot;
    }

    let mut issues = Vec::new();
    for file_name in IMPORT_SUMMARY_FILES {
        match read_checked_packet(&root.join(file_name), IMPORT_SUMMARY_SCHEMA) {
            Ok(packet) => record_import_summary(file_name, &packet, &mut snapshot),
            Err(error) => issues.push(error),
        }
    }

    for file_name in RELEASE_GATE_FILES {
        match read_checked_packet(&root.join(file_name), RELEASE_GATE_SCHEMA) {
            Ok(packet) => record_release_gate(file_name, &packet, &mut snapshot),
            Err(error) => issues.push(error),
        }
    }

    for file_name in FALLBACK_DRILL_FILES {
        match read_checked_packet(&root.join(file_name), FALLBACK_DRILL_SCHEMA) {
            Ok(packet) => record_fallback_drill(file_name, &packet, &mut snapshot),
            Err(error) => issues.push(error),
        }
    }

    snapshot.first_issue = issues.first().cloned();
    let missing_family = snapshot.import_summary_count == 0
        || snapshot.release_gate_count == 0
        || snapshot.fallback_drill_count == 0;
    if !issues.is_empty()
        || missing_family
        || !snapshot.no_command_fanout
        || snapshot.redaction_requires_review
    {
        snapshot.status = "warning".to_string();
        snapshot.operator_summary =
            "Launch readiness examples need review before the GPUI import is treated as final."
                .to_string();
    } else {
        snapshot.status = "ready".to_string();
        snapshot.operator_summary = "Launch readiness packets ready: import summary, release gate, and fallback drill examples are source-owned and fanout-safe.".to_string();
        snapshot.next_action = "zed_launch_import_ready".to_string();
    }

    snapshot.examples = balanced_examples(&snapshot.examples);
    snapshot
}

fn record_import_summary(
    file_name: &str,
    packet: &Value,
    snapshot: &mut DxLaunchReadinessSnapshot,
) {
    snapshot.import_summary_count += 1;
    let status = packet_status(packet);
    snapshot.import_status_counts.record(&status);
    record_packet_safety(packet, snapshot);

    let freshness = pointer_string(packet, "/freshness_policy/latest_freshness_state")
        .unwrap_or("unknown")
        .to_string();
    push_unique(&mut snapshot.freshness_states, freshness.clone());
    push_recovery_commands(packet, snapshot);

    let acceptance_count = pointer_usize(packet, "/release_gate/acceptance_count").unwrap_or(0);
    let action_count = pointer_usize(packet, "/handoff/action_count").unwrap_or(0);
    let next_action = string_field(packet, "next_action").map(ToString::to_string);
    snapshot.examples.push(DxLaunchReadinessExample {
        label: format!("Import {file_name}"),
        status,
        detail: format!(
            "{freshness} cached receipt, {acceptance_count} gate row(s), {action_count} action(s)"
        ),
        next_action,
    });
}

fn record_release_gate(file_name: &str, packet: &Value, snapshot: &mut DxLaunchReadinessSnapshot) {
    snapshot.release_gate_count += 1;
    let status = packet_status(packet);
    snapshot.release_gate_status_counts.record(&status);
    record_packet_safety(packet, snapshot);

    snapshot.acceptance_count = snapshot
        .acceptance_count
        .max(usize_field(packet, "acceptance_count").unwrap_or(0));
    snapshot.passed_count = snapshot
        .passed_count
        .max(usize_field(packet, "passed_count").unwrap_or(0));
    snapshot.warning_count = snapshot
        .warning_count
        .max(usize_field(packet, "warning_count").unwrap_or(0));
    snapshot.failed_count = snapshot
        .failed_count
        .max(usize_field(packet, "failed_count").unwrap_or(0));

    let freshness = pointer_string(packet, "/latest_status_receipt/freshness_state")
        .unwrap_or("unknown")
        .to_string();
    push_unique(&mut snapshot.freshness_states, freshness.clone());

    let next_action = string_field(packet, "next_action").map(ToString::to_string);
    snapshot.examples.push(DxLaunchReadinessExample {
        label: format!("Gate {file_name}"),
        status,
        detail: format!(
            "{} passed / {} warning / {} failed, cached {freshness}",
            usize_field(packet, "passed_count").unwrap_or(0),
            usize_field(packet, "warning_count").unwrap_or(0),
            usize_field(packet, "failed_count").unwrap_or(0),
        ),
        next_action,
    });
}

fn record_fallback_drill(
    file_name: &str,
    packet: &Value,
    snapshot: &mut DxLaunchReadinessSnapshot,
) {
    snapshot.fallback_drill_count += 1;
    let status = packet_status(packet);
    snapshot.fallback_status_counts.record(&status);
    record_packet_safety(packet, snapshot);
    push_recovery_commands(packet, snapshot);

    let active_state = string_field(packet, "active_receipt_state")
        .unwrap_or("unknown")
        .to_string();
    push_unique(&mut snapshot.fallback_states, active_state.clone());
    snapshot.fallback_state_count = snapshot
        .fallback_state_count
        .max(usize_field(packet, "state_count").unwrap_or(0));

    let next_action = string_field(packet, "next_action").map(ToString::to_string);
    snapshot.examples.push(DxLaunchReadinessExample {
        label: format!("Fallback {file_name}"),
        status,
        detail: format!(
            "{active_state} active state, {} cached state(s)",
            usize_field(packet, "state_count").unwrap_or(0)
        ),
        next_action,
    });
}

fn record_packet_safety(packet: &Value, snapshot: &mut DxLaunchReadinessSnapshot) {
    let fanout = command_fanout_count(packet);
    snapshot.command_fanout_count += fanout;
    snapshot.no_command_fanout =
        snapshot.no_command_fanout && bool_field(packet, "no_command_fanout") && fanout == 0;
    snapshot.redaction_requires_review |= redaction_requires_review(packet);
}
