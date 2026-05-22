use serde_json::Value;

#[derive(Clone)]
pub(crate) struct DxDeployLaunchBucket {
    pub id: Option<String>,
    pub label: String,
    pub status: Option<String>,
    pub score: Option<usize>,
    pub max_score: Option<usize>,
    pub estimated: Option<bool>,
    pub summary: Option<String>,
}

pub(crate) fn launch_buckets(receipt: &Value) -> Vec<DxDeployLaunchBucket> {
    receipt
        .get("bucket_scores")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(5)
        .filter_map(|row| {
            let label = string_field(row, "label").or_else(|| string_field(row, "id"))?;

            Some(DxDeployLaunchBucket {
                id: string_field(row, "id"),
                label,
                status: string_field(row, "status"),
                score: usize_field(row, "score"),
                max_score: usize_field(row, "max_score"),
                estimated: bool_field(row, "estimated"),
                summary: string_field(row, "summary"),
            })
        })
        .collect()
}

pub(crate) fn launch_bucket_summary_rows(buckets: &[DxDeployLaunchBucket]) -> Vec<String> {
    buckets
        .iter()
        .map(|bucket| {
            let mut parts = vec![bucket.label.clone()];

            if let (Some(score), Some(max_score)) = (bucket.score, bucket.max_score) {
                parts.push(format!("{score}/{max_score}"));
            }
            if bucket.estimated == Some(true) {
                parts.push("estimated".to_string());
            }
            if let Some(status) = bucket.status.as_ref() {
                parts.push(status.clone());
            }

            parts.join(" - ")
        })
        .collect()
}

pub(crate) fn launch_buckets_prompt(buckets: &[DxDeployLaunchBucket]) -> String {
    if buckets.is_empty() {
        return "none".to_string();
    }

    buckets
        .iter()
        .map(|bucket| {
            let id = bucket.id.as_deref().unwrap_or("unknown");
            let score = match (bucket.score, bucket.max_score) {
                (Some(score), Some(max_score)) => format!("{score}/{max_score}"),
                _ => "unknown".to_string(),
            };
            let status = bucket.status.as_deref().unwrap_or("unknown");
            let estimated = bucket
                .estimated
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let summary = bucket.summary.as_deref().unwrap_or("none");

            format!(
                "{} id={} score={} status={} estimated={} summary={}",
                bucket.label, id, score, status, estimated, summary
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
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

fn bool_field(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}
