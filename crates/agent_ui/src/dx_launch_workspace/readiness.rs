use gpui::{AnyElement, App, prelude::*};

use crate::dx_launch_readiness::DxLaunchReadinessSnapshot;

use self::{
    examples::launch_readiness_example_rows, status::launch_readiness_status_row,
    summary::launch_readiness_summary_rows,
};

mod examples;
mod status;
mod summary;
mod warnings;

pub(super) fn launch_readiness_state(snapshot: &DxLaunchReadinessSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .children(launch_readiness_summary_rows(snapshot))
        .child(launch_readiness_status_row(snapshot, cx));

    for row in launch_readiness_example_rows(snapshot) {
        stack = stack.child(row);
    }

    stack.into_any_element()
}
