pub(crate) fn runtime_proof_evidence_detail(
    minimum_evidence_lines_for_pass: usize,
    accepted_evidence_examples: &[String],
) -> String {
    let minimum = if minimum_evidence_lines_for_pass > 0 {
        format!("{minimum_evidence_lines_for_pass}+ evidence")
    } else {
        "evidence required".to_string()
    };
    let examples = accepted_evidence_examples
        .iter()
        .map(|example| example.trim())
        .filter(|example| !example.is_empty())
        .take(2)
        .collect::<Vec<_>>();
    let examples = if examples.is_empty() {
        "use operator proof lines".to_string()
    } else {
        examples.join(", ")
    };

    format!("{minimum} - {examples}")
}

pub(crate) fn runtime_proof_requirements_label(
    requires_clean_git: bool,
    requires_diff_check: bool,
    requires_visual_evidence: bool,
    requires_import: bool,
) -> String {
    let mut requirements = Vec::new();

    if requires_clean_git {
        requirements.push("clean git");
    }
    if requires_diff_check {
        requirements.push("diff check");
    }
    if requires_visual_evidence {
        requirements.push("visual proof");
    }
    if requires_import {
        requirements.push("proof import");
    }

    if requirements.is_empty() {
        "no extra requirements".to_string()
    } else {
        format!("requires {}", requirements.join(", "))
    }
}

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

#[cfg(test)]
mod tests {
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
}
