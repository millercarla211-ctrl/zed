use gpui::{AnyElement, App, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_launch_status::DxLaunchStatusSnapshot;

use self::rows::launch_status_valid_detail_rows;
use self::status::launch_status_status_rows;
use super::launch_status_labels::{launch_status_command_label, launch_status_summary_label};
use super::{metric_row, yes_no};

mod rows;
mod status;
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
        ))
        .children(launch_status_status_rows(snapshot, cx));

    if snapshot.schema_valid {
        stack = stack.children(launch_status_valid_detail_rows(snapshot));
    }

    stack.into_any_element()
}
