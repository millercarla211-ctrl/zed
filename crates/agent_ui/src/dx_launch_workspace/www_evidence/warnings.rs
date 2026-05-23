use gpui::SharedString;

use crate::dx_www_launch_evidence::DxWwwLaunchEvidenceSnapshot;

pub(super) fn www_launch_evidence_warning(
    snapshot: &DxWwwLaunchEvidenceSnapshot,
) -> Option<(SharedString, String)> {
    if let Some(issue) = snapshot.first_issue.as_ref() {
        Some(("dx-www-evidence-warning".into(), issue.clone()))
    } else if snapshot.present_count < snapshot.expected_count {
        Some((
            "dx-www-evidence-partial".into(),
            "DX-WWW release evidence is partial; keep runtime-green claims gated.".to_string(),
        ))
    } else {
        None
    }
}
