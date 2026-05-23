use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_launch_contracts::DxLaunchContractSnapshot;

use super::super::{metric_row, muted_card, signal_row};

pub(super) fn launch_contract_status_rows(
    snapshot: &DxLaunchContractSnapshot,
    cx: &App,
) -> Vec<AnyElement> {
    let mut rows = Vec::new();

    if !snapshot.manifest_present {
        rows.push(muted_card(
            format!(
                "Missing import manifest: {}",
                snapshot.manifest_path.display()
            ),
            cx,
        ));
    }
    if !snapshot.handoff_present {
        rows.push(muted_card(
            format!(
                "Missing handoff packet: {}",
                snapshot.handoff_path.display()
            ),
            cx,
        ));
    }

    if let Some(error) = snapshot.last_error.as_ref() {
        rows.push(signal_row(
            "dx-launch-contract-warning".into(),
            IconName::Warning,
            Color::Warning,
            error.clone(),
        ));
    } else if snapshot.redaction_requires_review {
        rows.push(signal_row(
            "dx-launch-contract-redaction-review".into(),
            IconName::Warning,
            Color::Warning,
            "Launch handoff redaction flags need review.".to_string(),
        ));
    } else if !snapshot.no_command_fanout {
        rows.push(signal_row(
            "dx-launch-contract-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "Launch handoff reports command fanout; keep GPUI import blocked.".to_string(),
        ));
    } else {
        rows.push(
            Label::new(snapshot.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate()
                .into_any_element(),
        );
    }

    if let Some(action) = snapshot.first_action.as_ref() {
        rows.push(metric_row("First Action", action.clone()));
    }

    rows
}
