use serde_json::Value;

#[derive(Clone)]
pub(crate) struct DxDeployLaunchAction {
    pub id: Option<String>,
    pub label: String,
    pub command: Option<String>,
    pub risk_level: Option<String>,
    pub requires_user_approval: Option<bool>,
    pub writes_receipts: Option<bool>,
    pub next_action: Option<String>,
}

pub(crate) fn launch_actions(value: Option<&Value>) -> Vec<DxDeployLaunchAction> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(5)
        .filter_map(launch_action)
        .collect()
}

fn launch_action(row: &Value) -> Option<DxDeployLaunchAction> {
    Some(DxDeployLaunchAction {
        id: string_field(row, "id"),
        label: string_field(row, "label").or_else(|| string_field(row, "next_action"))?,
        command: string_field(row, "command"),
        risk_level: string_field(row, "risk_level"),
        requires_user_approval: bool_field(row, "requires_user_approval"),
        writes_receipts: bool_field(row, "writes_receipts"),
        next_action: string_field(row, "next_action"),
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

fn bool_field(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}
