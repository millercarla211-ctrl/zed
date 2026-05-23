#[path = "proof_labels/evidence.rs"]
mod evidence;
#[path = "proof_labels/receipt.rs"]
mod receipt;

pub(crate) use evidence::{runtime_proof_evidence_detail, runtime_proof_requirements_label};
pub(crate) use receipt::runtime_proof_receipt_state_label;

#[cfg(test)]
mod proof_labels_tests;
