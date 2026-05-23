use serde_json::Value;

pub(super) fn string_at(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

pub(super) fn bool_at(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub(super) fn usize_at(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or_default()
}

pub(super) fn array_len_at(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}

pub(super) fn string_array_at(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn compact_string_at(value: &Value, key: &str) -> Option<String> {
    string_at(value, key)
        .map(|value| compact_text(value, 240))
        .filter(|value| !value.is_empty())
}

pub(super) fn compact_string_array_at(value: &Value, key: &str, limit: usize) -> Vec<String> {
    string_array_at(value, key)
        .into_iter()
        .take(limit)
        .map(|value| compact_text(value, 240))
        .filter(|value| !value.is_empty())
        .collect()
}

fn compact_text(value: String, max_chars: usize) -> String {
    let value = value.trim();
    let mut chars = value.chars();
    let compact = chars.by_ref().take(max_chars).collect::<String>();

    if chars.next().is_some() {
        format!("{compact}...")
    } else {
        compact
    }
}
