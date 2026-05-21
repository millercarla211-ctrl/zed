use std::{
    cmp::Ordering,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime},
};

use serde_json::Value;

const TOOL_HISTORY_CACHE_TTL: Duration = Duration::from_secs(5);
const MAX_RECEIPT_BYTES: u64 = 128 * 1024;

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
        (
            "Serializer/RLM",
            Path::new("tools").join("dx-serializer-rlm"),
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
            latest_summaries: Vec::new(),
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
    let latest_summaries = if label == "Forge History" {
        latest
            .iter()
            .filter_map(|(_, path, label)| forge_receipt_summary(path, label))
            .take(3)
            .collect()
    } else {
        Vec::new()
    };

    DxToolHistoryBucket {
        label,
        root_label: root_label(relative_root, workspace_roots),
        root_exists,
        count,
        latest: latest
            .into_iter()
            .take(3)
            .map(|(_, _, label)| label)
            .collect(),
        latest_summaries,
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
                if path.file_name().and_then(|file_name| file_name.to_str()) == Some("preview") {
                    0
                } else {
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
                }
            } else {
                0
            }
        })
        .sum()
}

fn push_latest_receipts(
    workspace_root: &Path,
    root: &Path,
    receipts: &mut Vec<(SystemTime, PathBuf, String)>,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten().take(64) {
        let path = entry.path();
        if path.is_file() {
            push_receipt_label(workspace_root, &path, receipts);
        } else if path.is_dir() {
            if path.file_name().and_then(|file_name| file_name.to_str()) == Some("preview") {
                continue;
            }
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
    receipts: &mut Vec<(SystemTime, PathBuf, String)>,
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
    receipts.push((modified, path.to_path_buf(), label));
}

fn is_receipt_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "jsonl" | "receipt")
    )
}

fn forge_receipt_summary(path: &Path, label: &str) -> Option<DxToolHistoryReceiptSummary> {
    let value = read_json(path)?;
    let schema = string_field(&value, &["schema"]).unwrap_or_else(|| "unknown".to_string());
    let kind = forge_history_kind(&schema, &value)?;
    let headline = forge_history_headline(kind);
    let status = forge_history_status(&value);
    let approval_ready = forge_history_approval_ready(&value);
    let plan_ready = forge_history_plan_ready(&value);
    let evidence_count = forge_history_evidence_count(&value);
    let blocker_count = forge_history_blocker_count(&value).unwrap_or_default();
    let mut details = Vec::new();

    if let Some(status) = status.as_ref() {
        details.push(format!("Status {status}"));
    }
    if let Some(plan_ready) = plan_ready {
        details.push(if plan_ready {
            "Plan ready".to_string()
        } else {
            "Plan blocked".to_string()
        });
    }
    if let Some(approval_ready) = approval_ready {
        details.push(if approval_ready {
            "Approval ready".to_string()
        } else {
            "Approval pending".to_string()
        });
    }
    if let Some(evidence_count) = evidence_count {
        details.push(format!("{evidence_count} evidence"));
    }
    if blocker_count > 0 {
        details.push(format!("{blocker_count} blockers"));
    }

    Some(DxToolHistoryReceiptSummary {
        label: label.to_string(),
        kind: kind.to_string(),
        headline: headline.to_string(),
        detail: if details.is_empty() {
            label.to_string()
        } else {
            details.join(" - ")
        },
        target_path: forge_history_target_path(&value),
        restore_destination_root: forge_history_restore_destination_root(&value),
        blocker_count,
    })
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

fn forge_history_kind(schema: &str, value: &Value) -> Option<&'static str> {
    if schema.contains(".restore_target_plan") || value.get("restore_target_plan").is_some() {
        Some("restore_target_plan")
    } else if schema.contains(".restore_approval") || value.get("restore_approval").is_some() {
        Some("restore_approval")
    } else if schema.contains(".restore_execution") || value.get("restore_execution").is_some() {
        Some("restore_execution")
    } else if schema.contains(".backup_execution") || value.get("backup_execution").is_some() {
        Some("backup_execution")
    } else if schema.contains(".backup_runner_gate") || value.get("runner_gate").is_some() {
        Some("runner_gate")
    } else if schema.contains(".safety_policy") || value.get("forge_safety_policy").is_some() {
        Some("safety_policy")
    } else {
        None
    }
}

