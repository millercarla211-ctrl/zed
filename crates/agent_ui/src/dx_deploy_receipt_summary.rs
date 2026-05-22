use serde_json::Value;

use crate::dx_deploy_receipt_fields::{
    array_len, first_string_array_item, string_array, string_field, usize_field,
};

#[derive(Clone)]
pub(crate) struct DxDeployCommandReceiptSummary {
    pub label: String,
    pub schema_version: Option<String>,
    pub command: Option<String>,
    pub status: Option<String>,
    pub latest_plan_status: Option<String>,
    pub dry_run: Option<bool>,
    pub deploy_ran: Option<bool>,
    pub provider_count: usize,
    pub ready_plan_count: Option<usize>,
    pub blocked_plan_count: Option<usize>,
    pub fixture_label: Option<String>,
    pub blocker_count: usize,
    pub warning_count: usize,
    pub next_action: Option<String>,
}

#[derive(Clone)]
pub(crate) struct DxDeployCapabilityRow {
    pub id: String,
    pub name: String,
    pub target_kinds: Vec<String>,
    pub current_support: String,
    pub write_support: String,
    pub dry_run: bool,
    pub needs_credentials_count: usize,
}

pub(crate) fn parse_deploy_command_receipt(
    label: String,
    value: &Value,
) -> Option<DxDeployCommandReceiptSummary> {
    let providers = deploy_provider_rows_from_value(value);
    let plans = value.get("plans").and_then(Value::as_array);

    Some(DxDeployCommandReceiptSummary {
        label,
        schema_version: string_field(value, "schema_version"),
        command: string_field(value, "command"),
        status: string_field(value, "status"),
        latest_plan_status: string_field(value, "latest_plan_status"),
        dry_run: value.get("dry_run").and_then(Value::as_bool),
        deploy_ran: value.get("deploy_ran").and_then(Value::as_bool),
        provider_count: usize_field(value, "provider_count").unwrap_or_else(|| providers.len()),
        ready_plan_count: usize_field(value, "ready_plan_count")
            .or_else(|| count_plan_status(plans, &["ready", "dry-run"])),
        blocked_plan_count: usize_field(value, "blocked_plan_count")
            .or_else(|| count_plan_status(plans, &["blocked"])),
        fixture_label: fixture_label(value),
        blocker_count: array_len(value, "blockers"),
        warning_count: array_len(value, "warnings"),
        next_action: first_string_array_item(value, "next_actions"),
    })
}

pub(crate) fn deploy_provider_rows_from_value(value: &Value) -> Vec<DxDeployCapabilityRow> {
    let providers = match value {
        Value::Array(values) => Some(values),
        Value::Object(map) => map.get("providers").and_then(Value::as_array),
        _ => None,
    };

    providers
        .into_iter()
        .flatten()
        .take(12)
        .filter_map(provider_row_from_value)
        .collect()
}

fn provider_row_from_value(value: &Value) -> Option<DxDeployCapabilityRow> {
    Some(DxDeployCapabilityRow {
        id: string_field(value, "id")?,
        name: string_field(value, "name").unwrap_or_else(|| "Deploy provider".to_string()),
        target_kinds: string_array(value, "target_kinds"),
        current_support: string_field(value, "current_support")
            .unwrap_or_else(|| "unknown".to_string()),
        write_support: string_field(value, "write_support")
            .unwrap_or_else(|| "unknown".to_string()),
        dry_run: value
            .get("dry_run")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        needs_credentials_count: value
            .get("needs_credentials")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or_default(),
    })
}

fn fixture_label(value: &Value) -> Option<String> {
    let fixture = value.get("selected_fixture")?;
    let name = string_field(fixture, "name");
    let source_dir = string_field(fixture, "source_dir");

    match (name, source_dir) {
        (Some(name), Some(source_dir)) => Some(format!("{name}: {source_dir}")),
        (Some(name), None) => Some(name),
        (None, Some(source_dir)) => Some(source_dir),
        (None, None) => None,
    }
}

fn count_plan_status(plans: Option<&Vec<Value>>, statuses: &[&str]) -> Option<usize> {
    let plans = plans?;
    Some(
        plans
            .iter()
            .filter_map(|plan| plan.get("status").and_then(Value::as_str))
            .filter(|status| statuses.iter().any(|expected| *expected == *status))
            .count(),
    )
}
