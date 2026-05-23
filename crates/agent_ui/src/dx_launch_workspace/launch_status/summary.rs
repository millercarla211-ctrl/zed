use gpui::{AnyElement, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_launch_status::DxLaunchStatusSnapshot;

use super::super::launch_status_labels::{
    launch_status_command_label, launch_status_summary_label,
};
use super::super::{metric_row, yes_no};

pub(super) fn launch_status_summary_rows(snapshot: &DxLaunchStatusSnapshot) -> Vec<AnyElement> {
    vec![
        metric_row("Status", snapshot.status.clone()),
        Label::new(launch_status_summary_label(&snapshot.operator_summary))
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate()
            .into_any_element(),
        metric_row(
            "Agents",
            format!(
                "{} / {} connected, {} need setup ({})",
                snapshot.agents.connected_accounts,
                snapshot.agents.configured_accounts,
                snapshot.agents.accounts_needing_connection,
                snapshot.agents.status
            ),
        ),
        metric_row(
            "Agent Work",
            format!(
                "{} automation(s), {} active task(s), {} QR-ready",
                snapshot.agents.automation_count,
                snapshot.agents.active_task_count,
                snapshot.agents.qr_connect_supported
            ),
        ),
        metric_row(
            "Tokens",
            format!(
                "{} / {} ({})",
                snapshot.tokens.budget_state,
                snapshot.tokens.estimated_tokens,
                snapshot.tokens.status
            ),
        ),
        metric_row(
            "Budget",
            format!(
                "{} soft / {} hard",
                snapshot.tokens.soft_budget_tokens, snapshot.tokens.hard_budget_tokens
            ),
        ),
        metric_row(
            "Discovery",
            format!(
                "{} / manifest {} / binary {}",
                snapshot.discovery.status,
                yes_no(snapshot.discovery.www_manifest_present),
                yes_no(snapshot.discovery.configured_binary_present)
            ),
        ),
        metric_row(
            "Templates",
            launch_status_command_label(
                &snapshot.discovery.templates_command,
                "No template command",
            ),
        ),
        metric_row(
            "Packages",
            launch_status_command_label(&snapshot.discovery.packages_command, "No package command"),
        ),
    ]
}
