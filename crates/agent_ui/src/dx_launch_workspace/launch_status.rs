use gpui::{AnyElement, App, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_launch_status::DxLaunchStatusSnapshot;

use self::rows::launch_status_valid_detail_rows;
use self::warnings::launch_status_warning;
use super::launch_status_labels::{
    launch_status_command_label, launch_status_next_action_label, launch_status_summary_label,
};
use super::{metric_row, muted_card, signal_row, yes_no};

mod rows;
mod warnings;

pub(super) fn launch_status_state(snapshot: &DxLaunchStatusSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Status", snapshot.status.clone()))
        .child(
            Label::new(launch_status_summary_label(&snapshot.operator_summary))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row(
            "Agents",
            format!(
                "{} / {} connected, {} need setup ({})",
                snapshot.agents.connected_accounts,
                snapshot.agents.configured_accounts,
                snapshot.agents.accounts_needing_connection,
                snapshot.agents.status
            ),
        ))
        .child(metric_row(
            "Agent Work",
            format!(
                "{} automation(s), {} active task(s), {} QR-ready",
                snapshot.agents.automation_count,
                snapshot.agents.active_task_count,
                snapshot.agents.qr_connect_supported
            ),
        ))
        .child(metric_row(
            "Tokens",
            format!(
                "{} / {} ({})",
                snapshot.tokens.budget_state,
                snapshot.tokens.estimated_tokens,
                snapshot.tokens.status
            ),
        ))
        .child(metric_row(
            "Budget",
            format!(
                "{} soft / {} hard",
                snapshot.tokens.soft_budget_tokens, snapshot.tokens.hard_budget_tokens
            ),
        ))
        .child(metric_row(
            "Discovery",
            format!(
                "{} / manifest {} / binary {}",
                snapshot.discovery.status,
                yes_no(snapshot.discovery.www_manifest_present),
                yes_no(snapshot.discovery.configured_binary_present)
            ),
        ))
        .child(metric_row(
            "Templates",
            launch_status_command_label(
                &snapshot.discovery.templates_command,
                "No template command",
            ),
        ))
        .child(metric_row(
            "Packages",
            launch_status_command_label(&snapshot.discovery.packages_command, "No package command"),
        ));

    if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!("Missing launch receipts: {}", snapshot.root.display()),
            cx,
        ));
    } else if !snapshot.latest_present {
        stack = stack.child(muted_card(
            format!(
                "Run dx launch status --json to write {}",
                snapshot.latest_path.display()
            ),
            cx,
        ));
    } else if let Some((id, message)) = launch_status_warning(snapshot) {
        stack = stack.child(signal_row(id, IconName::Warning, Color::Warning, message));
    } else {
        stack = stack.child(
            Label::new(launch_status_next_action_label(&snapshot.next_action))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if snapshot.schema_valid {
        stack = stack.children(launch_status_valid_detail_rows(snapshot));
    }

    stack.into_any_element()
}
