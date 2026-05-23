use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::prelude::*;

use self::rows::tool_history_bucket;
use crate::dx_receipt_history::DxToolHistorySnapshot;

mod rows;

pub(super) fn tool_history_state(snapshot: &DxToolHistorySnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex().gap_1();

    for (ix, bucket) in snapshot.buckets.iter().enumerate() {
        stack = stack.child(tool_history_bucket(
            SharedString::from(format!("dx-tool-history-{ix}")),
            bucket,
            cx,
        ));
    }

    stack.into_any_element()
}
