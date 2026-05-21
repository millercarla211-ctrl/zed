use serde_json::Value;
use std::{
    cmp::Ordering,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime},
};

const RUNTIME_PROOF_STATUS_CACHE_TTL: Duration = Duration::from_secs(5);
const MAX_RECEIPT_BYTES: u64 = 128 * 1024;

#[derive(Clone)]
pub(crate) struct DxRuntimeProofStatusSnapshot {
    pub workspace_root_count: usize,
    pub import_root_exists: bool,
    pub status_root_exists: bool,
    pub import_receipt_count: usize,
    pub status_receipt_count: usize,
    pub latest_import: Option<DxRuntimeProofReceiptSummary>,
    pub latest_status: Option<DxRuntimeProofReceiptSummary>,
    pub claim_state: String,
    pub blockers: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct DxRuntimeProofReceiptSummary {
    pub label: String,
    pub operator_status: String,
    pub validation_status: String,
    pub runtime_green_candidate: bool,
    pub can_claim_runtime_green: bool,
    pub evidence_count: usize,
    pub blocker_count: usize,
    pub headline: Option<String>,
    pub blockers: Vec<String>,
}

impl DxRuntimeProofStatusSnapshot {
    pub(crate) fn runtime_green_candidate(&self) -> bool {
        self.latest_import
            .as_ref()
            .map(|receipt| receipt.runtime_green_candidate)
            .unwrap_or(false)
            && self
                .latest_status
                .as_ref()
                .map(|receipt| receipt.can_claim_runtime_green)
                .unwrap_or(false)
    }
}

static RUNTIME_PROOF_STATUS_CACHE: OnceLock<
    Mutex<Option<(Instant, Vec<String>, DxRuntimeProofStatusSnapshot)>>,
> = OnceLock::new();

pub(crate) fn runtime_proof_status_snapshot(
    workspace_roots: &[String],
) -> DxRuntimeProofStatusSnapshot {
    let cache = RUNTIME_PROOF_STATUS_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, cached_roots, snapshot)) = cache.as_ref() {
            if cached_roots == workspace_roots
                && now.duration_since(*cached_at) <= RUNTIME_PROOF_STATUS_CACHE_TTL
            {
                return snapshot.clone();
            }
        }

        let snapshot = scan_runtime_proof_status(workspace_roots);
        *cache = Some((now, workspace_roots.to_vec(), snapshot.clone()));
        return snapshot;
    }

    scan_runtime_proof_status(workspace_roots)
}

fn scan_runtime_proof_status(workspace_roots: &[String]) -> DxRuntimeProofStatusSnapshot {
    let workspace_roots = workspace_roots
        .iter()
        .take(4)
        .map(PathBuf::from)
        .collect::<Vec<_>>();

    let mut import_root_exists = false;
    let mut status_root_exists = false;
    let mut import_receipt_count = 0;
    let mut status_receipt_count = 0;
    let mut import_receipts = Vec::new();
    let mut status_receipts = Vec::new();

    for workspace_root in &workspace_roots {
        let runtime_root = workspace_root.join("tools").join("dx-runtime-proof");
        let import_root = runtime_root.join("imports");
        let status_root = runtime_root.join("status");

        if import_root.is_dir() {
            import_root_exists = true;
        }
        if status_root.is_dir() {
            status_root_exists = true;
        }

        import_receipt_count += count_receipt_files(&import_root);
        status_receipt_count += count_receipt_files(&status_root);
        import_receipts.extend(latest_receipt_paths(workspace_root, &import_root));
        status_receipts.extend(latest_receipt_paths(workspace_root, &status_root));
    }

    import_receipts.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(Ordering::Equal));
    status_receipts.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(Ordering::Equal));

    let latest_import = import_receipts
        .first()
        .and_then(|(_, path, label)| parse_import_summary(path, label));
    let latest_status = status_receipts
        .first()
        .and_then(|(_, path, label)| parse_status_summary(path, label));

    let (claim_state, blockers) = claim_state(
        workspace_roots.len(),
        import_root_exists,
        status_root_exists,
        import_receipt_count,
        status_receipt_count,
        latest_import.as_ref(),
        latest_status.as_ref(),
    );

    DxRuntimeProofStatusSnapshot {
        workspace_root_count: workspace_roots.len(),
        import_root_exists,
        status_root_exists,
        import_receipt_count,
        status_receipt_count,
        latest_import,
        latest_status,
        claim_state,
        blockers,
    }
}

