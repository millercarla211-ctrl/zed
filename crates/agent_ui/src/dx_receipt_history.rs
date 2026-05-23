mod buckets;
mod fields;
mod forge_history;
mod forge_receipt_fields;
mod receipt_files;
mod receipt_io;

use self::buckets::scan_tool_history;
use std::{
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const TOOL_HISTORY_CACHE_TTL: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(crate) struct DxToolHistoryBucket {
    pub label: &'static str,
    pub root_label: String,
    pub root_exists: bool,
    pub count: usize,
    pub latest: Vec<String>,
    pub latest_summaries: Vec<DxToolHistoryReceiptSummary>,
}

#[derive(Clone)]
pub(crate) struct DxToolHistoryReceiptSummary {
    pub label: String,
    pub kind: String,
    pub headline: String,
    pub detail: String,
    pub target_path: Option<String>,
    pub restore_destination_root: Option<String>,
    pub blocker_count: usize,
}

#[derive(Clone)]
pub(crate) struct DxToolHistorySnapshot {
    pub buckets: Vec<DxToolHistoryBucket>,
}

static TOOL_HISTORY_CACHE: OnceLock<Mutex<Option<(Instant, Vec<String>, DxToolHistorySnapshot)>>> =
    OnceLock::new();

pub(crate) fn tool_history_snapshot(workspace_roots: &[String]) -> DxToolHistorySnapshot {
    let cache = TOOL_HISTORY_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, cached_roots, snapshot)) = cache.as_ref() {
            if cached_roots == workspace_roots
                && now.duration_since(*cached_at) <= TOOL_HISTORY_CACHE_TTL
            {
                return snapshot.clone();
            }
        }

        let snapshot = scan_tool_history(workspace_roots);
        *cache = Some((now, workspace_roots.to_vec(), snapshot.clone()));
        return snapshot;
    }

    scan_tool_history(workspace_roots)
}
