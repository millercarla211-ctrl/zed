use super::fields::{bool_field, string_field, usize_field};
use serde_json::Value;

pub(super) fn repo_row(repo: &Value) -> String {
    format!(
        "{}: {} / {} (entries {}, staged {}, unstaged {}, untracked {}, split {}, diff {})",
        string_field(repo, "name").unwrap_or("unknown"),
        string_field(repo, "state").unwrap_or("unknown"),
        string_field(repo, "commit_gate").unwrap_or("unknown"),
        usize_field(repo, "total_entries").unwrap_or_default(),
        usize_field(repo, "staged_count").unwrap_or_default(),
        usize_field(repo, "unstaged_count").unwrap_or_default(),
        usize_field(repo, "untracked_count").unwrap_or_default(),
        usize_field(repo, "split_index_worktree_count").unwrap_or_default(),
        usize_field(repo, "diff_check_exit")
            .map(|exit| exit.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    )
}

pub(super) fn delta_row(delta: &Value) -> String {
    format!(
        "{}: branch_changed={} active={} -> {} entries_delta={} tracked_delta={}",
        string_field(delta, "name").unwrap_or("unknown"),
        bool_label(bool_field(delta, "branch_changed")),
        bool_label(bool_field(delta, "previous_active_output")),
        bool_label(bool_field(delta, "current_active_output")),
        signed_field(delta, "total_entries_delta"),
        signed_field(delta, "tracked_dirty_delta")
    )
}

fn signed_field(value: &Value, field: &str) -> i64 {
    value.get(field).and_then(Value::as_i64).unwrap_or_default()
}

fn bool_label(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
