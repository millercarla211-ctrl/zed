use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::prelude::*;

use crate::dx_source_sets::{DxSourceSet, DxSourceSetSnapshot};

pub(super) use self::attachments::source_attachment_state;
pub(super) use self::receipts::receipt_source_state;
use self::rows::source_item_row;
use super::{DxSourceRowControl, metric_row, muted_card};

mod attachments;
mod drilldowns;
mod kinds;
mod receipts;
mod rows;

pub(super) fn source_set_stack(
    snapshot: &DxSourceSetSnapshot,
    mut source_row_controls: Vec<DxSourceRowControl>,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex().gap_1();

    if snapshot.total_sources == 0 {
        stack = stack.child(muted_card("No workspace source", cx));
    } else {
        for (ix, set) in snapshot.sets.iter().enumerate() {
            stack = stack.child(source_set_card(
                SharedString::from(format!("source-set-{ix}")),
                set,
                &mut source_row_controls,
                cx,
            ));
        }
    }

    stack.into_any_element()
}

fn source_set_card(
    id: SharedString,
    set: &DxSourceSet,
    source_row_controls: &mut Vec<DxSourceRowControl>,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .id(id)
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(cx.theme().colors().border_variant)
        .px_2()
        .py_1()
        .child(metric_row(set.label, set.status.clone()));

    if set.sources.is_empty() {
        return stack.into_any_element();
    }

    let set_id = set.label.to_ascii_lowercase().replace(' ', "-");
    for (ix, source) in set.sources.iter().take(3).enumerate() {
        let source_row_control = take_source_row_control(source_row_controls, &source.path);
        stack = stack.child(source_item_row(
            SharedString::from(format!("{set_id}-source-{ix}")),
            source,
            source_row_control,
            cx,
        ));
    }

    stack.into_any_element()
}

fn take_source_row_control(
    source_row_controls: &mut Vec<DxSourceRowControl>,
    source_path: &str,
) -> Option<AnyElement> {
    source_row_controls
        .iter()
        .position(|control| control.source_path == source_path)
        .map(|index| source_row_controls.remove(index).element)
}
