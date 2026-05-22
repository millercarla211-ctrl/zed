use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};

use crate::dx_agent_bridge::{
    DxAgentAutomation, DxAgentBridgeSnapshot, DxAgentReceipt, DxAgentRowAction,
    DxAgentSocialAccount, DxAgentSocialActionSummary,
};

use super::{metric_row, muted_card, signal_row, yes_no};

mod providers;
pub(super) use providers::dx_agent_provider_state;

pub(super) fn dx_agent_bridge_state(snapshot: &DxAgentBridgeSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Bridge", snapshot.status.clone()))
        .child(metric_row("CLI", snapshot.cli_path.clone()))
        .child(metric_row(
            "Accounts",
            format!(
                "{} connected / {} configured",
                snapshot.connected_accounts_summary.connected,
                snapshot.connected_accounts_summary.configured
            ),
        ))
        .child(metric_row(
            "Automations",
            snapshot.automation_count.to_string(),
        ))
        .child(metric_row("Tasks", snapshot.active_task_count.to_string()))
        .child(metric_row(
            "Catalog",
            if snapshot.catalog.present && !snapshot.catalog.stale {
                "fast".to_string()
            } else {
                "fallback".to_string()
            },
        ))
        .child(metric_row(
            "Contract",
            snapshot.contract_summary.status.clone(),
        ))
        .child(metric_row(
            "Commands",
            snapshot.contract_summary.command_count.to_string(),
        ))
        .child(metric_row(
            "Receipts",
            snapshot.contract_summary.receipt_count.to_string(),
        ))
        .child(metric_row(
            "Contract Catalog",
            format!(
                "{} / {} receipt(s)",
                snapshot.contract_summary.provider_catalog_source,
                snapshot.contract_summary.provider_catalog_receipt_count
            ),
        ))
        .child(metric_row(
            "Redaction",
            if snapshot.contract_summary.redaction_requires_review {
                "review".to_string()
            } else {
                snapshot.contract_summary.redaction_summary.clone()
            },
        ))
        .child(metric_row("Import", snapshot.import_summary.status.clone()))
        .child(metric_row(
            "Release",
            format!(
                "{} / {} warning(s) / {} blocker(s)",
                snapshot.import_summary.release_gate_status,
                snapshot.import_summary.release_gate_warning_count,
                snapshot.import_summary.release_gate_failed_count
            ),
        ))
        .child(metric_row(
            "Action Map",
            format!(
                "{} / {}",
                snapshot.import_summary.action_map_status,
                snapshot.import_summary.recovery_counts.label()
            ),
        ))
        .child(metric_row(
            "Recovery",
            format!(
                "{} / {} fixture(s) / {}",
                snapshot.import_summary.recovery_controls_status,
                snapshot.import_summary.recovery_fixture_count,
                snapshot.import_summary.recovery_counts.label()
            ),
        ))
        .child(metric_row(
            "Command Fanout",
            if snapshot.import_summary.no_command_fanout {
                "none".to_string()
            } else {
                "review".to_string()
            },
        ))
        .child(metric_row(
            "Last Action",
            if snapshot.action_error.present {
                snapshot.action_error.status.clone()
            } else {
                "ready".to_string()
            },
        ))
        .child(metric_row("Gate", snapshot.release_gate.status.clone()))
        .child(metric_row(
            "Acceptance",
            format!(
                "{} / {} passed",
                snapshot.release_gate.passed_count, snapshot.release_gate.acceptance_count
            ),
        ))
        .child(metric_row(
            "Gate Warnings",
            format!(
                "{} warning(s) / {} blocker(s)",
                snapshot.release_gate.warning_count, snapshot.release_gate.failed_count
            ),
        ))
        .child(metric_row(
            "Gate Receipts",
            format!(
                "{} / {}",
                snapshot.release_gate.receipt_count, snapshot.release_gate.receipt_inbox_status
            ),
        ))
        .child(metric_row(
            "Gate Packets",
            format!(
                "{} packet(s) / {} fixture(s)",
                snapshot.release_gate.packet_count, snapshot.release_gate.fixture_family_count
            ),
        ))
        .child(metric_row(
            "Gate Retention",
            format!(
                "{} / {} overflow",
                snapshot.release_gate.retention_preview_status,
                snapshot.release_gate.retained_run_overflow_count
            ),
        ))
        .child(metric_row(
            "Gate Action",
            format!(
                "{} / {}",
                snapshot.release_gate.action_map_status,
                snapshot.release_gate.recovery_counts.label()
            ),
        ))
        .child(metric_row(
            "Gate Recovery",
            format!(
                "{} via {}, {} fixture(s), {}",
                snapshot.release_gate.recovery_controls_status,
                snapshot.release_gate.recovery_render_first,
                snapshot.release_gate.recovery_fixture_count,
                snapshot.release_gate.recovery_counts.label()
            ),
        ))
        .child(metric_row(
            "Gate Fanout",
            if snapshot.release_gate.no_command_fanout {
                "none".to_string()
            } else {
                "review".to_string()
            },
        ))
        .child(metric_row(
            "Receipt Index",
            snapshot.receipt_index.status.clone(),
        ))
        .child(metric_row(
            "Receipt Rows",
            format!(
                "{} / {}",
                snapshot.receipt_index.returned_receipt_count, snapshot.receipt_index.receipt_count
            ),
        ));

    if !snapshot.enabled {
        stack = stack.child(muted_card("Disabled in Zed settings", cx));
    } else if snapshot.action_error.present {
        let error = snapshot
            .action_error
            .error
            .clone()
            .unwrap_or_else(|| "DX Agents action failed".to_string());
        stack = stack.child(signal_row(
            "dx-agent-action-error".into(),
            IconName::Warning,
            Color::Warning,
            format!("Last DX Agents action failed: {error}"),
        ));
    } else if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!("Missing receipts: {}", snapshot.receipt_root.display()),
            cx,
        ));
    } else if !snapshot.contract_summary.present {
        stack = stack.child(muted_card("Run dx agents contract --json", cx));
    }
    if snapshot.enabled && snapshot.root_exists && !snapshot.import_summary.present {
        stack = stack.child(muted_card("Run dx agents import-summary --json", cx));
    }
    if snapshot.enabled && snapshot.root_exists && !snapshot.release_gate.present {
        stack = stack.child(muted_card("Run dx agents release-gate --json", cx));
    }

    if let Some(receipt) = snapshot.latest_receipts.first() {
        stack = stack.child(metric_row("Latest", receipt.clone()));
    }
    if snapshot.action_error.present && !snapshot.action_error.command.is_empty() {
        stack = stack.child(metric_row(
            "Failed Command",
            snapshot.action_error.command.clone(),
        ));
    }
    if snapshot.action_error.redaction_requires_review {
        stack = stack.child(signal_row(
            "dx-agent-action-error-redaction".into(),
            IconName::Warning,
            Color::Warning,
            snapshot.action_error.redaction_summary.clone(),
        ));
    }
    if let Some(command) = snapshot.contract_summary.commands.first() {
        stack = stack.child(metric_row("Command", command.clone()));
    } else if snapshot.contract_summary.present {
        stack = stack.child(metric_row(
            "Catalog Regen",
            snapshot.contract_summary.safe_regeneration_command.clone(),
        ));
    }
    if let Some(command) = snapshot.import_summary.recovery_commands.first() {
        stack = stack.child(metric_row("Import Command", command.clone()));
    }
    if let Some(row) = snapshot.release_gate.acceptance_rows.first() {
        stack = stack.child(metric_row("Gate Row", row.clone()));
    }

    if let Some(reason) = snapshot.release_gate.blocking_reasons.first() {
        stack = stack.child(signal_row(
            "dx-agent-release-gate-blocker".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents release gate blocker: {reason}"),
        ));
    } else if snapshot.release_gate.present && !snapshot.release_gate.no_command_fanout {
        stack = stack.child(signal_row(
            "dx-agent-release-gate-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents release gate reports command fanout; keep bridge import blocked."
                .to_string(),
        ));
    } else if let Some(reason) = snapshot.release_gate.warning_reasons.first() {
        stack = stack.child(signal_row(
            "dx-agent-release-gate-warning".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents release gate warning: {reason}"),
        ));
    } else if let Some(reason) = snapshot.import_summary.blocking_reasons.first() {
        stack = stack.child(signal_row(
            "dx-agent-import-summary-blocker".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents import summary blocker: {reason}"),
        ));
    } else if snapshot.import_summary.present && !snapshot.import_summary.no_command_fanout {
        stack = stack.child(signal_row(
            "dx-agent-import-summary-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents import summary reports command fanout; keep recovery controls disabled."
                .to_string(),
        ));
    } else if let Some(reason) = snapshot.import_summary.warning_reasons.first() {
        stack = stack.child(signal_row(
            "dx-agent-import-summary-warning".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents import summary warning: {reason}"),
        ));
    } else if snapshot.contract_summary.redaction_requires_review {
        stack = stack.child(signal_row(
            "dx-agent-contract-redaction-review".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents bridge contract reports redaction flags that need review.".to_string(),
        ));
    } else if let Some(error) = snapshot.last_error.as_ref() {
        stack = stack.child(signal_row(
            "dx-agent-bridge-error".into(),
            IconName::Warning,
            Color::Warning,
            error.clone(),
        ));
    } else {
        let next_action = if snapshot.release_gate.present {
            snapshot.release_gate.next_action.clone()
        } else if snapshot.import_summary.present {
            snapshot.import_summary.next_action.clone()
        } else if snapshot.contract_summary.present {
            snapshot.contract_summary.next_action.clone()
        } else {
            snapshot.next_action.clone()
        };
        stack = stack.child(
            Label::new(next_action)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    stack.into_any_element()
}

pub(super) fn dx_agent_social_state(snapshot: &DxAgentBridgeSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row(
            "Supported",
            snapshot.connected_accounts_summary.supported.to_string(),
        ))
        .child(metric_row(
            "Needs auth",
            snapshot.connected_accounts_summary.needs_auth.to_string(),
        ))
        .child(metric_row(
            "QR-ready",
            snapshot
                .connected_accounts_summary
                .qr_connect_supported
                .to_string(),
        ));

    if snapshot.social_accounts.is_empty() {
        stack = stack.child(muted_card("Run social list receipt", cx));
    } else {
        for (ix, account) in snapshot.social_accounts.iter().take(3).enumerate() {
            stack = stack.child(dx_agent_social_row(
                SharedString::from(format!("dx-agent-social-{ix}")),
                account,
                cx,
            ));
        }
    }

    if snapshot.social_connect.present {
        stack = stack.child(dx_agent_social_action_row(
            SharedString::from("dx-agent-social-connect-receipt"),
            &snapshot.social_connect,
            cx,
        ));
    }

    if snapshot.social_disconnect.present {
        stack = stack.child(dx_agent_social_action_row(
            SharedString::from("dx-agent-social-disconnect-receipt"),
            &snapshot.social_disconnect,
            cx,
        ));
    }

    stack.into_any_element()
}

