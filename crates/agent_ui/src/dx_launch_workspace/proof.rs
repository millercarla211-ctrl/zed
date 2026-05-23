use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_runtime_proof_status::DxRuntimeProofStatusSnapshot;

use self::runtime_rows::{runtime_proof_plan_row, runtime_proof_receipt_row};
use super::{metric_row, muted_card, signal_row};

mod freshness;
mod runtime_rows;
pub(super) use freshness::proof_freshness_state;

pub(super) fn runtime_proof_status_state(
    snapshot: &DxRuntimeProofStatusSnapshot,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Claim", snapshot.claim_state.clone()))
        .child(metric_row(
            "Receipts",
            format!(
                "{} plan, {} import, {} status",
                snapshot.plan_receipt_count,
                snapshot.import_receipt_count,
                snapshot.status_receipt_count
            ),
        ));

    if snapshot.workspace_root_count == 0 {
        stack = stack.child(muted_card("No workspace roots", cx));
    } else if !snapshot.plan_root_exists
        && !snapshot.import_root_exists
        && !snapshot.status_root_exists
    {
        stack = stack.child(muted_card("No runtime proof receipt roots", cx));
    }

    if let Some(plan) = snapshot.latest_plan.as_ref() {
        stack = stack.child(runtime_proof_plan_row(plan, cx));
    }

    if let Some(receipt) = snapshot.latest_import.as_ref() {
        stack = stack.child(runtime_proof_receipt_row(
            "dx-runtime-proof-latest-import",
            "Import",
            receipt,
            cx,
        ));
    }

    if let Some(receipt) = snapshot.latest_status.as_ref() {
        stack = stack.child(runtime_proof_receipt_row(
            "dx-runtime-proof-latest-status",
            "Status",
            receipt,
            cx,
        ));
    }

    for (ix, blocker) in snapshot.blockers.iter().take(2).enumerate() {
        stack = stack.child(signal_row(
            SharedString::from(format!("dx-runtime-proof-blocker-{ix}")),
            IconName::Warning,
            Color::Warning,
            blocker.clone(),
        ));
    }

    stack.into_any_element()
}
