use std::path::Path;

use gpui::{AnyElement, App};

use crate::dx_launch_audit::DxLaunchAuditSnapshot;

use super::super::muted_card;

pub(super) fn launch_audit_status_rows(
    snapshot: &DxLaunchAuditSnapshot,
    cx: &App,
) -> Vec<AnyElement> {
    let mut rows = Vec::new();

    if !snapshot.root_exists {
        rows.push(muted_card(
            format!("Missing launch example root: {}", snapshot.root.display()),
            cx,
        ));
    }

    for (present, path, label) in [
        (snapshot.schemas_present, &snapshot.schemas_path, "schemas"),
        (
            snapshot.fixtures_present,
            &snapshot.fixtures_path,
            "fixtures",
        ),
        (snapshot.smoke_present, &snapshot.smoke_path, "smoke"),
        (snapshot.status_present, &snapshot.status_path, "status"),
    ] {
        push_missing_packet_row(&mut rows, present, path, label, cx);
    }

    rows
}

fn push_missing_packet_row(
    rows: &mut Vec<AnyElement>,
    present: bool,
    path: &Path,
    label: &str,
    cx: &App,
) {
    if !present {
        rows.push(muted_card(
            format!("Missing {label}: {}", path.display()),
            cx,
        ));
    }
}
