use serde_json::Value;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const DX_LAUNCH_RECEIPT_ROOT: &str = r"G:\Dx\.dx\receipts\launch";
const DX_LAUNCH_STATUS_LATEST: &str = "status-latest.json";
const DX_LAUNCH_STATUS_SCHEMA: &str = "dx.launch.status.v1";
const DX_LAUNCH_STATUS_COMMAND: &str = "dx launch status --json";
const LAUNCH_STATUS_CACHE_TTL: Duration = Duration::from_secs(5);
const MAX_RECEIPT_BYTES: u64 = 128 * 1024;

#[derive(Clone)]
pub(crate) struct DxLaunchStatusSnapshot {
    pub root: PathBuf,
    pub latest_path: PathBuf,
    pub root_exists: bool,
    pub latest_present: bool,
    pub schema_valid: bool,
    pub status: String,
    pub operator_summary: String,
    pub agents: DxLaunchAgentsSummary,
    pub tokens: DxLaunchTokensSummary,
    pub discovery: DxLaunchDiscoverySummary,
    pub last_error: Option<String>,
    pub next_action: String,
    pub redaction_requires_review: bool,
    pub redaction_summary: String,
}

#[derive(Clone)]
pub(crate) struct DxLaunchAgentsSummary {
    pub status: String,
    pub configured_accounts: usize,
    pub connected_accounts: usize,
    pub accounts_needing_connection: usize,
    pub qr_connect_supported: usize,
    pub automation_count: usize,
    pub active_task_count: usize,
    pub next_action: String,
}

#[derive(Clone)]
pub(crate) struct DxLaunchTokensSummary {
    pub status: String,
    pub budget_state: String,
    pub estimated_tokens: u64,
    pub soft_budget_tokens: u64,
    pub hard_budget_tokens: u64,
    pub next_action: String,
}

#[derive(Clone)]
pub(crate) struct DxLaunchDiscoverySummary {
    pub status: String,
    pub templates_command: String,
    pub packages_command: String,
    pub www_manifest_present: bool,
    pub configured_binary_present: bool,
    pub next_action: String,
}

static LAUNCH_STATUS_CACHE: OnceLock<Mutex<Option<(Instant, DxLaunchStatusSnapshot)>>> =
    OnceLock::new();

pub(crate) fn launch_status_snapshot() -> DxLaunchStatusSnapshot {
    let cache = LAUNCH_STATUS_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, snapshot)) = cache.as_ref() {
            if now.duration_since(*cached_at) <= LAUNCH_STATUS_CACHE_TTL {
                return snapshot.clone();
            }
        }

        let snapshot = scan_launch_status();
        *cache = Some((now, snapshot.clone()));
        return snapshot;
    }

    scan_launch_status()
}

fn scan_launch_status() -> DxLaunchStatusSnapshot {
    let root = PathBuf::from(DX_LAUNCH_RECEIPT_ROOT);
    let latest_path = root.join(DX_LAUNCH_STATUS_LATEST);
    let root_exists = root.is_dir();
    let latest_present = latest_path.is_file();

    if !root_exists {
        return missing_snapshot(root, latest_path, false, "Missing receipt root");
    }

    if !latest_present {
        return missing_snapshot(root, latest_path, true, "No launch status receipt yet");
    }

    let value = match read_json_receipt(&latest_path) {
        Ok(value) => value,
        Err(error) => {
            return invalid_snapshot(root, latest_path, true, error);
        }
    };

    let schema_valid = string_field(&value, "schema_version") == Some(DX_LAUNCH_STATUS_SCHEMA);
    if !schema_valid {
        let schema = string_field(&value, "schema_version").unwrap_or("missing");
        return invalid_snapshot(
            root,
            latest_path,
            true,
            format!("Unexpected launch status schema: {schema}"),
        );
    }

    DxLaunchStatusSnapshot {
        root,
        latest_path,
        root_exists,
        latest_present,
        schema_valid,
        status: string_field(&value, "status")
            .unwrap_or("unknown")
            .to_string(),
        operator_summary: string_field(&value, "operator_summary")
            .unwrap_or("Launch status receipt is present.")
            .to_string(),
        agents: agents_summary(&value),
        tokens: tokens_summary(&value),
        discovery: discovery_summary(&value),
        last_error: string_field(&value, "last_error").map(ToString::to_string),
        next_action: string_field(&value, "next_action")
            .unwrap_or("review_launch_status")
            .to_string(),
        redaction_requires_review: redaction_requires_review(&value),
        redaction_summary: value
            .pointer("/redaction/detail")
            .and_then(Value::as_str)
            .unwrap_or("No redaction detail is present in the launch status receipt.")
            .to_string(),
    }
}

fn missing_snapshot(
    root: PathBuf,
    latest_path: PathBuf,
    root_exists: bool,
    status: &str,
) -> DxLaunchStatusSnapshot {
    DxLaunchStatusSnapshot {
        root,
        latest_path,
        root_exists,
        latest_present: false,
        schema_valid: false,
        status: status.to_string(),
        operator_summary: format!(
            "Run `{DX_LAUNCH_STATUS_COMMAND}` to create launch readiness metadata."
        ),
        agents: DxLaunchAgentsSummary::empty(),
        tokens: DxLaunchTokensSummary::empty(),
        discovery: DxLaunchDiscoverySummary::empty(),
        last_error: None,
        next_action: DX_LAUNCH_STATUS_COMMAND.to_string(),
        redaction_requires_review: false,
        redaction_summary: "No launch status receipt has been read yet.".to_string(),
    }
}

