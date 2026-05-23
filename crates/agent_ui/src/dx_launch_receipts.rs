mod fields;
mod freshness;
mod paths;
mod receipt_io;
mod summary;

use self::freshness::launch_receipt_operator_summary;
use self::paths::{launch_snapshot_paths, now_ms};
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const DX_LAUNCH_RECEIPT_ROOT: &str = r"G:\Dx\.dx\receipts\launch";
const DX_LAUNCH_RECEIPT_REVIEW_SCHEMA: &str = "dx.launch.receipts.v1";
const DX_LAUNCH_RECEIPTS_COMMAND: &str = "dx launch receipts --json";
const DX_LAUNCH_STATUS_LATEST: &str = "status-latest.json";
const DX_LAUNCH_STATUS_PREFIX: &str = "launch-status-";
const DX_LAUNCH_STATUS_COMMAND: &str = "dx launch status --json";
const DX_LAUNCH_STATUS_SCHEMA: &str = "dx.launch.status.v1";
const LAUNCH_RECEIPTS_CACHE_TTL: Duration = Duration::from_secs(5);
const RECEIPT_STALE_AFTER_MS: u64 = 300_000;
const RECEIPT_EXPIRED_AFTER_MS: u64 = 1_800_000;

#[derive(Clone)]
pub(crate) struct DxLaunchReceiptReviewSnapshot {
    pub schema_version: String,
    pub command: String,
    pub root: PathBuf,
    pub latest_path: PathBuf,
    pub root_exists: bool,
    pub latest_present: bool,
    pub status: String,
    pub operator_summary: String,
    pub latest: Option<DxLaunchReceiptSummary>,
    pub snapshots: Vec<DxLaunchReceiptSummary>,
    pub snapshot_count: usize,
    pub malformed_count: usize,
    pub stale_count: usize,
    pub expired_count: usize,
    pub stale_after_ms: u64,
    pub expired_after_ms: u64,
    pub last_error: Option<String>,
    pub next_action: String,
}

#[derive(Clone)]
pub(crate) struct DxLaunchReceiptSummary {
    pub kind: String,
    pub file_name: String,
    pub receipt_path: String,
    pub schema_version: Option<String>,
    pub status: Option<String>,
    pub generated_at_ms: Option<u64>,
    pub age_ms: Option<u64>,
    pub freshness_state: String,
    pub malformed: bool,
    pub last_error: Option<String>,
    pub next_action: Option<String>,
}

static LAUNCH_RECEIPTS_CACHE: OnceLock<Mutex<Option<(Instant, DxLaunchReceiptReviewSnapshot)>>> =
    OnceLock::new();

pub(crate) fn launch_receipt_review_snapshot() -> DxLaunchReceiptReviewSnapshot {
    let cache = LAUNCH_RECEIPTS_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, snapshot)) = cache.as_ref() {
            if now.duration_since(*cached_at) <= LAUNCH_RECEIPTS_CACHE_TTL {
                return snapshot.clone();
            }
        }

        let snapshot = scan_launch_receipts();
        *cache = Some((now, snapshot.clone()));
        return snapshot;
    }

    scan_launch_receipts()
}

