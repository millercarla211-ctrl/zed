use gpui::{AnyElement, App, prelude::*};

use crate::dx_launch_status::DxLaunchStatusSnapshot;

use self::rows::launch_status_valid_detail_rows;
use self::status::launch_status_status_rows;
use self::summary::launch_status_summary_rows;

mod rows;
mod status;
mod summary;
mod warnings;

pub(super) fn launch_status_state(snapshot: &DxLaunchStatusSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .children(launch_status_summary_rows(snapshot))
        .children(launch_status_status_rows(snapshot, cx));

    if snapshot.schema_valid {
        stack = stack.children(launch_status_valid_detail_rows(snapshot));
    }

    stack.into_any_element()
}
