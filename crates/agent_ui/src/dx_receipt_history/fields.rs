use serde_json::Value;

pub(super) fn string_field(value: &Value, path: &[&str]) -> Option<String> {
    value_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
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
