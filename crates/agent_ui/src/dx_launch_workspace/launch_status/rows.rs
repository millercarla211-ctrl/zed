use gpui::{AnyElement, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_launch_status::DxLaunchStatusSnapshot;

use super::super::launch_status_labels::{
    launch_status_next_action_label, launch_status_optional_summary,
};
use super::super::metric_row;

pub(super) fn launch_status_valid_detail_rows(
    snapshot: &DxLaunchStatusSnapshot,
) -> Vec<AnyElement> {
    let mut rows = vec![
        metric_row(
            "Agent Next",
            launch_status_next_action_label(&snapshot.agents.next_action),
        ),
        metric_row(
            "Token Next",
            launch_status_next_action_label(&snapshot.tokens.next_action),
        ),
        metric_row(
            "Discovery Next",
            launch_status_next_action_label(&snapshot.discovery.next_action),
        ),
    ];

    if let Some(redaction_summary) = launch_status_optional_summary(&snapshot.redaction_summary) {
        rows.push(
            Label::new(redaction_summary)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate()
                .into_any_element(),
        );
    }

    rows
}
