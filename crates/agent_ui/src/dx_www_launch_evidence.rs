use serde_json::Value;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

mod expected_artifacts;

use expected_artifacts::{
    EXPECTED_EVIDENCE_ARTIFACTS, EvidenceFormat, ExpectedWwwEvidenceArtifact,
};

const DEFAULT_DX_WWW_PROJECT: &str = r"G:\WWW\www";
const FALLBACK_DX_WWW_TEMPLATE: &str = r"G:\Dx\www\examples\launch-template";
const RELEASE_ROOT: &str = ".dx/forge/release";
const WWW_EVIDENCE_CACHE_TTL: Duration = Duration::from_secs(5);
const MAX_EVIDENCE_BYTES: u64 = 256 * 1024;

#[derive(Clone)]
pub(crate) struct DxWwwLaunchEvidenceSnapshot {
    pub project_root: PathBuf,
    pub project_root_exists: bool,
    pub release_root: PathBuf,
    pub release_root_exists: bool,
    pub status: String,
    pub operator_summary: String,
    pub expected_count: usize,
    pub present_count: usize,
    pub json_count: usize,
    pub markdown_count: usize,
    pub passed_count: usize,
    pub blocked_count: usize,
    pub warning_count: usize,
    pub no_execution_count: usize,
    pub latest_rows: Vec<String>,
    pub missing_rows: Vec<String>,
    pub next_commands: Vec<String>,
    pub first_issue: Option<String>,
}

#[derive(Clone)]
struct DxWwwLaunchEvidenceArtifact {
    pub label: String,
    pub path: String,
    pub command: String,
    pub present: bool,
    pub status: String,
    pub schema: String,
    pub score: Option<u64>,
    pub no_execution: bool,
    pub finding_count: usize,
    pub modified_ms: Option<u64>,
}

static WWW_EVIDENCE_CACHE: OnceLock<
    Mutex<Option<(Instant, Vec<String>, DxWwwLaunchEvidenceSnapshot)>>,
> = OnceLock::new();

pub(crate) fn www_launch_evidence_snapshot(
    workspace_roots: &[String],
) -> DxWwwLaunchEvidenceSnapshot {
    let cache = WWW_EVIDENCE_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, cached_roots, snapshot)) = cache.as_ref() {
            if now.duration_since(*cached_at) <= WWW_EVIDENCE_CACHE_TTL
                && cached_roots.as_slice() == workspace_roots
            {
                return snapshot.clone();
            }
        }

        let snapshot = scan_www_launch_evidence(workspace_roots);
        *cache = Some((now, workspace_roots.to_vec(), snapshot.clone()));
        return snapshot;
    }

    scan_www_launch_evidence(workspace_roots)
}

fn scan_www_launch_evidence(workspace_roots: &[String]) -> DxWwwLaunchEvidenceSnapshot {
    let project_root = select_www_project_root(workspace_roots);
    let project_root_exists = project_root.is_dir();
    let release_root = project_root.join(RELEASE_ROOT);
    let release_root_exists = release_root.is_dir();

    let mut rows = Vec::new();
    let mut issues = Vec::new();
    for expected in EXPECTED_EVIDENCE_ARTIFACTS {
        let row = inspect_expected_artifact(&project_root, *expected);
        if row.present && row.status == "blocked" {
            issues.push(format!("{} reports blocked status", row.label));
        } else if row.present && row.status == "invalid" {
            issues.push(format!("{} is not readable as metadata", row.label));
        }
        rows.push(row);
    }

    let expected_count = EXPECTED_EVIDENCE_ARTIFACTS.len();
    let present_count = rows.iter().filter(|row| row.present).count();
    let json_count = rows
        .iter()
        .filter(|row| row.present && row.path.ends_with(".json"))
        .count();
    let markdown_count = rows
        .iter()
        .filter(|row| row.present && row.path.ends_with(".md"))
        .count();
    let passed_count = rows
        .iter()
        .filter(|row| row.present && row.status == "ready")
        .count();
    let blocked_count = rows
        .iter()
        .filter(|row| row.present && row.status == "blocked")
        .count();
    let warning_count = rows
        .iter()
        .filter(|row| row.present && row.status == "warning")
        .count();
    let no_execution_count = rows
        .iter()
        .filter(|row| row.present && row.no_execution)
        .count();

    let mut latest = rows
        .iter()
        .filter(|row| row.present)
        .filter_map(|row| row.modified_ms.map(|modified| (modified, row.clone())))
        .collect::<Vec<_>>();
    latest.sort_by(|left, right| right.0.cmp(&left.0));
    let latest_rows = latest
        .into_iter()
        .take(4)
        .map(|(_, row)| evidence_row_summary(&row))
        .collect::<Vec<_>>();
    let missing_rows = rows
        .iter()
        .filter(|row| !row.present)
        .take(5)
        .map(|row| format!("{} -> {}", row.label, row.command))
        .collect::<Vec<_>>();
    let next_commands = rows
        .iter()
        .filter(|row| !row.present || row.status == "blocked" || row.status == "invalid")
        .take(3)
        .map(|row| row.command.clone())
        .collect::<Vec<_>>();

    let status = if !project_root_exists || present_count == 0 {
        "missing"
    } else if !issues.is_empty() || blocked_count > 0 {
        "blocked"
    } else if present_count < expected_count || warning_count > 0 {
        "warning"
    } else {
        "ready"
    };
    let operator_summary = www_evidence_operator_summary(
        status,
        present_count,
        expected_count,
        &project_root,
        release_root_exists,
    );

    DxWwwLaunchEvidenceSnapshot {
        project_root,
        project_root_exists,
        release_root,
        release_root_exists,
        status: status.to_string(),
        operator_summary,
        expected_count,
        present_count,
        json_count,
        markdown_count,
        passed_count,
        blocked_count,
        warning_count,
        no_execution_count,
        latest_rows,
        missing_rows,
        next_commands,
        first_issue: issues.first().cloned(),
    }
}

