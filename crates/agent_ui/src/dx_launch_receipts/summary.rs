use super::{
    DX_LAUNCH_STATUS_SCHEMA, DxLaunchReceiptSummary,
    fields::{optional_string_field, u64_field},
    freshness::freshness_state,
    paths::{file_name, receipt_order_ms},
    receipt_io::read_json_receipt,
};
use std::path::Path;

impl DxLaunchReceiptSummary {
    pub(super) fn from_path(kind: &str, path: &Path, now_ms: u64) -> Self {
        match read_json_receipt(path) {
            Ok(value) => {
                let generated_at_ms =
                    u64_field(&value, "generated_at_ms").or_else(|| receipt_order_ms(path));
                let age_ms = generated_at_ms.map(|generated| now_ms.saturating_sub(generated));

                Self {
                    kind: kind.to_string(),
                    file_name: file_name(path),
                    receipt_path: path.display().to_string(),
                    schema_version: optional_string_field(&value, "schema_version"),
                    status: optional_string_field(&value, "status"),
                    generated_at_ms,
                    age_ms,
                    freshness_state: freshness_state(false, age_ms).to_string(),
                    malformed: false,
                    last_error: optional_string_field(&value, "last_error"),
                    next_action: optional_string_field(&value, "next_action"),
                }
            }
            Err(error) => {
                let generated_at_ms = receipt_order_ms(path);
                let age_ms = generated_at_ms.map(|generated| now_ms.saturating_sub(generated));

                Self {
                    kind: kind.to_string(),
                    file_name: file_name(path),
                    receipt_path: path.display().to_string(),
                    schema_version: None,
                    status: None,
                    generated_at_ms,
                    age_ms,
                    freshness_state: freshness_state(true, age_ms).to_string(),
                    malformed: true,
                    last_error: Some(error),
                    next_action: Some("repair_or_prune_malformed_launch_receipt".to_string()),
                }
            }
        }
    }

    pub(crate) fn display_state(&self) -> String {
        let status = self.status.as_deref().unwrap_or("unknown");
        let schema = self.schema_version.as_deref().unwrap_or("missing schema");
        format!("{} / {status} / {schema}", self.freshness_state)
    }

    pub(crate) fn schema_matches_launch_status(&self) -> bool {
        self.schema_version.as_deref() == Some(DX_LAUNCH_STATUS_SCHEMA)
    }
}
