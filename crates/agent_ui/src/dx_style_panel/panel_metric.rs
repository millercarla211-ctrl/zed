use gpui::{IntoElement, SharedString, div, px};
use ui::prelude::*;

pub(super) fn metric(
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
) -> impl IntoElement {
    h_flex()
        .justify_between()
        .gap_2()
        .min_w_0()
        .child(
            Label::new(label.into())
                .size(LabelSize::XSmall)
                .color(Color::Muted),
        )
        .child(
            div()
                .max_w(px(190.0))
                .child(Label::new(value.into()).size(LabelSize::XSmall).truncate()),
        )
}
