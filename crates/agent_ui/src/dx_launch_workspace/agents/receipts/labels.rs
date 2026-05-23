use crate::dx_agent_bridge::DxAgentReceipt;

use super::super::super::list_labels::yes_no;

pub(super) fn receipt_state_label(receipt: &DxAgentReceipt) -> String {
    if !receipt.safe_to_render {
        "Unsafe".to_string()
    } else if receipt.active_task {
        "Active".to_string()
    } else if receipt.metadata_redacted {
        format!("{} / redacted", receipt.status)
    } else {
        receipt.status.clone()
    }
}

pub(super) fn receipt_detail_label(receipt: &DxAgentReceipt) -> String {
    if receipt.command.is_empty() {
        format!("{} - {} bytes", receipt.kind, receipt.size_bytes)
    } else {
        format!(
            "{} - {} - {} bytes",
            receipt.kind, receipt.command, receipt.size_bytes
        )
    }
}

pub(super) fn receipt_provider_model_label(receipt: &DxAgentReceipt) -> Option<String> {
    match (
        receipt.provider_status.as_deref(),
        receipt.model_status.as_deref(),
    ) {
        (Some(provider), Some(model)) => Some(format!("Provider {provider}, model {model}")),
        (Some(provider), None) => Some(format!("Provider {provider}")),
        (None, Some(model)) => Some(format!("Model {model}")),
        (None, None) => None,
    }
}

pub(super) fn receipt_action_label(receipt: &DxAgentReceipt) -> Option<String> {
    match (receipt.retry_supported, receipt.cancel_supported) {
        (Some(retry), Some(cancel)) => Some(format!(
            "Retry {}, cancel {}",
            yes_no(retry),
            yes_no(cancel)
        )),
        (Some(retry), None) => Some(format!("Retry {}", yes_no(retry))),
        (None, Some(cancel)) => Some(format!("Cancel {}", yes_no(cancel))),
        (None, None) => None,
    }
}

pub(super) fn receipt_social_label(receipt: &DxAgentReceipt) -> Option<String> {
    match (receipt.social_connected, receipt.social_needs_auth) {
        (Some(connected), Some(needs_auth)) => Some(format!(
            "Social connected {connected}, needs auth {needs_auth}"
        )),
        (Some(connected), None) => Some(format!("Social connected {connected}")),
        (None, Some(needs_auth)) => Some(format!("Social needs auth {needs_auth}")),
        (None, None) => None,
    }
}

pub(super) fn receipt_automation_label(receipt: &DxAgentReceipt) -> Option<String> {
    match (receipt.automation_enabled, receipt.automation_warning) {
        (Some(enabled), Some(warning)) => {
            Some(format!("Automations enabled {enabled}, warning {warning}"))
        }
        (Some(enabled), None) => Some(format!("Automations enabled {enabled}")),
        (None, Some(warning)) => Some(format!("Automation warnings {warning}")),
        (None, None) => None,
    }
}
