use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::prelude::*;

use crate::dx_launch_binary_cache::{DxBinaryCacheRow, DxBinaryCacheSnapshot};

use super::binary_cache_labels::{
    binary_cache_next_action_label, binary_cache_row_detail_label, binary_cache_row_path_label,
    binary_cache_summary_label,
};
use super::metric_row;

pub(super) fn binary_cache_state(snapshot: &DxBinaryCacheSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Status", snapshot.status.clone()))
        .child(
            Label::new(binary_cache_summary_label(&snapshot.operator_summary))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row(
            "Next",
            binary_cache_next_action_label(&snapshot.next_action),
        ));

    for (ix, row) in snapshot.rows.iter().take(4).enumerate() {
        stack = stack.child(binary_cache_row(
            SharedString::from(format!("dx-binary-cache-row-{ix}")),
            row,
            cx,
        ));
    }

    stack.into_any_element()
}

fn binary_cache_row(id: SharedString, row: &DxBinaryCacheRow, cx: &App) -> AnyElement {
    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(row.label.clone(), row.state.clone()))
        .child(
            Label::new(binary_cache_row_detail_label(&row.detail))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(
            Label::new(binary_cache_row_path_label(&row.path))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}
