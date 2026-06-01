use gpui::AnyElement;
use ui::{Color, IconName};

use crate::dx_launch_readiness::DxLaunchReadinessSnapshot;

use super::super::signal_row;

pub(super) fn launch_readiness_warning(snapshot: &DxLaunchReadinessSnapshot) -> Option<AnyElement> {
    if let Some(issue) = snapshot.first_issue.as_ref() {
        Some(signal_row(
            "dx-launch-readiness-warning".into(),
            IconName::Warning,
            Color::Warning,
            issue.clone(),
        ))
    } else if snapshot.redaction_requires_review {
        Some(signal_row(
            "dx-launch-readiness-redaction-review".into(),
            IconName::Warning,
            Color::Warning,
            "Launch readiness redaction flags need review.".to_string(),
        ))
    } else if !snapshot.no_command_fanout {
        Some(signal_row(
            "dx-launch-readiness-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "Launch readiness packets report command fanout; keep import blocked.".to_string(),
        ))
    } else {
        None
    }
}
