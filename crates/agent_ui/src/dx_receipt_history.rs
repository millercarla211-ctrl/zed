use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime},
};

const TOOL_HISTORY_CACHE_TTL: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(crate) struct DxToolHistoryBucket {
    pub label: &'static str,
    pub root_label: String,
    pub root_exists: bool,
    pub count: usize,
    pub latest: Vec<String>,
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

fn scan_tool_history(workspace_roots: &[String]) -> DxToolHistorySnapshot {
    let workspace_roots = workspace_roots
        .iter()
        .take(4)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let buckets = [
        ("Forge History", Path::new("tools").join("dx-forge")),
        (
            "Media Executions",
            Path::new("tools").join("dx-media").join("executions"),
        ),
    ]
    .into_iter()
    .map(|(label, relative_root)| scan_bucket(label, &relative_root, &workspace_roots))
    .collect();

    DxToolHistorySnapshot { buckets }
}

fn scan_bucket(
    label: &'static str,
    relative_root: &Path,
    workspace_roots: &[PathBuf],
) -> DxToolHistoryBucket {
    if workspace_roots.is_empty() {
        return DxToolHistoryBucket {
            label,
            root_label: "No workspace".to_string(),
            root_exists: false,
            count: 0,
            latest: Vec::new(),
        };
    }

    let mut count = 0;
    let mut latest = Vec::new();
    let mut root_exists = false;

    for workspace_root in workspace_roots {
        let root = workspace_root.join(relative_root);
        if !root.is_dir() {
            continue;
        }

        root_exists = true;
        count += count_receipt_files(&root);
        push_latest_receipts(workspace_root, &root, &mut latest);
    }

    latest.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(Ordering::Equal));

    DxToolHistoryBucket {
        label,
        root_label: root_label(relative_root, workspace_roots),
        root_exists,
        count,
        latest: latest.into_iter().take(3).map(|(_, label)| label).collect(),
    }
}

fn root_label(relative_root: &Path, workspace_roots: &[PathBuf]) -> String {
    if workspace_roots.len() == 1 {
        return workspace_roots[0].join(relative_root).display().to_string();
    }

    format!("{} workspaces", workspace_roots.len())
}

fn count_receipt_files(root: &Path) -> usize {
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };

    entries
        .flatten()
        .take(192)
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
                                entry.path().is_file() && is_receipt_file(&entry.path())
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

fn push_latest_receipts(
    workspace_root: &Path,
    root: &Path,
    receipts: &mut Vec<(SystemTime, String)>,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten().take(64) {
        let path = entry.path();
        if path.is_file() {
            push_receipt_label(workspace_root, &path, receipts);
        } else if path.is_dir() {
            let Ok(children) = fs::read_dir(path) else {
                continue;
            };
            for child in children.flatten().take(64) {
                let path = child.path();
                if path.is_file() {
                    push_receipt_label(workspace_root, &path, receipts);
                }
            }
        }
    }
}

fn push_receipt_label(
    workspace_root: &Path,
    path: &Path,
    receipts: &mut Vec<(SystemTime, String)>,
) {
    if !is_receipt_file(path) {
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
