use gpui::{AnyElement, App, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_runtime_proof_status::DxRuntimeProofReceiptSummary;

use super::super::super::metric_row;
use super::super::super::proof_labels::runtime_proof_receipt_state_label;
use super::receipt_details::runtime_proof_receipt_detail_rows;

pub(in super::super) fn runtime_proof_receipt_row(
    id: &'static str,
    label: &'static str,
    receipt: &DxRuntimeProofReceiptSummary,
    cx: &App,
) -> AnyElement {
    let state = runtime_proof_receipt_state_label(
        receipt.runtime_green_candidate,
        receipt.can_claim_runtime_green,
        &receipt.validation_status,
        receipt.blocker_count,
    );
    let stack = v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(label, state))
        .child(
            Label::new(format!(
                "{} evidence - operator {}",
                receipt.evidence_count, receipt.operator_status
            ))
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        )
        .child(
            Label::new(receipt.label.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .children(runtime_proof_receipt_detail_rows(receipt));

    stack.into_any_element()
}