fn select_www_project_root(workspace_roots: &[String]) -> PathBuf {
    for root in workspace_roots {
        let path = PathBuf::from(root);
        if is_dx_www_project_candidate(&path) {
            return path;
        }
    }

    for candidate in [DEFAULT_DX_WWW_PROJECT, FALLBACK_DX_WWW_TEMPLATE] {
        let path = PathBuf::from(candidate);
        if path.is_dir() {
            return path;
        }
    }

    PathBuf::from(DEFAULT_DX_WWW_PROJECT)
}

fn is_dx_www_project_candidate(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    path.join(".dx/forge").is_dir()
        || path.join("app/launch/page.tsx").is_file()
        || path.join("launch-route-contract.ts").is_file()
        || path.join("dx-www").is_dir()
}

fn inspect_expected_artifact(
    project_root: &Path,
    expected: ExpectedWwwEvidenceArtifact,
) -> DxWwwLaunchEvidenceArtifact {
    let path = project_root.join(expected.relative_path);
    let metadata = path.metadata().ok();
    let present = metadata.as_ref().is_some_and(|metadata| metadata.is_file());
    let modified_ms = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(system_time_ms);

    if !present {
        return DxWwwLaunchEvidenceArtifact {
            label: expected.label.to_string(),
            path: expected.relative_path.to_string(),
            command: expected.command.to_string(),
            present,
            status: "missing".to_string(),
            schema: "missing".to_string(),
            score: None,
            no_execution: true,
            finding_count: 0,
            modified_ms,
        };
    }

    if expected.format == EvidenceFormat::Markdown {
        return DxWwwLaunchEvidenceArtifact {
            label: expected.label.to_string(),
            path: expected.relative_path.to_string(),
            command: expected.command.to_string(),
            present,
            status: "ready".to_string(),
            schema: "markdown".to_string(),
            score: None,
            no_execution: true,
            finding_count: 0,
            modified_ms,
        };
    }

    match read_json_packet(&path) {
        Ok(packet) => {
            let passed = packet.get("passed").and_then(Value::as_bool);
            let finding_count = packet
                .get("findings")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or_default();
            let status = match passed {
                Some(true) => "ready",
                Some(false) => "blocked",
                None if finding_count > 0 => "warning",
                None => "ready",
            };
            DxWwwLaunchEvidenceArtifact {
                label: expected.label.to_string(),
                path: expected.relative_path.to_string(),
                command: expected.command.to_string(),
                present,
                status: status.to_string(),
                schema: packet_schema(&packet),
                score: packet.get("score").and_then(Value::as_u64),
                no_execution: packet
                    .get("no_execution")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                finding_count,
                modified_ms,
            }
        }
        Err(error) => DxWwwLaunchEvidenceArtifact {
            label: expected.label.to_string(),
            path: expected.relative_path.to_string(),
            command: expected.command.to_string(),
            present,
            status: "invalid".to_string(),
            schema: error,
            score: None,
            no_execution: false,
            finding_count: 1,
            modified_ms,
        },
    }
}

fn read_json_packet(path: &Path) -> Result<Value, String> {
    let metadata = path
        .metadata()
        .map_err(|error| format!("Unable to inspect WWW launch evidence: {error}"))?;
    if metadata.len() > MAX_EVIDENCE_BYTES {
        return Err(format!(
            "WWW launch evidence is too large to render safely: {} bytes",
            metadata.len()
        ));
    }

    let mut contents = String::new();
    File::open(path)
        .and_then(|mut file| file.read_to_string(&mut contents))
        .map_err(|error| format!("Unable to read WWW launch evidence: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Unable to parse WWW launch evidence: {error}"))
}

fn packet_schema(packet: &Value) -> String {
    packet
        .get("schema")
        .or_else(|| packet.get("schema_version"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string()
}

fn evidence_row_summary(row: &DxWwwLaunchEvidenceArtifact) -> String {
    let score = row
        .score
        .map(|score| format!("{score}/100"))
        .unwrap_or_else(|| row.schema.clone());
    let findings = if row.finding_count == 0 {
        "no findings".to_string()
    } else {
        format!("{} finding(s)", row.finding_count)
    };
    format!("{}: {} ({score}, {findings})", row.label, row.status)
}

fn www_evidence_operator_summary(
    status: &str,
    present_count: usize,
    expected_count: usize,
    project_root: &Path,
    release_root_exists: bool,
) -> String {
    if status == "ready" {
        return format!(
            "DX-WWW release evidence ready: {present_count}/{expected_count} launch handoff artifacts are present."
        );
    }

    if present_count == 0 {
        return format!(
            "DX-WWW release evidence is not generated yet for `{}`.",
            project_root.display()
        );
    }

    if !release_root_exists {
        return format!(
            "DX-WWW release root is missing at `{}`.",
            project_root.join(RELEASE_ROOT).display()
        );
    }

    format!(
        "DX-WWW release evidence is partial: {present_count}/{expected_count} expected handoff artifacts are present."
    )
}

fn system_time_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
}
