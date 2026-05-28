use serde_json::Value;

use super::super::{array_field, safe_string_field, value_at};

const MAX_RECEIPT_DISPLAY_CHARS: usize = 180;
const MAX_RECEIPT_STRING_VALUES: usize = 8;

pub(super) fn receipt_string_field(value: &Value, path: &[&str]) -> Option<String> {
    safe_string_field(value, path).and_then(bound_receipt_string)
}

pub(super) fn receipt_string_array_field(value: &Value, path: &[&str]) -> Vec<String> {
    array_field(value, path)
        .into_iter()
        .flatten()
        .filter_map(receipt_string_value)
        .take(MAX_RECEIPT_STRING_VALUES)
        .collect()
}

pub(super) fn receipt_string_values_field(value: &Value, path: &[&str]) -> Vec<String> {
    value_at(value, path)
        .and_then(|value| value.as_object())
        .into_iter()
        .flat_map(|values| values.values())
        .filter_map(receipt_string_value)
        .take(MAX_RECEIPT_STRING_VALUES)
        .collect()
}

fn receipt_string_value(value: &Value) -> Option<String> {
    safe_string_field(value, &[]).and_then(bound_receipt_string)
}

fn bound_receipt_string(value: String) -> Option<String> {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let compact = compact
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>();

    if compact.is_empty() {
        return None;
    }
    if compact.chars().count() <= MAX_RECEIPT_DISPLAY_CHARS {
        return Some(compact);
    }

    let mut bounded = compact
        .chars()
        .take(MAX_RECEIPT_DISPLAY_CHARS.saturating_sub(3))
        .collect::<String>();
    bounded.push_str("...");
    Some(bounded)
}
