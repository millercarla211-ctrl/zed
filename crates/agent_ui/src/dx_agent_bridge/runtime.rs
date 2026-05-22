use std::path::PathBuf;

use serde_json::Value;

use super::{
    DxAgentAutomation, DxAgentCatalogSummary, DxAgentModel, DxAgentProvider, DxAgentRowAction,
    DxAgentSocialAccount, DxAgentSocialActionSummary, DxConnectedAccountsSummary, array_field,
    bool_field, is_dx_agents_command, is_public_dx_agents_command, is_safe_platform_arg,
    is_secret_like_arg, public_command_for_runtime, string_array_field, string_field, usize_field,
};

pub(super) fn connected_accounts_summary(value: &Value) -> DxConnectedAccountsSummary {
    let needs_connection = usize_field(value, &["needs_connection"]).unwrap_or_default();
    DxConnectedAccountsSummary {
        supported: usize_field(value, &["supported"]).unwrap_or_default(),
        configured: usize_field(value, &["configured"]).unwrap_or_default(),
        connected: usize_field(value, &["connected"]).unwrap_or_default(),
        needs_connection,
        needs_auth: usize_field(value, &["needs_auth"]).unwrap_or(needs_connection),
        qr_connect_supported: usize_field(value, &["qr_connect_supported"]).unwrap_or_default(),
    }
}

