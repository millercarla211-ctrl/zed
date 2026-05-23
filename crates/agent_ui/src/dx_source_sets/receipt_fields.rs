use serde_json::Value;

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

pub(super) fn string_at(value: &Value, path: &[&str]) -> Option<String> {
    value_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

pub(super) fn bool_at(value: &Value, path: &[&str]) -> Option<bool> {
    value_at(value, path).and_then(Value::as_bool)
}

pub(super) fn array_strings_at(value: &Value, path: &[&str]) -> Vec<String> {
    value_at(value, path)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .take(4)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn usize_at(value: &Value, path: &[&str]) -> Option<usize> {
    value_at(value, path)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

pub(super) fn u64_at(value: &Value, path: &[&str]) -> Option<u64> {
    value_at(value, path).and_then(Value::as_u64)
}