fn claim_state(
    workspace_root_count: usize,
    import_root_exists: bool,
    status_root_exists: bool,
    import_receipt_count: usize,
    status_receipt_count: usize,
    latest_import: Option<&DxRuntimeProofReceiptSummary>,
    latest_status: Option<&DxRuntimeProofReceiptSummary>,
) -> (String, Vec<String>) {
    let mut blockers = Vec::new();

    if workspace_root_count == 0 {
        blockers.push("No workspace root is available for runtime proof receipts.".to_string());
        return ("No workspace".to_string(), blockers);
    }

    if !import_root_exists {
        blockers.push("Runtime proof import root is missing.".to_string());
    }
    if !status_root_exists {
        blockers.push("Runtime proof status root is missing.".to_string());
    }
    if import_receipt_count == 0 && status_receipt_count == 0 {
        blockers.push("No runtime proof import/status receipts are present.".to_string());
        return ("Needs operator evidence".to_string(), blockers);
    }

    let import_ready = latest_import
        .map(|receipt| receipt.runtime_green_candidate)
        .unwrap_or(false);
    let status_ready = latest_status
        .map(|receipt| receipt.can_claim_runtime_green)
        .unwrap_or(false);

    if import_ready && status_ready && blockers.is_empty() {
        return ("Runtime green candidate".to_string(), blockers);
    }

    if import_ready || status_ready {
        blockers
            .push("Runtime proof import and status receipts are not both claim-ready.".to_string());
    }

    if let Some(receipt) = latest_import {
        if receipt.evidence_count == 0 {
            blockers.push("Latest runtime import has no evidence lines.".to_string());
        }
        blockers.extend(receipt.blockers.iter().take(3).cloned());
        if receipt.validation_status == "blocked" {
            return ("Blocked by operator evidence".to_string(), blockers);
        }
        if receipt.validation_status == "failed" {
            return ("Runtime proof failed".to_string(), blockers);
        }
    }

    if let Some(receipt) = latest_status {
        blockers.extend(receipt.blockers.iter().take(3).cloned());
    }

    ("Import not claim-ready".to_string(), blockers)
}

fn parse_import_summary(path: &Path, label: &str) -> Option<DxRuntimeProofReceiptSummary> {
    let value = read_json(path)?;
    let proof = value.get("runtime_proof").unwrap_or(&value);
    let request = proof.get("request").unwrap_or(proof);
    let validation = proof.get("validation").unwrap_or(&Value::Null);
    let operator_status_copy = proof.get("operator_status_copy").unwrap_or(&Value::Null);

    Some(DxRuntimeProofReceiptSummary {
        label: label.to_string(),
        operator_status: string_at(request, "operator_status")
            .or_else(|| string_at(operator_status_copy, "operator_status"))
            .unwrap_or_else(|| "unknown".to_string()),
        validation_status: string_at(validation, "status").unwrap_or_else(|| "unknown".to_string()),
        runtime_green_candidate: bool_at(validation, "runtime_green_candidate"),
        can_claim_runtime_green: bool_at(operator_status_copy, "can_claim_runtime_green"),
        evidence_count: usize_at(validation, "evidence_count"),
        blocker_count: usize_at(validation, "blocker_count"),
        headline: string_at(operator_status_copy, "headline"),
        blockers: string_array_at(validation, "blockers"),
    })
}

fn parse_status_summary(path: &Path, label: &str) -> Option<DxRuntimeProofReceiptSummary> {
    let value = read_json(path)?;
    let status_copy = value.get("operator_status_copy").unwrap_or(&Value::Null);
    let validation = value.get("validation").unwrap_or(&Value::Null);

    Some(DxRuntimeProofReceiptSummary {
        label: label.to_string(),
        operator_status: string_at(status_copy, "operator_status").unwrap_or_else(|| {
            string_at(validation, "operator_status").unwrap_or_else(|| "unknown".to_string())
        }),
        validation_status: string_at(validation, "status").unwrap_or_else(|| "unknown".to_string()),
        runtime_green_candidate: bool_at(validation, "runtime_green_candidate"),
        can_claim_runtime_green: bool_at(status_copy, "can_claim_runtime_green"),
        evidence_count: usize_at(validation, "evidence_count"),
        blocker_count: usize_at(validation, "blocker_count"),
        headline: string_at(status_copy, "headline"),
        blockers: string_array_at(validation, "blockers"),
    })
}

fn count_receipt_files(root: &Path) -> usize {
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };

    entries
        .flatten()
        .take(128)
        .filter(|entry| {
            let path = entry.path();
            path.is_file() && is_receipt_file(&path)
        })
        .count()
}

fn latest_receipt_paths(
    workspace_root: &Path,
    receipt_root: &Path,
) -> Vec<(SystemTime, PathBuf, String)> {
    let Ok(entries) = fs::read_dir(receipt_root) else {
        return Vec::new();
    };

    entries
        .flatten()
        .take(128)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() || !is_receipt_file(&path) {
                return None;
            }
            let modified = path
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let label = path
                .strip_prefix(workspace_root)
                .unwrap_or(&path)
                .display()
                .to_string();
            Some((modified, path, label))
        })
        .collect()
}

fn read_json(path: &Path) -> Option<Value> {
    let mut file = File::open(path).ok()?;
    let mut buffer = Vec::new();
    file.by_ref()
        .take(MAX_RECEIPT_BYTES)
        .read_to_end(&mut buffer)
        .ok()?;
    serde_json::from_slice(&buffer).ok()
}

fn string_at(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn bool_at(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn usize_at(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or_default()
}

fn string_array_at(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn is_receipt_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "receipt")
    )
}