pub(super) fn social_accounts(value: &Value) -> Vec<DxAgentSocialAccount> {
    array_field(value, &["accounts"])
        .map(|accounts| {
            accounts
                .iter()
                .take(12)
                .map(|account| DxAgentSocialAccount {
                    platform: string_field(account, &["platform"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    label: string_field(account, &["label"])
                        .unwrap_or_else(|| "Account".to_string()),
                    status: string_field(account, &["status"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    configured: bool_field(account, &["configured"]).unwrap_or(false),
                    connected: bool_field(account, &["connected"]).unwrap_or(false),
                    qr_connect_supported: bool_field(account, &["qr_connect_supported"])
                        .unwrap_or(false),
                    actions: social_row_actions(account),
                    next_action: string_field(account, &["next_action"]).unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Clone, Copy)]
pub(super) enum DxAgentSocialActionKind {
    Connect,
    Disconnect,
}

pub(super) fn social_action_summary(
    value: Option<&Value>,
    root_exists: bool,
    kind: DxAgentSocialActionKind,
) -> DxAgentSocialActionSummary {
    let action = match kind {
        DxAgentSocialActionKind::Connect => "connect",
        DxAgentSocialActionKind::Disconnect => "disconnect",
    };
    let command = match kind {
        DxAgentSocialActionKind::Connect => "dx agents social connect --json",
        DxAgentSocialActionKind::Disconnect => "dx agents social disconnect --json",
    };
    let waiting_status = match kind {
        DxAgentSocialActionKind::Connect => "waiting_for_social_connect_receipt",
        DxAgentSocialActionKind::Disconnect => "waiting_for_social_disconnect_receipt",
    };
    let account = value.and_then(|value| value.get("account"));
    let flow = value.and_then(|value| value.get("flow"));

    DxAgentSocialActionSummary {
        action,
        present: value.is_some(),
        status: value
            .and_then(|value| string_field(value, &["status"]))
            .unwrap_or_else(|| {
                if root_exists {
                    waiting_status.to_string()
                } else {
                    "missing_receipt_root".to_string()
                }
            }),
        platform: account
            .and_then(|account| string_field(account, &["platform"]))
            .unwrap_or_else(|| "unknown".to_string()),
        label: account
            .and_then(|account| string_field(account, &["label"]))
            .unwrap_or_else(|| "Social account".to_string()),
        connected: account.and_then(|account| bool_field(account, &["connected"])),
        connect_supported: flow
            .and_then(|flow| bool_field(flow, &["connect_supported"]))
            .unwrap_or(false),
        disconnect_supported: flow
            .and_then(|flow| bool_field(flow, &["disconnect_supported"]))
            .unwrap_or(false),
        qr_supported: flow
            .and_then(|flow| bool_field(flow, &["qr_supported"]))
            .unwrap_or(false),
        link_supported: flow
            .and_then(|flow| bool_field(flow, &["link_supported"]))
            .unwrap_or(false),
        connect_method: flow
            .and_then(|flow| string_field(flow, &["connect_method"]))
            .unwrap_or_else(|| "none".to_string()),
        manual_revoke_required: flow
            .and_then(|flow| bool_field(flow, &["manual_revoke_required"]))
            .unwrap_or(false),
        explicit_user_action_required: flow
            .and_then(|flow| bool_field(flow, &["explicit_user_action_required"]))
            .unwrap_or(false),
        safe_config_state: flow
            .and_then(|flow| string_field(flow, &["safe_config_state"]))
            .unwrap_or_else(|| "unknown".to_string()),
        next_action: value
            .and_then(|value| string_field(value, &["next_action"]))
            .unwrap_or_else(|| command.to_string()),
    }
}

pub(super) fn automations(value: &Value) -> Vec<DxAgentAutomation> {
    array_field(value, &["automations"])
        .map(|automations| {
            automations
                .iter()
                .take(12)
                .map(|automation| DxAgentAutomation {
                    id: string_field(automation, &["id"])
                        .unwrap_or_else(|| "automation".to_string()),
                    status: string_field(automation, &["status"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    enabled: bool_field(automation, &["enabled"]).unwrap_or(false),
                    schedule_kind: string_field(automation, &["schedule_kind"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    source: string_field(automation, &["source"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    actions: automation_row_actions(automation),
                    next_action: string_field(automation, &["next_action"]).unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn social_row_actions(value: &Value) -> Vec<DxAgentRowAction> {
    row_actions(value, |id, command, receipt_filename, refresh_command| {
        is_dx_agents_command(refresh_command, "social list --json")
            && match id {
                "connect" => {
                    receipt_filename == "social-connect-latest.json"
                        && is_social_action_command(command, "connect")
                }
                "disconnect" => {
                    receipt_filename == "social-disconnect-latest.json"
                        && is_social_action_command(command, "disconnect")
                }
                "refresh" => {
                    receipt_filename == "social-list-latest.json"
                        && is_dx_agents_command(command, "social list --json")
                }
                _ => false,
            }
    })
}

fn automation_row_actions(value: &Value) -> Vec<DxAgentRowAction> {
    row_actions(value, |id, command, receipt_filename, refresh_command| {
        is_dx_agents_command(refresh_command, "automate list --json")
            && match id {
                "run" => {
                    receipt_filename == "run-latest.json"
                        && is_dx_agents_command(command, "run --json")
                }
                "refresh" => {
                    receipt_filename == "automate-list-latest.json"
                        && is_dx_agents_command(command, "automate list --json")
                }
                _ => false,
            }
    })
}

fn row_actions<F>(value: &Value, is_allowed: F) -> Vec<DxAgentRowAction>
where
    F: Fn(&str, &str, &str, &str) -> bool,
{
    array_field(value, &["actions"])
        .map(|actions| {
            actions
                .iter()
                .take(8)
                .filter_map(|action| row_action(action, &is_allowed))
                .collect()
        })
        .unwrap_or_default()
}

fn row_action<F>(value: &Value, is_allowed: &F) -> Option<DxAgentRowAction>
where
    F: Fn(&str, &str, &str, &str) -> bool,
{
    let id = string_field(value, &["id"])?;
    let command = string_field(value, &["command"])?;
    let public_command = string_field(value, &["public_command"])
        .unwrap_or_else(|| public_command_for_runtime(&command));
    let receipt_filename = string_field(value, &["receipt_filename"])?;
    let refresh_command = string_field(value, &["refresh_command"])?;
    let public_refresh_command = string_field(value, &["public_refresh_command"])
        .unwrap_or_else(|| public_command_for_runtime(&refresh_command));
    let secrets_exposed = bool_field(value, &["secrets_exposed"]).unwrap_or(true);
    let writes_receipt = bool_field(value, &["writes_receipt"]).unwrap_or(false);

    if !writes_receipt
        || secrets_exposed
        || is_secret_like_arg(&command)
        || is_secret_like_arg(&public_command)
        || is_secret_like_arg(&receipt_filename)
        || is_secret_like_arg(&refresh_command)
        || is_secret_like_arg(&public_refresh_command)
        || !is_public_dx_agents_command(&public_command)
        || !is_public_dx_agents_command(&public_refresh_command)
        || !is_allowed(&id, &command, &receipt_filename, &refresh_command)
        || !is_allowed(
            &id,
            &public_command,
            &receipt_filename,
            &public_refresh_command,
        )
    {
        return None;
    }

    Some(DxAgentRowAction {
        label: string_field(value, &["label"]).unwrap_or_else(|| id.clone()),
        id,
        command,
        public_command,
        enabled: bool_field(value, &["enabled"]).unwrap_or(false),
        user_action_required: bool_field(value, &["user_action_required"]).unwrap_or(false),
        writes_receipt,
        receipt_filename,
        refresh_command,
        public_refresh_command,
        secrets_exposed,
    })
}

fn is_social_action_command(command: &str, action: &str) -> bool {
    let runtime_prefix = format!("dx-agents agents social {action}");
    let public_prefix = format!("dx agents social {action}");
    [runtime_prefix.as_str(), public_prefix.as_str()]
        .into_iter()
        .any(|prefix| social_action_command_matches_prefix(command, prefix))
}

fn social_action_command_matches_prefix(command: &str, prefix: &str) -> bool {
    if command == format!("{prefix} --json") {
        return true;
    }

    let platform_prefix = format!("{prefix} --platform ");
    command
        .strip_prefix(&platform_prefix)
        .and_then(|value| value.strip_suffix(" --json"))
        .is_some_and(|platform| is_safe_platform_arg(platform))
}

pub(super) fn providers(value: &Value) -> Vec<DxAgentProvider> {
    array_field(value, &["providers"])
        .map(|providers| {
            providers
                .iter()
                .take(24)
                .map(|provider| DxAgentProvider {
                    id: string_field(provider, &["id"]).unwrap_or_else(|| "provider".to_string()),
                    display_name: string_field(provider, &["display_name"])
                        .unwrap_or_else(|| "Provider".to_string()),
                    status: string_field(provider, &["status"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    configured: bool_field(provider, &["configured"]).unwrap_or(false),
                    active: bool_field(provider, &["active"]).unwrap_or(false),
                    local: bool_field(provider, &["local"]).unwrap_or(false),
                    compatibility: string_array_field(provider, &["compatibility"]),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn models(value: &Value) -> Vec<DxAgentModel> {
    array_field(value, &["models"])
        .map(|models| {
            models
                .iter()
                .take(24)
                .map(|model| DxAgentModel {
                    id: string_field(model, &["id"]).unwrap_or_else(|| "model".to_string()),
                    provider_id: string_field(model, &["provider_id"])
                        .unwrap_or_else(|| "provider".to_string()),
                    model_id: string_field(model, &["model_id"])
                        .unwrap_or_else(|| "model".to_string()),
                    status: string_field(model, &["status"])
                        .unwrap_or_else(|| "unknown".to_string()),
                    active: bool_field(model, &["active"]).unwrap_or(false),
                    compatibility: string_array_field(model, &["compatibility"]),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn catalog_summary(
    provider_value: Option<&Value>,
    model_value: Option<&Value>,
    default_path: PathBuf,
) -> DxAgentCatalogSummary {
    let catalog = provider_value
        .and_then(|value| value.get("catalog"))
        .or_else(|| model_value.and_then(|value| value.get("catalog")));
    let path = catalog
        .and_then(|catalog| string_field(catalog, &["binary_cache_path"]))
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or(default_path);
    DxAgentCatalogSummary {
        present: catalog
            .and_then(|catalog| bool_field(catalog, &["binary_cache_present"]))
            .unwrap_or_else(|| path.is_file()),
        stale: catalog
            .and_then(|catalog| bool_field(catalog, &["binary_cache_stale"]))
            .unwrap_or(true),
        provider_count: catalog
            .and_then(|catalog| usize_field(catalog, &["provider_count"]))
            .unwrap_or_default(),
        model_count: catalog
            .and_then(|catalog| usize_field(catalog, &["model_count"]))
            .unwrap_or_default(),
        source_hash: catalog.and_then(|catalog| string_field(catalog, &["source_hash"])),
        safe_regeneration_command: catalog
            .and_then(|catalog| string_field(catalog, &["safe_regeneration_command"]))
            .unwrap_or_else(|| "dx agents providers catalog regenerate --json".to_string()),
        path,
    }
}
