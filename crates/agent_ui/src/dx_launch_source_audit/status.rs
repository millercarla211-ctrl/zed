pub(super) fn source_audit_status(
    root_exists: bool,
    latest_present: bool,
    audit_valid: bool,
    ready_for_commit_coordination: bool,
    coordination_status: &str,
    risk_review_count: usize,
    diff_check_failure_count: usize,
    passed: bool,
    has_issues: bool,
    has_blockers: bool,
    score: usize,
) -> &'static str {
    if !root_exists || !latest_present {
        "missing"
    } else if !audit_valid {
        "invalid"
    } else if !ready_for_commit_coordination
        || coordination_status.contains("blocked")
        || risk_review_count > 0
        || diff_check_failure_count > 0
    {
        "blocked"
    } else if !passed || has_issues || has_blockers || score < 100 {
        "warning"
    } else {
        "ready"
    }
}

pub(super) fn source_audit_operator_summary(
    issues: &[String],
    score: usize,
    coordination_status: &str,
    next_target: &str,
) -> String {
    if let Some(first_issue) = issues.first() {
        first_issue.clone()
    } else {
        format!(
            "DX source audit {score}/100 reports {coordination_status}; next target: {next_target}"
        )
    }
}
