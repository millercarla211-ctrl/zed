use serde_json::Value;

pub(super) fn u64_field(value: &Value, field: &str) -> Option<u64> {
    value.get(field).and_then(Value::as_u64)
}

pub(super) fn optional_string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(render_safe_string)
}

fn render_safe_string(value: &str) -> String {
    let mut bounded = value.chars().take(180).collect::<String>();
    if value.chars().count() > 180 {
        bounded.push_str("...");
    }
    bounded
}