fn scan_launch_receipts() -> DxLaunchReceiptReviewSnapshot {
    let generated_at_ms = now_ms();
    let root = PathBuf::from(DX_LAUNCH_RECEIPT_ROOT);
    let latest_path = root.join(DX_LAUNCH_STATUS_LATEST);
    let root_exists = root.is_dir();

    if !root_exists {
        return empty_snapshot(
            root,
            latest_path,
            false,
            "missing",
            format!("Launch receipt directory is missing; run `{DX_LAUNCH_STATUS_COMMAND}`."),
            DX_LAUNCH_STATUS_COMMAND,
        );
    }

    let latest = latest_path
        .is_file()
        .then(|| DxLaunchReceiptSummary::from_path("latest", &latest_path, generated_at_ms));
    let mut snapshots = launch_snapshot_paths(&root)
        .into_iter()
        .map(|path| DxLaunchReceiptSummary::from_path("snapshot", &path, generated_at_ms))
        .collect::<Vec<_>>();

    snapshots.sort_by(|left, right| {
        right
            .generated_at_ms
            .unwrap_or_default()
            .cmp(&left.generated_at_ms.unwrap_or_default())
            .then_with(|| right.file_name.cmp(&left.file_name))
    });

    let latest_present = latest.is_some();
    let malformed_count = latest
        .iter()
        .chain(snapshots.iter())
        .filter(|entry| entry.malformed)
        .count();
    let stale_count = latest
        .iter()
        .chain(snapshots.iter())
        .filter(|entry| entry.freshness_state == "stale")
        .count();
    let expired_count = latest
        .iter()
        .chain(snapshots.iter())
        .filter(|entry| entry.freshness_state == "expired")
        .count();
    let latest_freshness = latest
        .as_ref()
        .map(|latest| latest.freshness_state.as_str());
    let latest_schema_matches = latest
        .as_ref()
        .is_some_and(DxLaunchReceiptSummary::schema_matches_launch_status);
    let last_error = if latest.is_none() {
        Some("missing launch latest receipt".to_string())
    } else {
        latest
            .iter()
            .chain(snapshots.iter())
            .find_map(|entry| entry.last_error.clone())
            .or_else(|| {
                (!latest_schema_matches)
                    .then(|| "launch latest receipt has unexpected schema version".to_string())
            })
            .or_else(|| match latest_freshness {
                Some("expired") => Some("launch latest receipt is expired".to_string()),
                Some("stale") => Some("launch latest receipt is stale".to_string()),
                _ => None,
            })
    };
    let status = if last_error.is_some() || malformed_count > 0 {
        "warning"
    } else {
        "ready"
    };
    let next_action = if status == "ready" {
        "launch_receipts_ready_for_zed_review"
    } else {
        "review_launch_receipt_warnings"
    };
    let operator_summary = launch_receipt_operator_summary(
        status,
        malformed_count,
        latest_present,
        snapshots.len(),
        latest_freshness,
    );

    DxLaunchReceiptReviewSnapshot {
        schema_version: DX_LAUNCH_RECEIPT_REVIEW_SCHEMA.to_string(),
        command: DX_LAUNCH_RECEIPTS_COMMAND.to_string(),
        root,
        latest_path,
        root_exists,
        latest_present,
        status: status.to_string(),
        operator_summary,
        snapshot_count: snapshots.len(),
        malformed_count,
        stale_count,
        expired_count,
        stale_after_ms: RECEIPT_STALE_AFTER_MS,
        expired_after_ms: RECEIPT_EXPIRED_AFTER_MS,
        latest,
        snapshots,
        last_error,
        next_action: next_action.to_string(),
    }
}

fn empty_snapshot(
    root: PathBuf,
    latest_path: PathBuf,
    root_exists: bool,
    status: &str,
    operator_summary: String,
    next_action: &str,
) -> DxLaunchReceiptReviewSnapshot {
    DxLaunchReceiptReviewSnapshot {
        schema_version: DX_LAUNCH_RECEIPT_REVIEW_SCHEMA.to_string(),
        command: DX_LAUNCH_RECEIPTS_COMMAND.to_string(),
        root,
        latest_path,
        root_exists,
        latest_present: false,
        status: status.to_string(),
        operator_summary,
        latest: None,
        snapshots: Vec::new(),
        snapshot_count: 0,
        malformed_count: 0,
        stale_count: 0,
        expired_count: 0,
        stale_after_ms: RECEIPT_STALE_AFTER_MS,
        expired_after_ms: RECEIPT_EXPIRED_AFTER_MS,
        last_error: None,
        next_action: next_action.to_string(),
    }
}
