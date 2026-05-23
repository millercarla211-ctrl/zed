use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_www_launch_evidence::DxWwwLaunchEvidenceSnapshot;

use self::status::www_launch_evidence_status_rows;
use self::warnings::www_launch_evidence_warning;
use super::signal_row;

mod status;
mod warnings;

pub(super) fn www_launch_evidence_state(
    snapshot: &DxWwwLaunchEvidenceSnapshot,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .children(www_launch_evidence_status_rows(snapshot, cx));

    if let Some((id, message)) = www_launch_evidence_warning(snapshot) {
        stack = stack.child(signal_row(id, IconName::Warning, Color::Warning, message));
    }

    stack.into_any_element()
}
