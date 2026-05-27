use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_launch_source_audit::DxLaunchSourceAuditSnapshot;

use self::status::launch_source_audit_status_rows;
use self::summary::launch_source_audit_summary_rows;
use self::warnings::launch_source_audit_warning;
use super::signal_row;

mod status;
mod summary;
mod warnings;

pub(super) fn launch_source_audit_state(
    snapshot: &DxLaunchSourceAuditSnapshot,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .children(launch_source_audit_summary_rows(snapshot))
        .children(launch_source_audit_status_rows(snapshot, cx));

    if let Some((id, message)) = launch_source_audit_warning(snapshot) {
        stack = stack.child(signal_row(id, IconName::Warning, Color::Warning, message));
    }

    stack.into_any_element()
}
