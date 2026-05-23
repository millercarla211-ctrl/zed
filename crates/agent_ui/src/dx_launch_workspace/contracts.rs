use gpui::{AnyElement, App, prelude::*};
use ui::{Color, prelude::*};

use crate::dx_launch_contracts::DxLaunchContractSnapshot;

use self::status::launch_contract_status_rows;
use super::{bounded_items, metric_row};

mod status;

pub(super) fn launch_contract_state(snapshot: &DxLaunchContractSnapshot, cx: &App) -> AnyElement {
    let first_packet = bounded_items(&snapshot.first_packets, 3, "No packet commands");
    let startup = bounded_items(&snapshot.startup_commands, 3, "No startup commands");
    let diagnostics = bounded_items(&snapshot.diagnostics_commands, 3, "No diagnostics commands");
    let details = bounded_items(&snapshot.detail_commands, 3, "No detail commands");
    let refresh = snapshot
        .refresh_command
        .as_deref()
        .unwrap_or("dx launch status --json");
    let cached = snapshot
        .cached_receipt_path
        .as_deref()
        .unwrap_or(".dx/receipts/launch/status-latest.json");

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Status", snapshot.status.clone()))
        .child(
            Label::new(snapshot.operator_summary.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row(
            "Packets",
            format!(
                "{} packet(s), {} fixture familie(s)",
                snapshot.packet_count, snapshot.fixture_family_count
            ),
        ))
        .child(metric_row(
            "Commands",
            format!(
                "{} command(s), {} action(s)",
                snapshot.command_count, snapshot.action_count
            ),
        ))
        .child(metric_row(
            "Metadata",
            format!(
                "{} metadata-only / {} fanout",
                snapshot.metadata_only_count, snapshot.command_fanout_count
            ),
        ))
        .child(metric_row(
            "Confirmations",
            snapshot.confirmation_action_count.to_string(),
        ))
        .child(metric_row("Refresh", refresh.to_string()))
        .child(metric_row("Cached", cached.to_string()))
        .child(metric_row("Startup", startup))
        .child(metric_row("Diagnostics", diagnostics))
        .child(metric_row("Details", details))
        .child(metric_row("First Packets", first_packet));

    stack = stack.children(launch_contract_status_rows(snapshot, cx));

    stack.into_any_element()
}
