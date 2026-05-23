use super::receipt_fields::{array_strings_at, bool_at, string_at};
use serde_json::Value;

pub(super) fn forge_restore_warnings(value: &Value) -> Vec<String> {
    let restore = value.get("restore").or_else(|| {
        value
            .get("restore_execution")
            .and_then(|value| value.get("restore"))
    });
    let Some(restore) = restore else {
        return vec!["Restore receipt has no restore summary".to_string()];
    };

    let mut warnings = array_strings_at(restore, &["blockers"]);
    if bool_at(restore, &["target_mutation_applied"]).unwrap_or_default() {
        warnings.push("Receipt reports target mutation".to_string());
    }
    if bool_at(restore, &["overwrote_existing_files"]).unwrap_or_default() {
        warnings.push("Receipt reports overwritten files".to_string());
    }
    if bool_at(restore, &["ran_shell"]).unwrap_or_default()
        || bool_at(restore, &["ran_external_process"]).unwrap_or_default()
    {
        warnings.push("Receipt reports external execution".to_string());
    }
    if !bool_at(restore, &["restore_ready"]).unwrap_or_default() {
        let status = string_at(restore, &["status"]).unwrap_or_else(|| "not ready".to_string());
        warnings.push(format!("Restore preview status: {status}"));
    }

    warnings.truncate(3);
    warnings
}
