use gpui::{AnyElement, App, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_launch_readiness::DxLaunchReadinessSnapshot;

use super::super::muted_card;
use super::warnings::launch_readiness_warning;

pub(super) fn launch_readiness_status_row(
    snapshot: &DxLaunchReadinessSnapshot,
    cx: &App,
) -> AnyElement {
    if !snapshot.root_exists {
        return muted_card(
            format!(
                "Missing source-owned launch examples: {}",
                snapshot.root.display()
            ),
            cx,
        );
    }

    if let Some(warning) = launch_readiness_warning(snapshot) {
        return warning;
    }

    Label::new(snapshot.next_action.clone())
        .size(LabelSize::XSmall)
        .color(Color::Muted)
        .truncate()
        .into_any_element()
}
