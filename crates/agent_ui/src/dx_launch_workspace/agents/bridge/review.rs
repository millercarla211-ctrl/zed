use gpui::{AnyElement, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use self::next_action::dx_agent_bridge_next_action;
use self::warnings::dx_agent_bridge_warning_row;

mod next_action;
mod warnings;

pub(super) fn dx_agent_bridge_review_row(snapshot: &DxAgentBridgeSnapshot) -> AnyElement {
    if let Some(review_row) = dx_agent_bridge_warning_row(snapshot) {
        return review_row;
    }

    Label::new(dx_agent_bridge_next_action(snapshot))
        .size(LabelSize::XSmall)
        .color(Color::Muted)
        .truncate()
        .into_any_element()
}
