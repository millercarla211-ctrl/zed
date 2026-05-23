use gpui::SharedString;

use crate::dx_launch_status::DxLaunchStatusSnapshot;

pub(super) fn launch_status_warning(
    snapshot: &DxLaunchStatusSnapshot,
) -> Option<(SharedString, String)> {
    if !snapshot.schema_valid {
        Some((
            "dx-launch-status-invalid".into(),
            snapshot
                .last_error
                .clone()
                .unwrap_or_else(|| "Launch status receipt schema is invalid".to_string()),
        ))
    } else if snapshot.redaction_requires_review {
        Some((
            "dx-launch-status-redaction-review".into(),
            "Launch status redaction flags need review".to_string(),
        ))
    } else {
        snapshot
            .last_error
            .as_ref()
            .map(|error| ("dx-launch-status-warning".into(), error.clone()))
    }
}
