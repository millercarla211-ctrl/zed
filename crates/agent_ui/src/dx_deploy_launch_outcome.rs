use serde_json::Value;

#[derive(Clone, Default)]
pub(crate) struct DxDeployLaunchOutcome {
    pub pass_count: Option<usize>,
    pub fail_count: Option<usize>,
    pub warn_count: Option<usize>,
    pub skipped_count: Option<usize>,
    pub duration_ms: Option<usize>,
    pub skipped_expensive_checks: Vec<String>,
}

pub(crate) fn launch_outcome(receipt: &Value) -> DxDeployLaunchOutcome {
    DxDeployLaunchOutcome {
        pass_count: usize_field(receipt, "pass_count"),
        fail_count: usize_field(receipt, "fail_count"),
        warn_count: usize_field(receipt, "warn_count"),
        skipped_count: usize_field(receipt, "skipped_count"),
        duration_ms: usize_field(receipt, "duration_ms"),
        skipped_expensive_checks: string_array(receipt, "skipped_expensive_checks", 4),
    }
}

pub(crate) fn launch_outcome_summary(outcome: &DxDeployLaunchOutcome) -> Option<String> {
    let parts = [
        outcome.pass_count.map(|count| format!("{count} pass")),
        outcome.fail_count.map(|count| format!("{count} fail")),
        outcome.warn_count.map(|count| format!("{count} warn")),
        outcome
            .skipped_count
            .map(|count| format!("{count} skipped")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" / "))
    }
}

pub(crate) fn launch_duration_label(outcome: &DxDeployLaunchOutcome) -> Option<String> {
    outcome
        .duration_ms
        .map(|duration_ms| format!("{duration_ms} ms"))
}

pub(crate) fn launch_outcome_prompt(outcome: &DxDeployLaunchOutcome) -> String {
    let parts = [
        outcome.pass_count.map(|count| format!("pass={count}")),
        outcome.fail_count.map(|count| format!("fail={count}")),
        outcome.warn_count.map(|count| format!("warn={count}")),
        outcome
            .skipped_count
            .map(|count| format!("skipped={count}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if parts.is_empty() {
        "unknown".to_string()
    } else {
        parts.join(",")
    }
}

pub(crate) fn skipped_checks_prompt(outcome: &DxDeployLaunchOutcome) -> String {
    if outcome.skipped_expensive_checks.is_empty() {
        "none".to_string()
    } else {
        outcome.skipped_expensive_checks.join("; ")
    }
}

fn usize_field(value: &Value, key: &str) -> Option<usize> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
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
