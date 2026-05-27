use gpui::{AnyElement, App, prelude::*};
use ui::prelude::*;

use crate::dx_source_sets::DxSourceAttachmentSummary;

use super::super::{metric_row, muted_card};

pub(crate) fn source_attachment_state(summary: &DxSourceAttachmentSummary, cx: &App) -> AnyElement {
    let state = if summary.attachable_sources == 0 {
        "No attach-ready sources".to_string()
    } else {
        format!("{} ready", summary.attachable_sources)
    };

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Attach-ready", state))
        .child(metric_row(
            "Workspace roots",
            summary.workspace_roots.to_string(),
        ))
        .child(metric_row(
            "Managed receipts",
            summary.managed_receipts.to_string(),
        ));

    if summary.produced_files > 0 {
        stack = stack.child(metric_row(
            "Produced media",
            summary.produced_files.to_string(),
        ));
    }

    if summary.restore_previews > 0 {
        stack = stack.child(metric_row(
            "Restore previews",
            summary.restore_previews.to_string(),
        ));
    }

    if summary.attachable_sources == 0 {
        stack = stack.child(muted_card(
            "Create a source-pack or media receipt first",
            cx,
        ));
    }

    stack.into_any_element()
}
