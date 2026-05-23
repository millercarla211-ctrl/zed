use super::*;

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn runtime_proof_evidence_detail_ignores_blank_examples() {
    assert_eq!(
        runtime_proof_evidence_detail(3, &strings(&["", " operator ok ", "  ", "status"])),
        "3+ evidence - operator ok, status"
    );
    assert_eq!(
        runtime_proof_evidence_detail(0, &strings(&["", "  "])),
        "evidence required - use operator proof lines"
    );
}

#[test]
fn runtime_proof_requirements_label_keeps_contract_words() {
    assert_eq!(
        runtime_proof_requirements_label(true, true, false, true),
        "requires clean git, diff check, proof import"
    );
    assert_eq!(
        runtime_proof_requirements_label(false, false, false, false),
        "no extra requirements"
    );
}

#[test]
fn runtime_proof_receipt_state_label_handles_blank_validation_status() {
    assert_eq!(
        runtime_proof_receipt_state_label(false, false, "  ", 2),
        "unknown - 2 blocker(s)"
    );
    assert_eq!(
        runtime_proof_receipt_state_label(true, false, "blocked", 4),
        "Claim-ready"
    );
}
