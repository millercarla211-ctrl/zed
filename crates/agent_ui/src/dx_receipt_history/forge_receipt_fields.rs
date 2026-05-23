use super::fields::{bool_field, string_field, usize_field};
use serde_json::Value;

pub(super) fn forge_history_kind(schema: &str, value: &Value) -> Option<&'static str> {
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

pub(super) fn forge_history_headline(kind: &str) -> &'static str {
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

pub(super) fn forge_history_status(value: &Value) -> Option<String> {
    string_field(value, &["restore_target_plan", "validation", "status"])
        .or_else(|| string_field(value, &["restore_approval", "validation", "status"]))
        .or_else(|| string_field(value, &["status"]))
        .or_else(|| string_field(value, &["restore_execution", "restore", "status"]))
        .or_else(|| string_field(value, &["backup_execution", "execution", "status"]))
        .or_else(|| string_field(value, &["runner_gate", "validation", "status"]))
        .or_else(|| string_field(value, &["forge_safety_policy", "policy", "status"]))
}

pub(super) fn forge_history_target_path(value: &Value) -> Option<String> {
    string_field(value, &["restore_target_plan", "request", "target_path"])
        .or_else(|| string_field(value, &["restore_approval", "request", "target_path"]))
        .or_else(|| string_field(value, &["restore_execution", "backup", "target_path"]))
        .or_else(|| string_field(value, &["backup_execution", "gate", "target_path"]))
        .or_else(|| string_field(value, &["runner_gate", "policy", "target_path"]))
        .or_else(|| string_field(value, &["forge_safety_policy", "policy", "target_path"]))
}

pub(super) fn forge_history_restore_destination_root(value: &Value) -> Option<String> {
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

pub(super) fn forge_history_approval_ready(value: &Value) -> Option<bool> {
    bool_field(value, &["restore_approval", "validation", "approval_ready"])
        .or_else(|| {
            bool_field(
                value,
                &["restore_target_plan", "approval", "approval_ready"],
            )
        })
        .or_else(|| bool_field(value, &["approval_ready"]))
}

pub(super) fn forge_history_plan_ready(value: &Value) -> Option<bool> {
    bool_field(value, &["restore_target_plan", "validation", "plan_ready"])
        .or_else(|| bool_field(value, &["plan_ready"]))
}

pub(super) fn forge_history_evidence_count(value: &Value) -> Option<usize> {
    usize_field(value, &["restore_approval", "validation", "evidence_count"])
        .or_else(|| {
            usize_field(
                value,
                &["restore_target_plan", "approval", "evidence_count"],
            )
        })
        .or_else(|| usize_field(value, &["evidence_count"]))
}

pub(super) fn forge_history_blocker_count(value: &Value) -> Option<usize> {
    usize_field(
        value,
        &["restore_target_plan", "validation", "blocker_count"],
    )
    .or_else(|| usize_field(value, &["restore_approval", "validation", "blocker_count"]))
    .or_else(|| usize_field(value, &["blocker_count"]))
}
