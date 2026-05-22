use serde_json::Value;

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
pub(crate) struct DxDeployProviderGateReceiptSummary {
    pub label: String,
    pub schema_version: Option<String>,
    pub provider: Option<String>,
    pub command: Option<String>,
    pub status: Option<String>,
    pub deploy_ready: Option<bool>,
    pub dry_run: Option<bool>,
    pub deploy_ran: Option<bool>,
    pub zed_panel_kind: Option<String>,
    pub zed_title: Option<String>,
    pub zed_summary: Option<String>,
    pub blocker_count: usize,
    pub warning_count: usize,
    pub next_action: Option<String>,
    pub rows: Vec<DxDeployProviderGateRow>,
    pub quick_fix_count: usize,
}

#[derive(Clone)]
pub(crate) struct DxDeployProviderGateRow {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: Option<String>,
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

pub(crate) fn parse_deploy_provider_gate_receipt(
    label: String,
    value: &Value,
) -> Option<DxDeployProviderGateReceiptSummary> {
    let zed = value.get("zed");

    Some(DxDeployProviderGateReceiptSummary {
        label,
        schema_version: string_field(value, "schema_version"),
        provider: string_field(value, "provider"),
        command: string_field(value, "command"),
        status: string_field(value, "status"),
        deploy_ready: value.get("deploy_ready").and_then(Value::as_bool),
        dry_run: value.get("dry_run").and_then(Value::as_bool),
        deploy_ran: value.get("deploy_ran").and_then(Value::as_bool),
        zed_panel_kind: zed.and_then(|zed| string_field(zed, "panel_kind")),
        zed_title: zed.and_then(|zed| string_field(zed, "title")),
        zed_summary: zed.and_then(|zed| string_field(zed, "summary")),
        blocker_count: array_len(value, "blockers"),
        warning_count: array_len(value, "warnings"),
        next_action: first_string_array_item(value, "next_actions").or_else(|| {
            value
                .get("launch_approved")
                .and_then(|launch| string_field(launch, "next_action"))
        }),
        rows: provider_gate_rows(value),
        quick_fix_count: zed.map_or(0, |zed| array_len(zed, "quick_fixes")),
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

fn provider_gate_rows(value: &Value) -> Vec<DxDeployProviderGateRow> {
    value
        .get("zed")
        .and_then(|zed| zed.get("rows"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(6)
        .filter_map(provider_gate_row_from_value)
        .collect()
}

fn provider_gate_row_from_value(value: &Value) -> Option<DxDeployProviderGateRow> {
    Some(DxDeployProviderGateRow {
        id: string_field(value, "id")?,
        label: string_field(value, "label").unwrap_or_else(|| "Deploy gate".to_string()),
        status: string_field(value, "status").unwrap_or_else(|| "unknown".to_string()),
        detail: string_field(value, "detail"),
    })
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

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn usize_field(value: &Value, key: &str) -> Option<usize> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn array_len(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}

fn string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(8)
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn first_string_array_item(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(ToOwned::to_owned)
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
