use gpui::{AnyElement, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_runtime_proof_status::DxRuntimeProofReceiptSummary;

pub(super) fn runtime_proof_receipt_detail_rows(
    receipt: &DxRuntimeProofReceiptSummary,
) -> Vec<AnyElement> {
    let mut rows = Vec::new();

    if let Some(headline) = receipt.headline.as_ref() {
        rows.push(detail_label(headline.clone()));
    }

    if let Some(summary) = receipt.proof_summary.as_ref() {
        rows.push(detail_label(format!("Summary {summary}")));
    }

    if let Some(command) = receipt.final_command.as_ref() {
        rows.push(detail_label(format!("Command {command}")));
    }

    if let Some(source) = receipt.source.as_ref() {
        rows.push(detail_label(format!("Source {source}")));
    }

    if let Some(evidence) = receipt.evidence_samples.first() {
        rows.push(detail_label(format!("Evidence {evidence}")));
    }

    rows
}

fn detail_label(text: String) -> AnyElement {
    Label::new(text)
        .size(LabelSize::XSmall)
        .color(Color::Muted)
        .truncate()
        .into_any_element()
}
