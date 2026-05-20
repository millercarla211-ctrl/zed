use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime},
};

use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};

use crate::dx_receipt_history::{DxToolHistoryBucket, DxToolHistorySnapshot};
use crate::dx_source_sets::{DxSourceItem, DxSourceKind, DxSourceSet, DxSourceSetSnapshot};

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

#[derive(Clone)]
pub(crate) struct DxLaunchWorkspaceStatus {
    pub active_status: SharedString,
    pub background_task_count: usize,
    pub visible_worktree_count: usize,
    pub receipt_snapshot: DxReceiptSnapshot,
    pub source_sets: DxSourceSetSnapshot,
    pub tool_history: DxToolHistorySnapshot,
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

pub(crate) fn render_workspace_chrome(
    center: AnyElement,
    sidebar_actions: AnyElement,
    status: DxLaunchWorkspaceStatus,
    cx: &mut App,
) -> AnyElement {
    h_flex()
        .id("dx-launch-workspace")
        .size_full()
        .min_w_0()
        .bg(cx.theme().colors().panel_background)
        .child(render_sources_rail(sidebar_actions, &status, cx))
        .child(div().flex_1().min_w_0().size_full().child(center))
        .child(render_right_rail(&status, cx))
        .into_any_element()
}

fn render_sources_rail(
    sidebar_actions: AnyElement,
    status: &DxLaunchWorkspaceStatus,
    cx: &mut App,
) -> AnyElement {
    v_flex()
        .id("dx-sources-rail")
        .w(px(218.0))
        .h_full()
        .flex_none()
        .gap_2()
        .p_2()
        .border_r_1()
        .border_color(cx.theme().colors().border)
        .bg(cx.theme().colors().tab_bar_background)
        .child(section_title("Workspace", IconName::Library))
        .child(sidebar_actions)
        .child(section_title("Sources", IconName::Book))
        .child(source_set_stack(&status.source_sets, cx))
        .child(section_title("Receipts", IconName::FileTextOutlined))
        .child(receipt_source_state(&status.receipt_snapshot, cx))
        .into_any_element()
}

fn render_right_rail(status: &DxLaunchWorkspaceStatus, cx: &mut App) -> AnyElement {
    v_flex()
        .id("dx-progress-rail")
        .w(px(244.0))
        .h_full()
        .flex_none()
        .gap_2()
        .p_2()
        .border_l_1()
        .border_color(cx.theme().colors().border)
        .bg(cx.theme().colors().tab_bar_background)
        .child(section_title("Progress", IconName::TodoProgress))
        .child(metric_row("Thread", status.active_status.clone()))
        .child(metric_row(
            "Background",
            format!("{} tasks", status.background_task_count),
        ))
        .child(section_title("Git", IconName::GitBranch))
        .child(metric_row(
            "Worktrees",
            status.visible_worktree_count.to_string(),
        ))
        .child(section_title("Tool History", IconName::Archive))
        .child(tool_history_state(&status.tool_history, cx))
        .child(section_title("Background Tasks", IconName::Clock))
        .child(background_task_state(status.background_task_count, cx))
        .child(section_title("Token And Tool Slots", IconName::Sliders))
        .child(token_meter_slots(&status.receipt_snapshot))
        .into_any_element()
}

fn source_set_stack(snapshot: &DxSourceSetSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex().gap_1();

    if snapshot.total_sources == 0 {
        stack = stack.child(muted_card("No workspace source", cx));
    } else {
        for (ix, set) in snapshot.sets.iter().enumerate() {
            stack = stack.child(source_set_card(
                SharedString::from(format!("source-set-{ix}")),
                set,
                cx,
            ));
        }
    }

    stack.into_any_element()
}

fn source_set_card(id: SharedString, set: &DxSourceSet, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .id(id)
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(cx.theme().colors().border_variant)
        .px_2()
        .py_1()
        .child(metric_row(set.label, set.status.clone()));

    if set.sources.is_empty() {
        return stack.into_any_element();
    }

    let set_id = set.label.to_ascii_lowercase().replace(' ', "-");
    for (ix, source) in set.sources.iter().take(3).enumerate() {
        stack = stack.child(source_item_row(
            SharedString::from(format!("{set_id}-source-{ix}")),
            source,
            cx,
        ));
    }

    stack.into_any_element()
}

fn source_item_row(id: SharedString, source: &DxSourceItem, cx: &App) -> AnyElement {
    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(
            h_flex()
                .gap_1()
                .min_w_0()
                .items_center()
                .child(
                    Icon::new(source_kind_icon(source.kind))
                        .size(IconSize::XSmall)
                        .color(Color::Muted),
                )
                .child(
                    Label::new(source.label.clone())
                        .size(LabelSize::XSmall)
                        .color(Color::Default)
                        .truncate(),
                ),
        )
        .child(
            Label::new(source.detail.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(
            Label::new(source.path.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

fn source_kind_icon(kind: DxSourceKind) -> IconName {
    match kind {
        DxSourceKind::WorkspaceRoot => IconName::Folder,
        DxSourceKind::MetasearchSourcePack => IconName::FileTextOutlined,
        DxSourceKind::MediaOutput => IconName::File,
        DxSourceKind::ForgeRestorePreview => IconName::Archive,
    }
}

fn receipt_source_state(snapshot: &DxReceiptSnapshot, cx: &mut App) -> AnyElement {
    if !snapshot.root_exists {
        return muted_card(
            format!("Receipts not found: {}", snapshot.root.display()),
            cx,
        );
    }

    let total = snapshot
        .buckets
        .iter()
        .map(|bucket| bucket.count)
        .sum::<usize>();
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Receipt files", total.to_string()));

    if snapshot.latest.is_empty() {
        stack = stack.child(muted_card("Waiting for first DX receipt", cx));
    } else {
        for (ix, label) in snapshot.latest.iter().enumerate() {
            stack = stack.child(source_row(
                SharedString::from(format!("latest-receipt-{ix}")),
                IconName::FileTextOutlined,
                label.clone(),
                cx,
            ));
        }
    }

    stack.into_any_element()
}

fn tool_history_state(snapshot: &DxToolHistorySnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex().gap_1();

    for (ix, bucket) in snapshot.buckets.iter().enumerate() {
        stack = stack.child(tool_history_bucket(
            SharedString::from(format!("dx-tool-history-{ix}")),
            bucket,
            cx,
        ));
    }

    stack.into_any_element()
}

fn tool_history_bucket(id: SharedString, bucket: &DxToolHistoryBucket, cx: &App) -> AnyElement {
    let state = if !bucket.root_exists {
        format!("Missing: {}", bucket.root_label)
    } else if bucket.count == 0 {
        "No receipts".to_string()
    } else {
        format!("{} receipts", bucket.count)
    };
    let mut stack = v_flex()
        .id(id)
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(cx.theme().colors().border_variant)
        .px_2()
        .py_1()
        .child(metric_row(bucket.label, state));

    if bucket.root_exists {
        let bucket_id = bucket.label.to_ascii_lowercase().replace(' ', "-");
        for (ix, label) in bucket.latest.iter().enumerate() {
            stack = stack.child(source_row(
                SharedString::from(format!("{bucket_id}-latest-{ix}")),
                IconName::FileTextOutlined,
                label.clone(),
                cx,
            ));
        }
    }

    stack.into_any_element()
}

fn background_task_state(count: usize, cx: &App) -> AnyElement {
    if count == 0 {
        muted_card("No retained background tasks", cx)
    } else {
        metric_row("Retained", count.to_string())
    }
}

fn token_meter_slots(snapshot: &DxReceiptSnapshot) -> AnyElement {
    let token_count = snapshot
        .buckets
        .iter()
        .find(|bucket| bucket.label == "Tokens")
        .map(|bucket| bucket.count)
        .unwrap_or_default();
    let rlm_count = snapshot
        .buckets
        .iter()
        .find(|bucket| bucket.label == "RLM")
        .map(|bucket| bucket.count)
        .unwrap_or_default();
    let serializer_count = snapshot
        .buckets
        .iter()
        .find(|bucket| bucket.label == "Serializer")
        .map(|bucket| bucket.count)
        .unwrap_or_default();

    v_flex()
        .gap_1()
        .child(metric_row("Prompt", "-"))
        .child(metric_row("Output", "-"))
        .child(metric_row("Tools", "-"))
        .child(metric_row("Token receipts", token_count.to_string()))
        .child(metric_row("RLM receipts", rlm_count.to_string()))
        .child(metric_row("Serializer", serializer_count.to_string()))
        .into_any_element()
}

fn section_title(label: &'static str, icon: IconName) -> AnyElement {
    h_flex()
        .gap_1()
        .items_center()
        .pt_1()
        .child(Icon::new(icon).size(IconSize::XSmall).color(Color::Muted))
        .child(
            Label::new(label)
                .size(LabelSize::XSmall)
                .color(Color::Muted),
        )
        .into_any_element()
}

fn source_row(
    id: SharedString,
    icon: IconName,
    label: impl Into<SharedString>,
    cx: &App,
) -> AnyElement {
    h_flex()
        .id(id)
        .gap_1()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(Icon::new(icon).size(IconSize::XSmall).color(Color::Muted))
        .child(
            Label::new(label.into())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

fn metric_row(label: impl Into<SharedString>, value: impl Into<SharedString>) -> AnyElement {
    h_flex()
        .justify_between()
        .gap_2()
        .min_w_0()
        .child(
            Label::new(label.into())
                .size(LabelSize::XSmall)
                .color(Color::Muted),
        )
        .child(
            Label::new(value.into())
                .size(LabelSize::XSmall)
                .color(Color::Default)
                .truncate(),
        )
        .into_any_element()
}

fn muted_card(label: impl Into<SharedString>, cx: &App) -> AnyElement {
    div()
        .w_full()
        .rounded_sm()
        .border_1()
        .border_color(cx.theme().colors().border_variant)
        .px_2()
        .py_1()
        .child(
            Label::new(label.into())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}
