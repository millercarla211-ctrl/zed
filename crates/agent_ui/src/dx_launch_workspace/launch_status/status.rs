use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_launch_status::DxLaunchStatusSnapshot;

use super::super::launch_status_labels::launch_status_next_action_label;
use super::super::{muted_card, signal_row};
use super::warnings::launch_status_warning;

pub(super) fn launch_status_status_rows(
    snapshot: &DxLaunchStatusSnapshot,
    cx: &App,
) -> Vec<AnyElement> {
    if !snapshot.root_exists {
        return vec![muted_card(
            format!("Missing launch receipts: {}", snapshot.root.display()),
            cx,
        )];
    }

    if !snapshot.latest_present {
        return vec![muted_card(
            format!(
                "Run dx launch status --json to write {}",
                snapshot.latest_path.display()
            ),
            cx,
        )];
    }

    if let Some((id, message)) = launch_status_warning(snapshot) {
        return vec![signal_row(id, IconName::Warning, Color::Warning, message)];
    }

    vec![
        Label::new(launch_status_next_action_label(&snapshot.next_action))
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate()
            .into_any_element(),
    ]
}
