use gpui::AnyElement;

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::super::super::super::metric_row;

pub(super) fn dx_agent_bridge_contract_rows(snapshot: &DxAgentBridgeSnapshot) -> Vec<AnyElement> {
    vec![
        metric_row("Contract", snapshot.contract_summary.status.clone()),
        metric_row(
            "Commands",
            snapshot.contract_summary.command_count.to_string(),
        ),
        metric_row(
            "Receipts",
            snapshot.contract_summary.receipt_count.to_string(),
        ),
        metric_row(
            "Contract Catalog",
            format!(
                "{} / {} receipt(s)",
                snapshot.contract_summary.provider_catalog_source,
                snapshot.contract_summary.provider_catalog_receipt_count
            ),
        ),
        metric_row(
            "Redaction",
            if snapshot.contract_summary.redaction_requires_review {
                "review".to_string()
            } else {
                snapshot.contract_summary.redaction_summary.clone()
            },
        ),
    ]
}
