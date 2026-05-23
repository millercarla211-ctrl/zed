use super::packet_fields::{bool_field, bool_label, string_field, usize_field};
use serde_json::Value;

pub(super) fn status_agent_summary(packet: Option<&Value>) -> String {
    let Some(agent) = packet.and_then(|value| value.get("agents")) else {
        return "missing".to_string();
    };

    format!(
        "{} / {} connected",
        usize_field(agent, "connected_accounts_connected").unwrap_or_default(),
        usize_field(agent, "connected_accounts_configured").unwrap_or_default()
    )
}

pub(super) fn status_token_summary(packet: Option<&Value>) -> String {
    let Some(tokens) = packet.and_then(|value| value.get("tokens")) else {
        return "missing".to_string();
    };

    format!(
        "{} / {} tokens",
        string_field(tokens, "budget_state").unwrap_or("unknown"),
        usize_field(tokens, "estimated_tokens").unwrap_or_default()
    )
}

pub(super) fn status_discovery_summary(packet: Option<&Value>) -> String {
    let Some(discovery) = packet.and_then(|value| value.get("discovery")) else {
        return "missing".to_string();
    };

    format!(
        "{} / manifest {} / binary {}",
        string_field(discovery, "status").unwrap_or("unknown"),
        bool_label(bool_field(discovery, "www_manifest_present")),
        bool_label(bool_field(discovery, "configured_binary_present"))
    )
}
