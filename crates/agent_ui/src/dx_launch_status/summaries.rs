use super::{
    DxLaunchAgentsSummary, DxLaunchDiscoverySummary, DxLaunchTokensSummary,
    fields::{pointer_bool, pointer_string, pointer_u64, pointer_usize},
};
use serde_json::Value;

impl DxLaunchAgentsSummary {
    pub(super) fn empty() -> Self {
        Self {
            status: "unknown".to_string(),
            configured_accounts: 0,
            connected_accounts: 0,
            accounts_needing_connection: 0,
            qr_connect_supported: 0,
            automation_count: 0,
            active_task_count: 0,
            next_action: "run_launch_status".to_string(),
        }
    }
}

impl DxLaunchTokensSummary {
    pub(super) fn empty() -> Self {
        Self {
            status: "unknown".to_string(),
            budget_state: "unknown".to_string(),
            estimated_tokens: 0,
            soft_budget_tokens: 0,
            hard_budget_tokens: 0,
            next_action: "run_launch_status".to_string(),
        }
    }
}

impl DxLaunchDiscoverySummary {
    pub(super) fn empty() -> Self {
        Self {
            status: "unknown".to_string(),
            templates_command: "dx www templates --json".to_string(),
            packages_command: "dx forge packages --json".to_string(),
            www_manifest_present: false,
            configured_binary_present: false,
            next_action: "run_launch_status".to_string(),
        }
    }
}

pub(super) fn agents_summary(value: &Value) -> DxLaunchAgentsSummary {
    DxLaunchAgentsSummary {
        status: pointer_string(value, "/agents/status", "unknown"),
        configured_accounts: pointer_usize(value, "/agents/connected_accounts_configured"),
        connected_accounts: pointer_usize(value, "/agents/connected_accounts_connected"),
        accounts_needing_connection: pointer_usize(
            value,
            "/agents/connected_accounts_needs_connection",
        ),
        qr_connect_supported: pointer_usize(value, "/agents/qr_connect_supported"),
        automation_count: pointer_usize(value, "/agents/automation_count"),
        active_task_count: pointer_usize(value, "/agents/active_task_count"),
        next_action: pointer_string(value, "/agents/next_action", "review_agent_status"),
    }
}

pub(super) fn tokens_summary(value: &Value) -> DxLaunchTokensSummary {
    DxLaunchTokensSummary {
        status: pointer_string(value, "/tokens/status", "unknown"),
        budget_state: pointer_string(value, "/tokens/budget_state", "unknown"),
        estimated_tokens: pointer_u64(value, "/tokens/estimated_tokens"),
        soft_budget_tokens: pointer_u64(value, "/tokens/soft_budget_tokens"),
        hard_budget_tokens: pointer_u64(value, "/tokens/hard_budget_tokens"),
        next_action: pointer_string(value, "/tokens/next_action", "review_token_budget"),
    }
}

pub(super) fn discovery_summary(value: &Value) -> DxLaunchDiscoverySummary {
    DxLaunchDiscoverySummary {
        status: pointer_string(value, "/discovery/status", "unknown"),
        templates_command: pointer_string(
            value,
            "/discovery/templates_command",
            "dx www templates --json",
        ),
        packages_command: pointer_string(
            value,
            "/discovery/packages_command",
            "dx forge packages --json",
        ),
        www_manifest_present: pointer_bool(value, "/discovery/www_manifest_present"),
        configured_binary_present: pointer_bool(value, "/discovery/configured_binary_present"),
        next_action: pointer_string(value, "/discovery/next_action", "review_discovery_bridge"),
    }
}
