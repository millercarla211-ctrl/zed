pub(crate) fn runtime_proof_receipt_state_label(
    runtime_green_candidate: bool,
    can_claim_runtime_green: bool,
    validation_status: &str,
    blocker_count: usize,
) -> String {
    if runtime_green_candidate || can_claim_runtime_green {
        return "Claim-ready".to_string();
    }

    let validation_status = validation_status.trim();
    let validation_status = if validation_status.is_empty() {
        "unknown"
    } else {
        validation_status
    };

    format!("{validation_status} - {blocker_count} blocker(s)")
}