pub(super) fn dx_agent_automation_state(snapshot: &DxAgentBridgeSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Count", snapshot.automation_count.to_string()))
        .child(metric_row("Active", snapshot.active_task_count.to_string()))
        .child(metric_row(
            "Command",
            "dx agents automate list --json".to_string(),
        ));

    if snapshot.automations.is_empty() {
        stack = stack.child(muted_card("Run automation list receipt", cx));
    } else {
        for (ix, automation) in snapshot.automations.iter().take(3).enumerate() {
            stack = stack.child(dx_agent_automation_row(
                SharedString::from(format!("dx-agent-automation-{ix}")),
                automation,
                cx,
            ));
        }
    }

    stack.into_any_element()
}

fn dx_agent_social_row(id: SharedString, account: &DxAgentSocialAccount, cx: &App) -> AnyElement {
    let state = if account.connected {
        "Connected".to_string()
    } else if account.qr_connect_supported {
        "QR ready".to_string()
    } else if account.configured {
        "Configured".to_string()
    } else {
        "Needs setup".to_string()
    };

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(account.platform.clone(), state))
        .child(
            Label::new(format!("{} - {}", account.label, account.status))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .when(!account.next_action.is_empty(), |this| {
            this.child(
                Label::new(account.next_action.clone())
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(
            dx_agent_action_line(&account.actions),
            |this, action_line| {
                this.child(
                    Label::new(action_line)
                        .size(LabelSize::XSmall)
                        .color(Color::Muted)
                        .truncate(),
                )
            },
        )
        .into_any_element()
}

fn dx_agent_action_line(actions: &[DxAgentRowAction]) -> Option<String> {
    if actions.is_empty() {
        return None;
    }

    let ready = actions.iter().filter(|action| action.enabled).count();
    let user_actions = actions
        .iter()
        .filter(|action| action.user_action_required)
        .count();
    let public_bridges = actions
        .iter()
        .filter(|action| action.public_command.starts_with("dx agents "))
        .count();
    let receipts = actions
        .iter()
        .take(2)
        .map(|action| action.receipt_filename.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    Some(format!(
        "{ready}/{} action(s) ready, {public_bridges} public bridge(s), {user_actions} user action(s), receipts {receipts}",
        actions.len()
    ))
}

fn dx_agent_social_action_row(
    id: SharedString,
    receipt: &DxAgentSocialActionSummary,
    cx: &App,
) -> AnyElement {
    let connected = if receipt.connected.unwrap_or(false) {
        "connected"
    } else {
        "not connected"
    };
    let detail = if receipt.action == "connect" {
        let support = if receipt.connect_supported {
            "supported"
        } else {
            "unsupported"
        };
        let qr = if receipt.qr_supported {
            "QR ready"
        } else {
            "QR unavailable"
        };
        let link = if receipt.link_supported {
            "link ready"
        } else {
            "link unavailable"
        };
        format!(
            "{} connect {}, via {}, {}, {}, {}",
            receipt.label, support, receipt.connect_method, qr, link, connected
        )
    } else {
        let support = if receipt.disconnect_supported {
            "supported"
        } else {
            "not needed"
        };
        let revoke = if receipt.manual_revoke_required {
            "provider revoke"
        } else {
            "no revoke"
        };
        format!(
            "{} disconnect {}, {}, {}, config {}",
            receipt.label, support, revoke, connected, receipt.safe_config_state
        )
    };

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(
            format!("Last {}", receipt.action),
            receipt.status.clone(),
        ))
        .child(
            Label::new(detail)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(
            Label::new(receipt.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

fn dx_agent_automation_row(
    id: SharedString,
    automation: &DxAgentAutomation,
    cx: &App,
) -> AnyElement {
    let state = if automation.enabled {
        automation.status.clone()
    } else {
        "paused".to_string()
    };

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(automation.id.clone(), state))
        .child(
            Label::new(format!(
                "{} schedule from {}",
                automation.schedule_kind, automation.source
            ))
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        )
        .when(!automation.next_action.is_empty(), |this| {
            this.child(
                Label::new(automation.next_action.clone())
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(
            dx_agent_action_line(&automation.actions),
            |this, action_line| {
                this.child(
                    Label::new(action_line)
                        .size(LabelSize::XSmall)
                        .color(Color::Muted)
                        .truncate(),
                )
            },
        )
        .into_any_element()
}

pub(super) fn dx_agent_receipt_state(snapshot: &DxAgentBridgeSnapshot, cx: &App) -> AnyElement {
    let index = &snapshot.receipt_index;
    let inbox = &snapshot.receipt_inbox;
    let redacted_count = snapshot
        .receipts
        .iter()
        .filter(|receipt| receipt.metadata_redacted)
        .count();
    let unsafe_count = snapshot
        .receipts
        .iter()
        .filter(|receipt| !receipt.safe_to_render)
        .count();
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Index", index.status.clone()))
        .child(metric_row(
            "Root",
            dx_agent_receipt_root_state(index.receipt_root_present, snapshot.root_exists),
        ))
        .child(metric_row("Inbox", inbox.status.clone()))
        .child(metric_row(
            "Returned",
            format!("{} / {}", index.returned_receipt_count, index.receipt_count),
        ))
        .child(metric_row("Active", index.active_task_count.to_string()))
        .child(metric_row("Redacted", redacted_count.to_string()))
        .child(metric_row("Unsafe", unsafe_count.to_string()));

    if index.receipt_root_present == Some(false) {
        stack = stack.child(signal_row(
            "dx-agent-receipt-root-missing".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents receipt root was missing before the latest receipt refresh.".to_string(),
        ));
    }
    if inbox.receipt_dir_present == Some(false) {
        stack = stack.child(signal_row(
            "dx-agent-receipt-inbox-root-missing".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents receipt inbox reports a missing receipt directory.".to_string(),
        ));
    } else if inbox.malformed_count > 0 {
        stack = stack.child(signal_row(
            "dx-agent-receipt-inbox-malformed".into(),
            IconName::Warning,
            Color::Warning,
            format!(
                "DX Agents receipt inbox found {} malformed receipt(s).",
                inbox.malformed_count
            ),
        ));
    }

    if !index.present {
        stack = stack.child(muted_card("Run dx agents receipts list --json", cx));
    } else if let Some(error) = index.last_error.as_ref() {
        stack = stack.child(signal_row(
            "dx-agent-receipt-index-error".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents receipt index error: {error}"),
        ));
    } else if unsafe_count > 0 {
        stack = stack.child(signal_row(
            "dx-agent-receipt-unsafe-row".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents receipt index contains rows that are not safe to render.".to_string(),
        ));
    } else if let Some(path) = index.latest_receipt_path.as_ref() {
        stack = stack.child(metric_row("Latest", path.clone()));
    }

    if snapshot.receipts.is_empty() {
        stack = stack.child(muted_card("No renderable receipt rows", cx));
    } else {
        for (ix, receipt) in snapshot.receipts.iter().take(3).enumerate() {
            stack = stack.child(dx_agent_receipt_row(
                SharedString::from(format!("dx-agent-receipt-{ix}")),
                receipt,
                cx,
            ));
        }
    }

    stack
        .when(inbox.present, |this| {
            this.child(metric_row(
                "Inbox review",
                format!(
                    "{} latest, {} missing, {} stale, {} expired",
                    inbox.latest_count,
                    inbox.missing_latest_count,
                    inbox.stale_count,
                    inbox.expired_count
                ),
            ))
        })
        .child(
            Label::new(index.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

fn dx_agent_receipt_root_state(receipt_root_present: Option<bool>, root_exists: bool) -> String {
    match receipt_root_present {
        Some(true) => "present".to_string(),
        Some(false) => "missing before refresh".to_string(),
        None if root_exists => "present".to_string(),
        None => "missing".to_string(),
    }
}

fn dx_agent_receipt_row(id: SharedString, receipt: &DxAgentReceipt, cx: &App) -> AnyElement {
    let state = if !receipt.safe_to_render {
        "Unsafe".to_string()
    } else if receipt.active_task {
        "Active".to_string()
    } else if receipt.metadata_redacted {
        format!("{} / redacted", receipt.status)
    } else {
        receipt.status.clone()
    };
    let detail = if receipt.command.is_empty() {
        format!("{} - {} bytes", receipt.kind, receipt.size_bytes)
    } else {
        format!(
            "{} - {} - {} bytes",
            receipt.kind, receipt.command, receipt.size_bytes
        )
    };
    let provider_model = match (
        receipt.provider_status.as_deref(),
        receipt.model_status.as_deref(),
    ) {
        (Some(provider), Some(model)) => Some(format!("Provider {provider}, model {model}")),
        (Some(provider), None) => Some(format!("Provider {provider}")),
        (None, Some(model)) => Some(format!("Model {model}")),
        (None, None) => None,
    };
    let actions = match (receipt.retry_supported, receipt.cancel_supported) {
        (Some(retry), Some(cancel)) => Some(format!(
            "Retry {}, cancel {}",
            yes_no(retry),
            yes_no(cancel)
        )),
        (Some(retry), None) => Some(format!("Retry {}", yes_no(retry))),
        (None, Some(cancel)) => Some(format!("Cancel {}", yes_no(cancel))),
        (None, None) => None,
    };
    let social_status = match (receipt.social_connected, receipt.social_needs_auth) {
        (Some(connected), Some(needs_auth)) => Some(format!(
            "Social connected {connected}, needs auth {needs_auth}"
        )),
        (Some(connected), None) => Some(format!("Social connected {connected}")),
        (None, Some(needs_auth)) => Some(format!("Social needs auth {needs_auth}")),
        (None, None) => None,
    };
    let automation_status = match (receipt.automation_enabled, receipt.automation_warning) {
        (Some(enabled), Some(warning)) => {
            Some(format!("Automations enabled {enabled}, warning {warning}"))
        }
        (Some(enabled), None) => Some(format!("Automations enabled {enabled}")),
        (None, Some(warning)) => Some(format!("Automation warnings {warning}")),
        (None, None) => None,
    };

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(receipt.id.clone(), state))
        .child(
            Label::new(detail)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .when(!receipt.task_state.is_empty(), |this| {
            this.child(
                Label::new(format!("Task: {}", receipt.task_state))
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(receipt.duration_state.as_ref(), |this, duration| {
            this.child(
                Label::new(format!("Duration: {duration}"))
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(provider_model, |this, provider_model| {
            this.child(
                Label::new(provider_model)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(actions, |this, actions| {
            this.child(
                Label::new(actions)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(social_status, |this, social_status| {
            this.child(
                Label::new(social_status)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(automation_status, |this, automation_status| {
            this.child(
                Label::new(automation_status)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when(!receipt.schema_version.is_empty(), |this| {
            this.child(
                Label::new(receipt.schema_version.clone())
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when_some(receipt.last_error.as_ref(), |this, error| {
            this.child(
                Label::new(format!("Error: {error}"))
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .when(
            receipt.last_error.is_none() && !receipt.next_action.is_empty(),
            |this| {
                this.child(
                    Label::new(receipt.next_action.clone())
                        .size(LabelSize::XSmall)
                        .color(Color::Muted)
                        .truncate(),
                )
            },
        )
        .into_any_element()
}
