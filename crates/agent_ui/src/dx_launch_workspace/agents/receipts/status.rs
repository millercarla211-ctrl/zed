use gpui::{AnyElement, SharedString, prelude::*};
use ui::{Color, IconName};

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::super::super::signal_row;
use super::text::receipt_optional_label;

pub(super) fn dx_agent_receipt_warning_rows(
    snapshot: &DxAgentBridgeSnapshot,
    unsafe_count: usize,
) -> Vec<AnyElement> {
    let index = &snapshot.receipt_index;
    let inbox = &snapshot.receipt_inbox;
    let mut rows = Vec::new();

    if index.receipt_root_present == Some(false) {
        rows.push(warning_row(
            "dx-agent-receipt-root-missing".into(),
            "DX Agents receipt root was missing before the latest receipt refresh.".to_string(),
        ));
    }

    if inbox.receipt_dir_present == Some(false) {
        rows.push(warning_row(
            "dx-agent-receipt-inbox-root-missing".into(),
            "DX Agents receipt inbox reports a missing receipt directory.".to_string(),
        ));
    } else if inbox.malformed_count > 0 {
        rows.push(warning_row(
            "dx-agent-receipt-inbox-malformed".into(),
            format!(
                "DX Agents receipt inbox found {} malformed receipt(s).",
                inbox.malformed_count
            ),
        ));
    }

    if index.present {
        if let Some(error) = index.last_error.as_deref().and_then(receipt_optional_label) {
            rows.push(warning_row(
                "dx-agent-receipt-index-error".into(),
                format!("DX Agents receipt index error: {error}"),
            ));
        } else if index.last_error.is_some() {
            rows.push(warning_row(
                "dx-agent-receipt-index-error".into(),
                "DX Agents receipt index reported an error.".to_string(),
            ));
        } else if unsafe_count > 0 {
            rows.push(warning_row(
                "dx-agent-receipt-unsafe-row".into(),
                "DX Agents receipt index contains rows that are not safe to render.".to_string(),
            ));
        }
    }

    rows
}

pub(super) fn dx_agent_receipt_root_state(
    receipt_root_present: Option<bool>,
    root_exists: bool,
) -> String {
    match receipt_root_present {
        Some(true) => "present".to_string(),
        Some(false) => "missing before refresh".to_string(),
        None if root_exists => "present".to_string(),
        None => "missing".to_string(),
    }
}

fn warning_row(id: SharedString, message: String) -> AnyElement {
    signal_row(id, IconName::Warning, Color::Warning, message)
}
