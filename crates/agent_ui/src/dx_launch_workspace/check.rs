use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};

use self::rows::{check_blocker_row, check_quick_fix_rows, check_section_row, check_warning_rows};
use self::summary::check_summary_rows;
use crate::dx_check_score::DxCheckScoreSnapshot;

use super::{metric_row, muted_card, signal_row, source_row};

mod rows;
mod summary;

pub(super) fn check_score_state(snapshot: &DxCheckScoreSnapshot, cx: &App) -> AnyElement {
    let panel = &snapshot.panel;
    let mut stack = v_flex().gap_1().children(check_summary_rows(panel));

    if !panel.receipt_present {
        stack = stack.child(muted_card(
            format!("Missing receipt: {}", panel.receipt_path.display()),
            cx,
        ));
    } else if let Some(error) = panel.receipt_error.as_ref() {
        stack = stack.child(signal_row(
            "dx-check-panel-error".into(),
            IconName::Warning,
            Color::Warning,
            error.clone(),
        ));
    }

    for section in panel.sections.iter().take(5) {
        stack = stack.child(check_section_row(section));
    }

    for (ix, blocker) in panel.blockers.iter().take(2).enumerate() {
        stack = stack.child(check_blocker_row(ix, blocker));
    }

    for (ix, warning) in panel.warnings.iter().take(2).enumerate() {
        stack = stack.children(check_warning_rows(ix, warning));
    }

    for (ix, fix) in panel.quick_fixes.iter().take(2).enumerate() {
        stack = stack.children(check_quick_fix_rows(ix, fix));
    }

    stack = stack
        .child(metric_row("Next", panel.next_action.clone()))
        .child(metric_row(
            "Rail score",
            format!("{}/100 {}", snapshot.score, snapshot.state),
        ));

    for item in snapshot.items.iter().take(4) {
        stack = stack.child(metric_row(item.label, item.state.clone()));
    }

    for (ix, blocker) in snapshot.blockers.iter().take(1).enumerate() {
        stack = stack.child(source_row(
            SharedString::from(format!("dx-check-blocker-{ix}")),
            IconName::ListTodo,
            blocker.clone(),
            cx,
        ));
    }

    stack.into_any_element()
}
