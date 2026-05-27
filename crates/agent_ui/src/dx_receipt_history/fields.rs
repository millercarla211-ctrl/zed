use serde_json::Value;

pub(super) fn string_field(value: &Value, path: &[&str]) -> Option<String> {
    value_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

pub(super) fn safe_string_field(value: &Value, path: &[&str]) -> Option<String> {
    string_field(value, path).and_then(|value| {
        let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
        if value.is_empty() {
            None
        } else if is_secret_like_scalar(&value) {
            Some("<redacted>".to_string())
        } else {
            Some(value)
        }
    })
}

pub(super) fn bool_field(value: &Value, path: &[&str]) -> Option<bool> {
    value_at(value, path).and_then(Value::as_bool)
}

pub(super) fn usize_field(value: &Value, path: &[&str]) -> Option<usize> {
    value_at(value, path)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

fn is_secret_like_scalar(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let normalized = lower.replace('-', "_");
    DX_RECEIPT_SECRET_MARKERS
        .iter()
        .any(|marker| lower.contains(marker) || normalized.contains(marker))
}

const DX_RECEIPT_SECRET_MARKERS: &[&str] = &[
    "sk-",
    "secret",
    "token",
    "password",
    "passwd",
    "cookie",
    "authorization",
    "bearer ",
    "api_key",
    "apikey",
    "provider_key",
    "access_key",
    "access_token",
    "refresh_token",
    "private-token",
    "xoxb-",
    "xoxp-",
];
