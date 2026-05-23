use gpui::{AnyElement, SharedString};
use ui::{Color, IconName};

use crate::dx_check_panel::{DxCheckPanelNotice, DxCheckPanelQuickFix, DxCheckPanelSection};

use super::super::{metric_row, signal_row};

pub(super) fn check_section_row(section: &DxCheckPanelSection) -> AnyElement {
    let score = match (section.score, section.max_score) {
        (Some(score), Some(max_score)) => {
            let estimated = if section.estimated { " est" } else { "" };
            format!(
                "{score}/{max_score} {status}{estimated}",
                status = section.status
            )
        }
        _ => section.status.clone(),
    };
    metric_row(section.title.clone(), score)
}

pub(super) fn check_blocker_row(ix: usize, blocker: &DxCheckPanelNotice) -> AnyElement {
    signal_row(
        SharedString::from(format!("dx-check-panel-blocker-{ix}")),
        IconName::Warning,
        Color::Warning,
        format!("{}: {}", blocker.code, blocker.message),
    )
}

pub(super) fn check_warning_rows(ix: usize, warning: &DxCheckPanelNotice) -> Vec<AnyElement> {
    let mut rows = vec![signal_row(
        SharedString::from(format!("dx-check-panel-warning-{ix}")),
        IconName::Warning,
        Color::Warning,
        format!("{}: {}", warning.code, warning.message),
    )];
    if let Some(next_action) = warning.next_action.as_ref() {
        rows.push(metric_row(
            format!("Warn next {}", ix + 1),
            next_action.clone(),
        ));
    }
    rows
}

pub(super) fn check_quick_fix_rows(ix: usize, fix: &DxCheckPanelQuickFix) -> Vec<AnyElement> {
    let approval = if fix.requires_user_approval {
        "approval required"
    } else {
        "no approval"
    };
    let writes_receipts = if fix.writes_receipts {
        "writes receipts"
    } else {
        "no receipt write"
    };
    let mut detail = format!(
        "{} - risk: {}; {}; {}",
        fix.next_action, fix.risk_level, approval, writes_receipts
    );
    if let Some(command) = fix.command.as_ref() {
        detail.push_str(&format!(" - {command}"));
    }

    vec![
        metric_row(format!("Fix {}", ix + 1), fix.label.clone()),
        metric_row(format!("Fix next {}", ix + 1), detail),
    ]
}
