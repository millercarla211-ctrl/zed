use super::{
    DxToolHistoryBucket, DxToolHistorySnapshot,
    forge_history::forge_receipt_summary,
    receipt_files::{count_receipt_files, push_latest_receipts, root_label},
};
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

pub(super) fn scan_tool_history(workspace_roots: &[String]) -> DxToolHistorySnapshot {
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
