use gpui::{AnyElement, prelude::*};
use ui::{Color, IconName};

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use super::super::super::signal_row;

pub(super) fn dx_agent_receipt_warning_rows(
    snapshot: &DxAgentBridgeSnapshot,
    unsafe_count: usize,
) -> Vec<AnyElement> {
    let index = &snapshot.receipt_index;
    let inbox = &snapshot.receipt_inbox;
    let mut rows = Vec::new();

    if index.receipt_root_present == Some(false) {
        rows.push(signal_row(
            "dx-agent-receipt-root-missing".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents receipt root was missing before the latest receipt refresh.".to_string(),
        ));
    }

    if inbox.receipt_dir_present == Some(false) {
        rows.push(signal_row(
            "dx-agent-receipt-inbox-root-missing".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents receipt inbox reports a missing receipt directory.".to_string(),
        ));
    } else if inbox.malformed_count > 0 {
        rows.push(signal_row(
            "dx-agent-receipt-inbox-malformed".into(),
            IconName::Warning,
            Color::Warning,
            format!(
                "DX Agents receipt inbox found {} malformed receipt(s).",
                inbox.malformed_count
            ),
        ));
    }

    if index.present {
        if let Some(error) = index.last_error.as_ref() {
            rows.push(signal_row(
                "dx-agent-receipt-index-error".into(),
                IconName::Warning,
                Color::Warning,
                format!("DX Agents receipt index error: {error}"),
            ));
        } else if unsafe_count > 0 {
            rows.push(signal_row(
                "dx-agent-receipt-unsafe-row".into(),
                IconName::Warning,
                Color::Warning,
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
