use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName};

use crate::dx_launch_source_audit::DxLaunchSourceAuditSnapshot;

use super::super::{muted_card, signal_row};

pub(super) fn launch_source_audit_status_rows(
    snapshot: &DxLaunchSourceAuditSnapshot,
    cx: &App,
) -> Vec<AnyElement> {
    let mut rows = Vec::new();

    if !snapshot.root_exists {
        rows.push(muted_card(
            format!("Missing source audit root: {}", snapshot.root.display()),
            cx,
        ));
    } else if !snapshot.latest_present {
        rows.push(muted_card(
            format!(
                "No source audit latest receipt at {}",
                snapshot.latest_path.display()
            ),
            cx,
        ));
    } else if !snapshot.schema_valid {
        rows.push(signal_row(
            "dx-source-audit-invalid".into(),
            IconName::Warning,
            Color::Warning,
            snapshot
                .last_error
                .clone()
                .unwrap_or_else(|| "Source audit receipt schema is not valid.".to_string()),
        ));
    }

    if !snapshot.markdown_present {
        rows.push(muted_card(
            format!(
                "Missing source audit markdown summary: {}",
                snapshot.markdown_path.display()
            ),
            cx,
        ));
    }

    if !snapshot.dx_studio_qa_present {
        rows.push(muted_card(
            format!(
                "Missing DX Studio QA receipt: {}",
                snapshot.dx_studio_qa_path.display()
            ),
            cx,
        ));
    }

    rows
}
