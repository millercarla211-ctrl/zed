use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime},
};

const PROOF_FRESHNESS_CACHE_TTL: Duration = Duration::from_secs(5);
const FRESH_PROOF_WINDOW: Duration = Duration::from_secs(24 * 60 * 60);
const STALE_PROOF_WINDOW: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Clone)]
pub(crate) struct DxProofFreshnessSnapshot {
    pub buckets: Vec<DxProofFreshnessBucket>,
}

#[derive(Clone)]
pub(crate) struct DxProofFreshnessBucket {
    pub label: &'static str,
    pub root_label: &'static str,
    pub root_exists: bool,
    pub count: usize,
    pub status: String,
    pub latest: Vec<String>,
}

impl DxProofFreshnessSnapshot {
    pub(crate) fn receipt_count(&self, label: &str) -> usize {
        self.buckets
            .iter()
            .find(|bucket| bucket.label == label)
            .map(|bucket| bucket.count)
            .unwrap_or_default()
    }

    pub(crate) fn fresh_receipt_count(&self) -> usize {
        self.buckets
            .iter()
            .filter(|bucket| bucket.label != "Runtime Plan")
            .filter(|bucket| bucket.status == "Fresh")
            .map(|bucket| bucket.count)
            .sum()
    }
}

static PROOF_FRESHNESS_CACHE: OnceLock<
    Mutex<Option<(Instant, Vec<String>, DxProofFreshnessSnapshot)>>,
> = OnceLock::new();

pub(crate) fn proof_freshness_snapshot(workspace_roots: &[String]) -> DxProofFreshnessSnapshot {
    let cache = PROOF_FRESHNESS_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, cached_roots, snapshot)) = cache.as_ref() {
            if cached_roots == workspace_roots
                && now.duration_since(*cached_at) <= PROOF_FRESHNESS_CACHE_TTL
            {
                return snapshot.clone();
            }
        }

        let snapshot = scan_proof_freshness(workspace_roots);
        *cache = Some((now, workspace_roots.to_vec(), snapshot.clone()));
        return snapshot;
    }

    scan_proof_freshness(workspace_roots)
}

fn scan_proof_freshness(workspace_roots: &[String]) -> DxProofFreshnessSnapshot {
    let workspace_roots = workspace_roots
        .iter()
        .take(4)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let buckets = vec![
        proof_bucket(
            "Validation",
            "tools/agent-plugins/browser-final-validation",
            &[
                "tools/agent-plugins/browser-final-validation",
                "tools/dx-validation",
                "tools/dx-check/validation",
            ],
            &workspace_roots,
        ),
        proof_bucket(
            "Visual Proof",
            "tools/agent-plugins/browser-panel-control-results",
            &[
                "tools/agent-plugins/browser-panel-control-results",
                "tools/dx-visual-proof",
                "tools/dx-visual-proofs",
            ],
            &workspace_roots,
        ),
        proof_bucket(
            "Runtime Plan",
            "tools/dx-runtime-proof/plans",
            &["tools/dx-runtime-proof/plans"],
            &workspace_roots,
        ),
        proof_bucket(
            "Runtime Proof",
            "tools/dx-runtime-proof/imports",
            &[
                "tools/dx-runtime-proof/imports",
                "tools/dx-runtime-proof/status",
                "tools/agent-plugins/runtime-green",
                "tools/agent-plugins/runtime-status",
            ],
            &workspace_roots,
        ),
    ];

    DxProofFreshnessSnapshot { buckets }
}

fn proof_bucket(
    label: &'static str,
    root_label: &'static str,
    relative_roots: &[&'static str],
    workspace_roots: &[PathBuf],
) -> DxProofFreshnessBucket {
    let mut root_exists = false;
    let mut count = 0;
    let mut latest = Vec::new();

    for workspace_root in workspace_roots {
        for relative_root in relative_roots {
            let root = relative_path(workspace_root, relative_root);
            if root.is_dir() {
                root_exists = true;
            }
            count += count_receipt_files(&root);
            latest.extend(latest_receipt_labels(workspace_root, &root, 2));
        }
    }

    latest.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(Ordering::Equal));
    latest.truncate(2);
    let newest = latest.first().map(|(modified, _)| *modified);

    DxProofFreshnessBucket {
        label,
        root_label,
        root_exists,
        count,
        status: proof_status(root_exists, count, newest),
        latest: latest.into_iter().map(|(_, label)| label).collect(),
    }
}

fn proof_status(root_exists: bool, count: usize, newest: Option<SystemTime>) -> String {
    if !root_exists {
        return "Missing".to_string();
    }

    if count == 0 {
        return "No receipts".to_string();
    }

    let Some(newest) = newest else {
        return "No timestamp".to_string();
    };

    match SystemTime::now().duration_since(newest) {
        Ok(age) if age <= FRESH_PROOF_WINDOW => "Fresh".to_string(),
        Ok(age) if age <= STALE_PROOF_WINDOW => "Stale".to_string(),
        Ok(_) => "Old".to_string(),
        Err(_) => "Fresh".to_string(),
    }
}

fn relative_path(root: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .fold(root.to_path_buf(), |path, segment| path.join(segment))
}

fn count_receipt_files(root: &Path) -> usize {
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };

    entries
        .flatten()
        .take(128)
        .map(|entry| {
            let path = entry.path();
            if path.is_file() {
                usize::from(is_receipt_file(&path))
            } else if path.is_dir() {
                fs::read_dir(path)
                    .map(|entries| {
                        entries
                            .flatten()
                            .take(64)
                            .filter(|entry| {
                                let path = entry.path();
                                path.is_file() && is_receipt_file(&path)
                            })
                            .count()
                    })
                    .unwrap_or_default()
            } else {
                0
            }
        })
        .sum()
}

fn latest_receipt_labels(
    workspace_root: &Path,
    receipt_root: &Path,
    limit: usize,
) -> Vec<(SystemTime, String)> {
    let Ok(entries) = fs::read_dir(receipt_root) else {
        return Vec::new();
    };

    let mut receipts = Vec::new();
    for entry in entries.flatten().take(64) {
        let path = entry.path();
        if path.is_file() {
            push_receipt_label(workspace_root, &path, &mut receipts);
        } else if let Ok(children) = fs::read_dir(&path) {
            for child in children.flatten().take(64) {
                push_receipt_label(workspace_root, &child.path(), &mut receipts);
            }
        }
    }

    receipts.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(Ordering::Equal));
    receipts.truncate(limit);
    receipts
}

fn push_receipt_label(
    workspace_root: &Path,
    path: &Path,
    receipts: &mut Vec<(SystemTime, String)>,
) {
    if !path.is_file() || !is_receipt_file(path) {
        return;
    }

    let modified = path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let label = path
        .strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string();
    receipts.push((modified, label));
}

fn is_receipt_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "jsonl" | "receipt")
    )
}
