use crate::dx_deploy_receipt_fields::{array_len, first_string_array_item, string_field};
use serde_json::Value;

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
    pub quick_fixes: Vec<DxDeployProviderGateQuickFix>,
}
#[derive(Clone)]
pub(crate) struct DxDeployProviderGateRow {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: Option<String>,
}
#[derive(Clone)]
pub(crate) struct DxDeployProviderGateQuickFix {
    pub id: String,
    pub label: String,
    pub command: String,
    pub risk_level: String,
    pub requires_user_approval: bool,
    pub writes_receipts: bool,
}

pub(crate) fn parse_deploy_provider_gate_receipt(
    label: String,
    value: &Value,
) -> Option<DxDeployProviderGateReceiptSummary> {
    let zed = value.get("zed");
    let quick_fixes = provider_gate_quick_fixes(zed);

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
        quick_fix_count: quick_fixes.len(),
        quick_fixes,
    })
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

fn provider_gate_quick_fixes(zed: Option<&Value>) -> Vec<DxDeployProviderGateQuickFix> {
    zed.and_then(|zed| zed.get("quick_fixes"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(6)
        .filter_map(provider_gate_quick_fix_from_value)
        .collect()
}

fn provider_gate_quick_fix_from_value(value: &Value) -> Option<DxDeployProviderGateQuickFix> {
    Some(DxDeployProviderGateQuickFix {
        id: string_field(value, "id")?,
        label: string_field(value, "label").unwrap_or_else(|| "Deploy quick fix".to_string()),
        command: string_field(value, "command")?,
        risk_level: string_field(value, "risk_level").unwrap_or_else(|| "unknown".to_string()),
        requires_user_approval: value
            .get("requires_user_approval")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        writes_receipts: value
            .get("writes_receipts")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}
