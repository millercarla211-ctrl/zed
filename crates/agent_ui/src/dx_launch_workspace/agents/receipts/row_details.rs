use gpui::{AnyElement, SharedString, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_agent_bridge::DxAgentReceipt;

use super::labels::{
    receipt_action_label, receipt_automation_label, receipt_detail_label,
    receipt_provider_model_label, receipt_social_label,
};
use super::text::receipt_optional_label;

pub(super) fn dx_agent_receipt_detail_rows(receipt: &DxAgentReceipt) -> Vec<AnyElement> {
    let mut rows = vec![muted_line(receipt_detail_label(receipt))];

    if !receipt.safe_to_render {
        return rows;
    }

    push_optional_metric(
        &mut rows,
        "Task",
        receipt_optional_label(&receipt.task_state),
    );
    if let Some(duration) = receipt
        .duration_state
        .as_deref()
        .and_then(receipt_optional_label)
    {
        rows.push(muted_line(format!("Duration: {duration}")));
    }
    push_optional_line(&mut rows, receipt_provider_model_label(receipt));
    push_optional_line(&mut rows, receipt_action_label(receipt));
    push_optional_line(&mut rows, receipt_social_label(receipt));
    push_optional_line(&mut rows, receipt_automation_label(receipt));
    push_optional_line(&mut rows, receipt_optional_label(&receipt.schema_version));

    if let Some(error) = receipt
        .last_error
        .as_deref()
        .and_then(receipt_optional_label)
    {
        rows.push(muted_line(format!("Error: {error}")));
    } else {
        push_optional_line(&mut rows, receipt_optional_label(&receipt.next_action));
    }

    rows
}

fn push_optional_line(rows: &mut Vec<AnyElement>, line: Option<String>) {
    if let Some(line) = line {
        rows.push(muted_line(line));
    }
}

fn push_optional_metric(rows: &mut Vec<AnyElement>, label: &str, value: Option<String>) {
    if let Some(value) = value {
        rows.push(muted_line(format!("{label}: {value}")));
    }
}

fn muted_line(line: impl Into<SharedString>) -> AnyElement {
    Label::new(line)
        .size(LabelSize::XSmall)
        .color(Color::Muted)
        .truncate()
        .into_any_element()
}
