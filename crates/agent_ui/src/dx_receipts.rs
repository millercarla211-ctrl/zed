use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime},
};

const DX_RECEIPTS_ROOT: &str = r"G:\Dx\.dx\receipts";
const RECEIPT_CACHE_TTL: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(crate) struct DxReceiptBucket {
    pub label: &'static str,
    pub count: usize,
}

#[derive(Clone)]
pub(crate) struct DxReceiptSnapshot {
    pub root: PathBuf,
    pub root_exists: bool,
    pub buckets: Vec<DxReceiptBucket>,
    pub latest: Vec<String>,
}

static RECEIPT_CACHE: OnceLock<Mutex<Option<(Instant, DxReceiptSnapshot)>>> = OnceLock::new();

pub(crate) fn receipt_snapshot() -> DxReceiptSnapshot {
    let cache = RECEIPT_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, snapshot)) = cache.as_ref() {
            if now.duration_since(*cached_at) <= RECEIPT_CACHE_TTL {
                return snapshot.clone();
            }
        }

        let snapshot = scan_receipts_root();
        *cache = Some((now, snapshot.clone()));
        return snapshot;
    }

    scan_receipts_root()
}

fn scan_receipts_root() -> DxReceiptSnapshot {
    let root = PathBuf::from(DX_RECEIPTS_ROOT);
    let root_exists = root.is_dir();

    let buckets = [
        ("Agents", "agents"),
        ("Launch", "launch"),
        ("Tokens", "tokens"),
        ("Forge", "forge"),
        ("Sources", "metasearch"),
        ("Media", "media"),
        ("RLM", "rlm"),
        ("Serializer", "serializer"),
    ]
    .into_iter()
    .map(|(label, child)| DxReceiptBucket {
        label,
        count: count_receipt_files(&root.join(child)),
    })
    .collect();

    DxReceiptSnapshot {
        latest: latest_receipt_labels(&root, root_exists),
        root,
        root_exists,
        buckets,
    }
}

fn count_receipt_files(path: &Path) -> usize {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };

    let mut count = 0;
    for entry in entries.flatten().take(128) {
        let path = entry.path();
        if path.is_file() {
            if is_receipt_file(&path) {
                count += 1;
            }
        } else if path.is_dir() {
            count += fs::read_dir(path)
                .map(|entries| {
                    entries
                        .flatten()
                        .take(32)
                        .filter(|entry| entry.path().is_file() && is_receipt_file(&entry.path()))
                        .count()
                })
                .unwrap_or_default();
        }
    }
    count
}

fn latest_receipt_labels(root: &Path, root_exists: bool) -> Vec<String> {
    if !root_exists {
        return Vec::new();
    }

    let mut receipts = Vec::new();
    let Ok(children) = fs::read_dir(root) else {
        return receipts;
    };

    for child in children.flatten().take(24) {
        let child_path = child.path();
        if child_path.is_file() {
            push_receipt_label(root, &child_path, &mut receipts);
        } else if let Ok(entries) = fs::read_dir(&child_path) {
            for entry in entries.flatten().take(24) {
                let path = entry.path();
                if path.is_file() {
                    push_receipt_label(root, &path, &mut receipts);
                }
            }
        }
    }

    receipts.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(Ordering::Equal));
    receipts
        .into_iter()
        .take(4)
        .map(|(_, label)| label)
        .collect()
}

fn push_receipt_label(root: &Path, path: &Path, receipts: &mut Vec<(SystemTime, String)>) {
    if !is_receipt_file(path) {
        return;
    }

    let modified = path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let label = path
        .strip_prefix(root)
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
