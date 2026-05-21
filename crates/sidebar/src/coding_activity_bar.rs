use gpui::{AnyElement, IntoElement, prelude::*};
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
        .py_1()
        .child(v_flex().items_center().gap_1().children(primary_actions))
        .child(v_flex().items_center().gap_1().children(secondary_actions))
}
