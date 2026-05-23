use super::{
    DxToolHistoryReceiptSummary,
    forge_receipt_fields::{
        forge_history_approval_ready, forge_history_blocker_count, forge_history_evidence_count,
        forge_history_headline, forge_history_kind, forge_history_plan_ready,
        forge_history_restore_destination_root, forge_history_status, forge_history_target_path,
    },
    receipt_io::read_json,
};
use std::path::Path;

pub(super) fn forge_receipt_summary(
    path: &Path,
    label: &str,
) -> Option<DxToolHistoryReceiptSummary> {
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
