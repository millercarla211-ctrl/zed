use serde_json::Value;

use crate::{
    dx_deploy_local_files::read_json_limited, dx_deploy_receipt_buckets::DxDeployReceiptSummary,
    dx_deploy_receipt_files::DeployReceiptCandidate,
};

pub(crate) fn deploy_receipt_summary(
    candidate: &DeployReceiptCandidate,
    bucket_label: &'static str,
) -> Option<DxDeployReceiptSummary> {
    let value = read_json_limited(&candidate.path)?;
    let status = first_string_for_keys(
        &value,
        &[
            "status",
            "state",
            "result",
            "conclusion",
            "deployment_status",
        ],
    );
    let headline = first_string_for_keys(
        &value,
        &["headline", "summary", "message", "title", "next_action"],
    )
    .or_else(|| status.clone())
    .unwrap_or_else(|| format!("{bucket_label} receipt"));
    let url = first_url(&value);
    let target = first_string_for_keys(
        &value,
        &["target", "platform", "environment", "deployment", "project"],
    );
    let blocker_count = count_named_arrays(&value, &["blockers", "errors"]);

    Some(DxDeployReceiptSummary {
        label: candidate.label.clone(),
        headline,
        status,
        url,
        target,
        blocker_count,
    })
}

fn first_string_for_keys(value: &Value, keys: &[&str]) -> Option<String> {
    first_string_for_keys_inner(value, keys, 0)
}

fn first_string_for_keys_inner(value: &Value, keys: &[&str], depth: usize) -> Option<String> {
    if depth > 6 {
        return None;
    }

    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(value) = map.get(*key).and_then(Value::as_str) {
                    let value = value.trim();
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }

            map.values()
                .take(64)
                .find_map(|value| first_string_for_keys_inner(value, keys, depth + 1))
        }
        Value::Array(values) => values
            .iter()
            .take(64)
            .find_map(|value| first_string_for_keys_inner(value, keys, depth + 1)),
        _ => None,
    }
}

fn first_url(value: &Value) -> Option<String> {
    first_url_inner(value, 0)
}

fn first_url_inner(value: &Value, depth: usize) -> Option<String> {
    if depth > 6 {
        return None;
    }

    match value {
        Value::String(value) => {
            let value = value.trim();
            if value.starts_with("https://") || value.starts_with("http://") {
                Some(value.to_string())
            } else {
                None
            }
        }
        Value::Object(map) => map
            .values()
            .take(64)
            .find_map(|value| first_url_inner(value, depth + 1)),
        Value::Array(values) => values
            .iter()
            .take(64)
            .find_map(|value| first_url_inner(value, depth + 1)),
        _ => None,
    }
}

fn count_named_arrays(value: &Value, keys: &[&str]) -> usize {
    count_named_arrays_inner(value, keys, 0)
}

fn count_named_arrays_inner(value: &Value, keys: &[&str], depth: usize) -> usize {
    if depth > 6 {
        return 0;
    }

    match value {
        Value::Object(map) => {
            let direct = keys
                .iter()
                .filter_map(|key| map.get(*key).and_then(Value::as_array))
                .map(Vec::len)
                .sum::<usize>();
            direct
                + map
                    .values()
                    .take(64)
                    .map(|value| count_named_arrays_inner(value, keys, depth + 1))
                    .sum::<usize>()
        }
        Value::Array(values) => values
            .iter()
            .take(64)
            .map(|value| count_named_arrays_inner(value, keys, depth + 1))
            .sum(),
        _ => 0,
    }
}
