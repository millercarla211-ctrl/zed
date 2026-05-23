use super::DxSourceItem;
use std::path::{Path, PathBuf};

pub(super) fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

pub(super) fn short_hash(hash: &str) -> String {
    hash.chars().take(12).collect()
}

pub(super) fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;

    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

pub(super) fn source_set_status(
    workspace_roots: &[PathBuf],
    sources: &[DxSourceItem],
    empty_label: &'static str,
) -> String {
    if workspace_roots.is_empty() {
        "No workspace".to_string()
    } else if sources.is_empty() {
        empty_label.to_string()
    } else {
        format!("{} source(s)", sources.len())
    }
}
