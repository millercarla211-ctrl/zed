use gpui::SharedString;

use crate::dx_launch_source_audit::DxLaunchSourceAuditSnapshot;

pub(super) fn launch_source_audit_warning(
    snapshot: &DxLaunchSourceAuditSnapshot,
) -> Option<(SharedString, String)> {
    if let Some(issue) = snapshot.first_issue.as_ref() {
        Some(("dx-source-audit-warning".into(), issue.clone()))
    } else if snapshot.risk_review_count > 0 {
        Some((
            "dx-source-audit-risk".into(),
            "Source audit is blocked by risk-review state in another launch repo.".to_string(),
        ))
    } else if !snapshot.template_trust_passed {
        Some((
            "dx-source-audit-template-trust".into(),
            "Template trust scan is not passing.".to_string(),
        ))
    } else if !snapshot.dx_studio_passed {
        Some((
            "dx-source-audit-www-qa".into(),
            "DX Studio WWW QA receipt is not passing.".to_string(),
        ))
    } else {
        None
    }
}
