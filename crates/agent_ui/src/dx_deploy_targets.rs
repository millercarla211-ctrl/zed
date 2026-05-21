use serde_json::Value;
use std::{
    cmp::Ordering,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime},
};

const DEPLOY_TARGET_CACHE_TTL: Duration = Duration::from_secs(5);
const MAX_CONFIG_BYTES: u64 = 64 * 1024;

#[derive(Clone)]
pub(crate) struct DxDeployTargetSnapshot {
    pub targets: Vec<DxDeployTarget>,
    pub workspace_root_count: usize,
    pub receipt_root_exists: bool,
    pub receipt_count: usize,
    pub latest_receipts: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct DxDeployTarget {
    pub label: String,
    pub platform: &'static str,
    pub detail: String,
    pub path: String,
}

static DEPLOY_TARGET_CACHE: OnceLock<
    Mutex<Option<(Instant, Vec<String>, DxDeployTargetSnapshot)>>,
> = OnceLock::new();

pub(crate) fn deploy_target_snapshot(workspace_roots: &[String]) -> DxDeployTargetSnapshot {
    let cache = DEPLOY_TARGET_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, cached_roots, snapshot)) = cache.as_ref() {
            if cached_roots == workspace_roots
                && now.duration_since(*cached_at) <= DEPLOY_TARGET_CACHE_TTL
            {
                return snapshot.clone();
            }
        }

        let snapshot = scan_deploy_targets(workspace_roots);
        *cache = Some((now, workspace_roots.to_vec(), snapshot.clone()));
        return snapshot;
    }

    scan_deploy_targets(workspace_roots)
}

fn scan_deploy_targets(workspace_roots: &[String]) -> DxDeployTargetSnapshot {
    let workspace_roots = workspace_roots
        .iter()
        .take(4)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let mut targets = Vec::new();

    for root in &workspace_roots {
        push_vercel_project_target(root, &mut targets);
        push_file_target(
            root,
            &["vercel.json"],
            "Vercel",
            "Vercel config",
            "Project-level Vercel configuration",
            &mut targets,
        );
        push_file_target(
            root,
            &["netlify.toml"],
            "Netlify",
            "Netlify site",
            "Netlify build and deploy configuration",
            &mut targets,
        );
        push_file_target(
            root,
            &["wrangler.toml"],
            "Cloudflare",
            "Cloudflare Worker",
            "Wrangler deploy configuration",
            &mut targets,
        );
        push_file_target(
            root,
            &["fly.toml"],
            "Fly.io",
            "Fly app",
            "Fly deploy configuration",
            &mut targets,
        );
        push_file_target(
            root,
            &["Dockerfile"],
            "Docker",
            "Container image",
            "Dockerfile build target",
            &mut targets,
        );
    }

    targets.truncate(6);
    let (receipt_root_exists, receipt_count, latest_receipts) =
        scan_deploy_receipts(&workspace_roots);

    DxDeployTargetSnapshot {
        targets,
        workspace_root_count: workspace_roots.len(),
        receipt_root_exists,
        receipt_count,
        latest_receipts,
    }
}

fn scan_deploy_receipts(workspace_roots: &[PathBuf]) -> (bool, usize, Vec<String>) {
    let mut root_exists = false;
    let mut count = 0;
    let mut latest = Vec::new();

    for root in workspace_roots {
        let receipt_root = root.join("tools").join("dx-deploy");
        if receipt_root.is_dir() {
            root_exists = true;
        }
        count += count_receipt_files(&receipt_root);
        latest.extend(latest_receipt_labels(root, &receipt_root, 4));
    }

    latest.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(Ordering::Equal));
    latest.truncate(4);

    (
        root_exists,
        count,
        latest.into_iter().map(|(_, label)| label).collect(),
    )
}

fn push_vercel_project_target(root: &Path, targets: &mut Vec<DxDeployTarget>) {
    let path = root.join(".vercel").join("project.json");
    if !path.is_file() {
        return;
    }

    let detail = read_json(&path)
        .and_then(|value| {
            let project_id = value.get("projectId").and_then(Value::as_str)?;
            let org_id = value
                .get("orgId")
                .and_then(Value::as_str)
                .unwrap_or("unknown org");
            Some(format!("{project_id} - {org_id}"))
        })
        .unwrap_or_else(|| "Linked Vercel project".to_string());

    targets.push(DxDeployTarget {
        label: format!("Vercel: {}", display_name(root)),
        platform: "Vercel",
        detail,
        path: relative_label(root, &path),
    });
}

fn push_file_target(
    root: &Path,
    relative_path: &[&str],
    platform: &'static str,
    label: &'static str,
    detail: &'static str,
    targets: &mut Vec<DxDeployTarget>,
) {
    let path = relative_path
        .iter()
        .fold(root.to_path_buf(), |path, segment| path.join(segment));
    if !path.is_file() {
        return;
    }

    targets.push(DxDeployTarget {
        label: format!("{label}: {}", display_name(root)),
        platform,
        detail: detail.to_string(),
        path: relative_label(root, &path),
    });
}

fn read_json(path: &Path) -> Option<Value> {
    let mut file = File::open(path).ok()?;
    let mut buffer = Vec::new();
    file.by_ref()
        .take(MAX_CONFIG_BYTES)
        .read_to_end(&mut buffer)
        .ok()?;
    serde_json::from_slice(&buffer).ok()
}

fn count_receipt_files(path: &Path) -> usize {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };

    entries
        .flatten()
        .take(128)
        .map(|entry| {
            let path = entry.path();
            if path.is_file() && is_receipt_file(&path) {
                1
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
    let label = relative_label(workspace_root, path);
    receipts.push((modified, label));
}

fn is_receipt_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "jsonl" | "receipt")
    )
}

fn relative_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}