fn forge_history_headline(kind: &str) -> &'static str {
    match kind {
        "restore_target_plan" => "Restore target plan",
        "restore_approval" => "Restore approval",
        "restore_execution" => "Restore preview",
        "backup_execution" => "Backup execution",
        "runner_gate" => "Backup runner gate",
        "safety_policy" => "Safety policy",
        _ => "Forge receipt",
    }
}

fn forge_history_status(value: &Value) -> Option<String> {
    string_field(value, &["restore_target_plan", "validation", "status"])
        .or_else(|| string_field(value, &["restore_approval", "validation", "status"]))
        .or_else(|| string_field(value, &["status"]))
        .or_else(|| string_field(value, &["restore_execution", "restore", "status"]))
        .or_else(|| string_field(value, &["backup_execution", "execution", "status"]))
        .or_else(|| string_field(value, &["runner_gate", "validation", "status"]))
        .or_else(|| string_field(value, &["forge_safety_policy", "policy", "status"]))
}

fn forge_history_target_path(value: &Value) -> Option<String> {
    string_field(value, &["restore_target_plan", "request", "target_path"])
        .or_else(|| string_field(value, &["restore_approval", "request", "target_path"]))
        .or_else(|| string_field(value, &["restore_execution", "backup", "target_path"]))
        .or_else(|| string_field(value, &["backup_execution", "gate", "target_path"]))
        .or_else(|| string_field(value, &["runner_gate", "policy", "target_path"]))
        .or_else(|| string_field(value, &["forge_safety_policy", "policy", "target_path"]))
}

fn forge_history_restore_destination_root(value: &Value) -> Option<String> {
    string_field(value, &["restore_destination_root"])
        .or_else(|| {
            string_field(
                value,
                &[
                    "restore_target_plan",
                    "approval",
                    "restore_destination_root",
                ],
            )
        })
        .or_else(|| {
            string_field(
                value,
                &["restore_approval", "restore", "restore_destination_root"],
            )
        })
        .or_else(|| {
            string_field(
                value,
                &["restore_execution", "restore", "restore_destination_root"],
            )
        })
}

fn forge_history_approval_ready(value: &Value) -> Option<bool> {
    bool_field(value, &["restore_approval", "validation", "approval_ready"])
        .or_else(|| {
            bool_field(
                value,
                &["restore_target_plan", "approval", "approval_ready"],
            )
        })
        .or_else(|| bool_field(value, &["approval_ready"]))
}

fn forge_history_plan_ready(value: &Value) -> Option<bool> {
    bool_field(value, &["restore_target_plan", "validation", "plan_ready"])
        .or_else(|| bool_field(value, &["plan_ready"]))
}

fn forge_history_evidence_count(value: &Value) -> Option<usize> {
    usize_field(value, &["restore_approval", "validation", "evidence_count"])
        .or_else(|| {
            usize_field(
                value,
                &["restore_target_plan", "approval", "evidence_count"],
            )
        })
        .or_else(|| usize_field(value, &["evidence_count"]))
}

fn forge_history_blocker_count(value: &Value) -> Option<usize> {
    usize_field(
        value,
        &["restore_target_plan", "validation", "blocker_count"],
    )
    .or_else(|| usize_field(value, &["restore_approval", "validation", "blocker_count"]))
    .or_else(|| usize_field(value, &["blocker_count"]))
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

fn string_field(value: &Value, path: &[&str]) -> Option<String> {
    value_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn bool_field(value: &Value, path: &[&str]) -> Option<bool> {
    value_at(value, path).and_then(Value::as_bool)
}

fn usize_field(value: &Value, path: &[&str]) -> Option<usize> {
    value_at(value, path)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}
