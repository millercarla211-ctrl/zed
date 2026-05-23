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
