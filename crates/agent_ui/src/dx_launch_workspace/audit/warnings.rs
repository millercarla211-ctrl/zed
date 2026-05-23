use gpui::SharedString;

use crate::dx_launch_audit::DxLaunchAuditSnapshot;

pub(super) fn launch_audit_warning(
    snapshot: &DxLaunchAuditSnapshot,
) -> Option<(SharedString, String)> {
    if let Some(issue) = snapshot.first_issue.as_ref() {
        Some(("dx-launch-audit-warning".into(), issue.clone()))
    } else if snapshot.redaction_requires_review {
        Some((
            "dx-launch-audit-redaction-review".into(),
            "Launch audit redaction flags need review.".to_string(),
        ))
    } else if snapshot.command_fanout_count > 0 {
        Some((
            "dx-launch-audit-fanout-review".into(),
            "Launch audit reports command fanout; keep final handoff blocked.".to_string(),
        ))
    } else {
        None
    }
}
