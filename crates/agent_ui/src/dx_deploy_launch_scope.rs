use serde_json::Value;

#[derive(Clone, Default)]
pub(crate) struct DxDeployLaunchScope {
    pub weight_profile: Option<String>,
    pub active_profile: Option<String>,
    pub scoring_status: Option<String>,
    pub scoring_config_present: Option<bool>,
    pub checked_paths: Vec<String>,
    pub next_action: Option<String>,
}

pub(crate) fn launch_scope(receipt: &Value) -> DxDeployLaunchScope {
    let scoring_config = receipt.get("scoring_config");

    DxDeployLaunchScope {
        weight_profile: string_field(receipt, "weight_profile"),
        active_profile: scoring_config
            .and_then(|scoring_config| string_field(scoring_config, "active_profile")),
        scoring_status: scoring_config
            .and_then(|scoring_config| string_field(scoring_config, "status")),
        scoring_config_present: scoring_config
            .and_then(|scoring_config| bool_field(scoring_config, "config_present")),
        checked_paths: string_array(receipt, "checked_paths", 4),
        next_action: scoring_config
            .and_then(|scoring_config| string_field(scoring_config, "next_action")),
    }
}

pub(crate) fn launch_scope_summary(scope: &DxDeployLaunchScope) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(profile) = scope
        .active_profile
        .as_ref()
        .or(scope.weight_profile.as_ref())
    {
        parts.push(format!("profile {profile}"));
    }
    if let Some(status) = scope.scoring_status.as_ref() {
        parts.push(status.clone());
    }
    if let Some(config_present) = scope.scoring_config_present {
        parts.push(if config_present {
            "configured".to_string()
        } else {
            "default config".to_string()
        });
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" / "))
    }
}

pub(crate) fn checked_paths_prompt(scope: &DxDeployLaunchScope) -> String {
    if scope.checked_paths.is_empty() {
        "none".to_string()
    } else {
        scope.checked_paths.join(" / ")
    }
}

pub(crate) fn launch_scope_prompt(scope: &DxDeployLaunchScope) -> String {
    let profile = scope
        .active_profile
        .as_deref()
        .or(scope.weight_profile.as_deref())
        .unwrap_or("unknown");
    let status = scope.scoring_status.as_deref().unwrap_or("unknown");
    let config_present = scope
        .scoring_config_present
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let next_action = scope.next_action.as_deref().unwrap_or("none");

    format!(
        "profile={profile},status={status},config_present={config_present},next_action={next_action}"
    )
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

fn string_array(value: &Value, key: &str, limit: usize) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .take(limit)
        .map(ToOwned::to_owned)
        .collect()
}
