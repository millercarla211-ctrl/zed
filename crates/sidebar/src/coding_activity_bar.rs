use gpui::{AnyElement, IntoElement, StatefulInteractiveElement as _, prelude::*};
use ui::prelude::*;

pub(crate) fn render_coding_activity_bar(
    primary_actions: Vec<AnyElement>,
    secondary_actions: Vec<AnyElement>,
) -> impl IntoElement {
    v_flex()
        .id("workspace-sidebar-activity-bar")
        .w_full()
        .h_full()
        .items_center()
        .justify_between()
        .overflow_hidden()
        .py_1()
        .child(
            v_flex()
                .id("workspace-sidebar-primary-actions")
                .min_h_0()
                .flex_1()
                .items_center()
                .gap_1()
                .overflow_y_scroll()
                .children(primary_actions),
        )
        .child(
            v_flex()
                .flex_none()
                .items_center()
                .gap_1()
                .children(secondary_actions),
        )
}