fn invalid_snapshot(
    root: PathBuf,
    latest_path: PathBuf,
    latest_present: bool,
    error: String,
) -> DxLaunchStatusSnapshot {
    DxLaunchStatusSnapshot {
        root,
        latest_path,
        root_exists: true,
        latest_present,
        schema_valid: false,
        status: "Invalid receipt".to_string(),
        operator_summary: error.clone(),
        agents: DxLaunchAgentsSummary::empty(),
        tokens: DxLaunchTokensSummary::empty(),
        discovery: DxLaunchDiscoverySummary::empty(),
        last_error: Some(error),
        next_action: DX_LAUNCH_STATUS_COMMAND.to_string(),
        redaction_requires_review: true,
        redaction_summary: "Launch status receipt could not be validated.".to_string(),
    }
}

impl DxLaunchAgentsSummary {
    fn empty() -> Self {
        Self {
            status: "unknown".to_string(),
            configured_accounts: 0,
            connected_accounts: 0,
            accounts_needing_connection: 0,
            qr_connect_supported: 0,
            automation_count: 0,
            active_task_count: 0,
            next_action: "run_launch_status".to_string(),
        }
    }
}

impl DxLaunchTokensSummary {
    fn empty() -> Self {
        Self {
            status: "unknown".to_string(),
            budget_state: "unknown".to_string(),
            estimated_tokens: 0,
            soft_budget_tokens: 0,
            hard_budget_tokens: 0,
            next_action: "run_launch_status".to_string(),
        }
    }
}

impl DxLaunchDiscoverySummary {
    fn empty() -> Self {
        Self {
            status: "unknown".to_string(),
            templates_command: "dx www templates --json".to_string(),
            packages_command: "dx forge packages --json".to_string(),
            www_manifest_present: false,
            configured_binary_present: false,
            next_action: "run_launch_status".to_string(),
        }
    }
}

fn agents_summary(value: &Value) -> DxLaunchAgentsSummary {
    DxLaunchAgentsSummary {
        status: pointer_string(value, "/agents/status", "unknown"),
        configured_accounts: pointer_usize(value, "/agents/connected_accounts_configured"),
        connected_accounts: pointer_usize(value, "/agents/connected_accounts_connected"),
        accounts_needing_connection: pointer_usize(
            value,
            "/agents/connected_accounts_needs_connection",
        ),
        qr_connect_supported: pointer_usize(value, "/agents/qr_connect_supported"),
        automation_count: pointer_usize(value, "/agents/automation_count"),
        active_task_count: pointer_usize(value, "/agents/active_task_count"),
        next_action: pointer_string(value, "/agents/next_action", "review_agent_status"),
    }
}

fn tokens_summary(value: &Value) -> DxLaunchTokensSummary {
    DxLaunchTokensSummary {
        status: pointer_string(value, "/tokens/status", "unknown"),
        budget_state: pointer_string(value, "/tokens/budget_state", "unknown"),
        estimated_tokens: pointer_u64(value, "/tokens/estimated_tokens"),
        soft_budget_tokens: pointer_u64(value, "/tokens/soft_budget_tokens"),
        hard_budget_tokens: pointer_u64(value, "/tokens/hard_budget_tokens"),
        next_action: pointer_string(value, "/tokens/next_action", "review_token_budget"),
    }
}

fn discovery_summary(value: &Value) -> DxLaunchDiscoverySummary {
    DxLaunchDiscoverySummary {
        status: pointer_string(value, "/discovery/status", "unknown"),
        templates_command: pointer_string(
            value,
            "/discovery/templates_command",
            "dx www templates --json",
        ),
        packages_command: pointer_string(
            value,
            "/discovery/packages_command",
            "dx forge packages --json",
        ),
        www_manifest_present: pointer_bool(value, "/discovery/www_manifest_present"),
        configured_binary_present: pointer_bool(value, "/discovery/configured_binary_present"),
        next_action: pointer_string(value, "/discovery/next_action", "review_discovery_bridge"),
    }
}

fn read_json_receipt(path: &Path) -> Result<Value, String> {
    let metadata = path
        .metadata()
        .map_err(|error| format!("Unable to inspect launch status receipt: {error}"))?;
    if metadata.len() > MAX_RECEIPT_BYTES {
        return Err(format!(
            "Launch status receipt is too large to render safely: {} bytes",
            metadata.len()
        ));
    }

    let mut contents = String::new();
    File::open(path)
        .and_then(|mut file| file.read_to_string(&mut contents))
        .map_err(|error| format!("Unable to read launch status receipt: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Unable to parse launch status receipt: {error}"))
}

fn redaction_requires_review(value: &Value) -> bool {
    let Some(redaction) = value.get("redaction") else {
        return true;
    };

    [
        "exports_source_file_contents",
        "exports_source_file_paths",
        "exports_secret_values",
        "exports_receipt_bodies",
        "exports_prompts",
        "exports_transcripts",
        "exports_command_payloads",
    ]
    .into_iter()
    .any(|field| {
        redaction
            .get(field)
            .and_then(Value::as_bool)
            .unwrap_or(true)
    })
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn pointer_string(value: &Value, pointer: &str, fallback: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn pointer_bool(value: &Value, pointer: &str) -> bool {
    value
        .pointer(pointer)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn pointer_u64(value: &Value, pointer: &str) -> u64 {
    value.pointer(pointer).and_then(Value::as_u64).unwrap_or(0)
}

fn pointer_usize(value: &Value, pointer: &str) -> usize {
    pointer_u64(value, pointer).try_into().unwrap_or(usize::MAX)
}
