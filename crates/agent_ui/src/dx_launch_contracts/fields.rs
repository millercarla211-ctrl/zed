use serde_json::Value;

pub(super) fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

pub(super) fn usize_field(value: &Value, field: &str) -> Option<usize> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| value.try_into().ok())
}

pub(super) fn bool_field(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_bool).unwrap_or(false)
}

pub(super) fn array_len(value: &Value, field: &str) -> usize {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}

pub(super) fn pointer_string<'a>(value: Option<&'a Value>, pointer: &str) -> Option<&'a str> {
    value
        .and_then(|value| value.pointer(pointer))
        .and_then(Value::as_str)
}

pub(super) fn pointer_string_array(value: Option<&Value>, pointer: &str) -> Vec<String> {
    value
        .and_then(|value| value.pointer(pointer))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .take(8)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}
