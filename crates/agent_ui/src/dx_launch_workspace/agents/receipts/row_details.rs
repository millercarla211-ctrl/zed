use gpui::{AnyElement, SharedString, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_agent_bridge::DxAgentReceipt;

use super::labels::{
    receipt_action_label, receipt_automation_label, receipt_detail_label,
    receipt_provider_model_label, receipt_social_label,
};

pub(super) fn dx_agent_receipt_detail_rows(receipt: &DxAgentReceipt) -> Vec<AnyElement> {
    let mut rows = vec![muted_line(receipt_detail_label(receipt))];

    push_nonempty_metric(&mut rows, "Task", &receipt.task_state);
    if let Some(duration) = receipt.duration_state.as_ref() {
        rows.push(muted_line(format!("Duration: {duration}")));
    }
    push_optional_line(&mut rows, receipt_provider_model_label(receipt));
    push_optional_line(&mut rows, receipt_action_label(receipt));
    push_optional_line(&mut rows, receipt_social_label(receipt));
    push_optional_line(&mut rows, receipt_automation_label(receipt));
    push_nonempty_line(&mut rows, receipt.schema_version.clone());

    if let Some(error) = receipt.last_error.as_ref() {
        rows.push(muted_line(format!("Error: {error}")));
    } else {
        push_nonempty_line(&mut rows, receipt.next_action.clone());
    }

    rows
}

fn push_optional_line(rows: &mut Vec<AnyElement>, line: Option<String>) {
    if let Some(line) = line {
        rows.push(muted_line(line));
    }
}

fn push_nonempty_metric(rows: &mut Vec<AnyElement>, label: &str, value: &str) {
    if !value.is_empty() {
        rows.push(muted_line(format!("{label}: {value}")));
    }
}

fn push_nonempty_line(rows: &mut Vec<AnyElement>, line: String) {
    if !line.is_empty() {
        rows.push(muted_line(line));
    }
}

fn muted_line(line: impl Into<SharedString>) -> AnyElement {
    Label::new(line)
        .size(LabelSize::XSmall)
        .color(Color::Muted)
        .truncate()
        .into_any_element()
}
