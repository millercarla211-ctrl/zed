use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_launch_audit::DxLaunchAuditSnapshot;

use self::status::launch_audit_status_rows;
use self::summary::launch_audit_summary_rows;
use self::warnings::launch_audit_warning;
use super::signal_row;

mod status;
mod summary;
mod warnings;

pub(super) fn launch_audit_state(snapshot: &DxLaunchAuditSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .children(launch_audit_summary_rows(snapshot))
        .children(launch_audit_status_rows(snapshot, cx));

    if let Some((id, message)) = launch_audit_warning(snapshot) {
        stack = stack.child(signal_row(id, IconName::Warning, Color::Warning, message));
    } else {
        stack = stack.child(
            Label::new(snapshot.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    stack.into_any_element()
}
